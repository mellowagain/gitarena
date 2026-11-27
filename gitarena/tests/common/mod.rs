// every test binary only uses part of the harness
#![allow(dead_code)]

pub mod scenarios;

use anyhow::{Context, Result, anyhow, bail, ensure};
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::env;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;
use tempfile::{TempDir, tempdir};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep, timeout};
use url::Url;
use uuid::Uuid;

const BOOT_ATTEMPTS: usize = 3;
const BOOT_TIMEOUT: Duration = Duration::from_secs(60);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const AGENT_TIMEOUT: Duration = Duration::from_secs(10);

pub const AUTHOR_NAME: &str = "Test Author";
pub const AUTHOR_EMAIL: &str = "author@example.com";

#[derive(Debug, Clone, Copy)]
pub enum Transport {
    Http,
    Ssh,
}

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    V1,
    V2,
}

impl Protocol {
    fn version(self) -> &'static str {
        match self {
            Protocol::V1 => "1",
            Protocol::V2 => "2",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    fn as_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
        }
    }
}

pub struct TestServer {
    child: Child,
    pub http_port: u16,
    pub ssh_port: u16,
    home: TempDir,
    repos_dir: TempDir,
}

impl TestServer {
    pub async fn start(pool: &PgPool) -> Result<TestServer> {
        let database_url = test_database_url(pool)?;
        let repos_dir = tempdir()?;
        let home = tempdir()?;

        let mut last_error = None;

        for _ in 0..BOOT_ATTEMPTS {
            let http_port = free_port()?;
            let ssh_port = free_port()?;

            seed_settings(pool, http_port, ssh_port, repos_dir.path()).await?;

            let mut child = Command::new(env!("CARGO_BIN_EXE_gitarena"))
                .env_clear()
                .env("PATH", env::var_os("PATH").unwrap_or_default())
                .env("HOME", home.path())
                .env("DATABASE_URL", &database_url)
                .env("BIND_ADDRESS", format!("127.0.0.1:{http_port}"))
                .env("RUST_LOG", "gitarena=debug,gitarena::git::io::reader=error")
                .env("MAX_POOL_CONNECTIONS", "3")
                .env("MAX_QUEUE_POOL_CONNECTIONS", "2")
                .env("DISABLE_UPDATE_CHECK", "1")
                .env("DISABLE_LICENSE_DETECTION", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_SYSTEM", "/dev/null")
                .stdin(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .context("failed to spawn gitarena")?;

            match wait_until_ready(&mut child, http_port, ssh_port).await {
                Ok(()) => {
                    return Ok(TestServer {
                        child,
                        http_port,
                        ssh_port,
                        home,
                        repos_dir,
                    });
                }
                Err(err) => {
                    let _ = child.kill().await;
                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("gitarena did not start")))
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.http_port)
    }

    pub fn remote(&self, transport: Transport, user: Option<&TestUser>, repo: &str) -> String {
        match (transport, user) {
            (Transport::Http, Some(user)) => format!("http://{}:{}@127.0.0.1:{}/{repo}.git", user.username, user.password, self.http_port),
            (Transport::Http, None) => format!("http://127.0.0.1:{}/{repo}.git", self.http_port),
            (Transport::Ssh, _) => format!("ssh://git@127.0.0.1:{}/{repo}.git", self.ssh_port),
        }
    }
}

fn test_database_url(pool: &PgPool) -> Result<String> {
    let mut url = Url::parse(&env::var("DATABASE_URL").context("DATABASE_URL needs to be set to run the git tests")?)?;
    let options = pool.connect_options();
    let database = options.get_database().context("sqlx test pool has no database name")?;

    url.set_path(database);
    Ok(url.into())
}

fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

async fn seed_settings(pool: &PgPool, http_port: u16, ssh_port: u16, repos_dir: &Path) -> Result<()> {
    // webauthn uses the domain as its relying party id, which has to be a hostname and not an ip
    let backend = format!("http://localhost:{http_port}");
    let repos_dir = repos_dir.to_str().context("temp dir is not valid unicode")?;
    let ssh_port = ssh_port.to_string();

    let settings = [
        ("domain.app", backend.as_str()),
        ("domain.backend", backend.as_str()),
        ("domain.user_email", "localhost"),
        ("repositories.base_dir", repos_dir),
        ("allow_registrations", "true"),
        ("ssh.enabled", "true"),
        ("ssh.port", ssh_port.as_str()),
    ];

    for (key, value) in settings {
        let result = sqlx::query("update settings set value = $2 where key = $1")
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;

        ensure!(result.rows_affected() == 1, "setting {key} does not exist after migrations");
    }

    Ok(())
}

async fn wait_until_ready(child: &mut Child, http_port: u16, ssh_port: u16) -> Result<()> {
    let client = Client::builder().timeout(HEALTH_TIMEOUT).build()?;
    let deadline = Instant::now() + BOOT_TIMEOUT;

    loop {
        if let Some(status) = child.try_wait()? {
            bail!("gitarena exited during startup with {status}");
        }

        let healthy = client
            .get(format!("http://127.0.0.1:{http_port}/healthz"))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success());

        if healthy && TcpStream::connect(("127.0.0.1", ssh_port)).await.is_ok() {
            return Ok(());
        }

        ensure!(Instant::now() < deadline, "gitarena did not become ready within {BOOT_TIMEOUT:?}");
        sleep(Duration::from_millis(100)).await;
    }
}

pub struct Fixture {
    pub server: TestServer,
    pub owner: TestUser,
    pub repo: String,
    pub transport: Transport,
    pub dir: TempDir,
}

impl Fixture {
    pub async fn new(pool: &PgPool, transport: Transport, visibility: Visibility) -> Result<Fixture> {
        let server = TestServer::start(pool).await?;
        let owner = TestUser::with_ssh_key(&server, "alice").await?;
        let repo = owner.create_repo("repo", visibility).await?;

        Ok(Fixture {
            server,
            owner,
            repo,
            transport,
            dir: tempdir()?,
        })
    }

    pub async fn user(&self, username: &str) -> Result<TestUser> {
        TestUser::with_ssh_key(&self.server, username).await
    }

    pub fn remote(&self, user: Option<&TestUser>) -> String {
        self.server.remote(self.transport, user, &self.repo)
    }

    pub fn owner_remote(&self) -> String {
        self.remote(Some(&self.owner))
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn checkout(&self, user: &TestUser, name: &str) -> Git {
        user.git(&self.path(name))
    }

    pub async fn push_seed(&self) -> Result<(Git, String)> {
        let local = self.checkout(&self.owner, "seed");
        let head = local.seed().await?;

        local.ok(&["remote", "add", "origin", &self.owner_remote()]).await?;
        local.ok(&["push", "--quiet", "--set-upstream", "origin", "main"]).await?;

        Ok((local, head))
    }

    pub async fn refs(&self) -> Result<BTreeMap<String, String>> {
        self.owner.git(self.dir.path()).ls_remote(&self.owner_remote()).await
    }

    pub async fn git_bug(&self, user: Option<&TestUser>, name: &str) -> Result<GitBug> {
        self.git_bug_over(self.transport, user, name).await
    }

    pub async fn git_bug_over(&self, transport: Transport, user: Option<&TestUser>, name: &str) -> Result<GitBug> {
        let dir = self.path(name);
        let home = self.path(&format!("{name}-home"));
        fs::create_dir_all(&home).await?;

        let ssh = match (transport, user.and_then(|user| user.ssh_key.as_deref())) {
            (Transport::Ssh, Some(key)) => Some(SshAuth::start(&home, key, self.server.ssh_port).await?),
            _ => None,
        };

        let git_bug = GitBug {
            git: user.map_or_else(|| Git::new(&dir), |user| user.git(&dir)),
            home,
            ssh,
        };

        git_bug.git.init().await?;
        git_bug.set_remote(&self.server.remote(transport, user, &self.repo)).await?;

        Ok(git_bug)
    }

    pub async fn forge_mail(&self, pool: &PgPool, user: &TestUser) -> Result<String> {
        let (id,): (Uuid,) = sqlx::query_as("select id from users where username = $1")
            .bind(&user.username)
            .fetch_one(pool)
            .await?;

        Ok(format!("{}@localhost", bs58::encode(id.as_bytes()).into_string()))
    }
}

pub struct TestUser {
    pub username: String,
    pub password: String,
    pub email: String,
    pub client: Client,
    pub ssh_key: Option<PathBuf>,
    base_url: String,
    key_dir: TempDir,
}

impl TestUser {
    pub async fn register(server: &TestServer, username: &str) -> Result<TestUser> {
        let user = TestUser {
            username: username.to_owned(),
            password: format!("password-{username}"),
            email: format!("{username}@example.com"),
            client: Client::builder().cookie_store(true).timeout(COMMAND_TIMEOUT).build()?,
            ssh_key: None,
            base_url: server.base_url(),
            key_dir: tempdir()?,
        };

        let response = user
            .client
            .post(user.url("/api/user"))
            .json(&json!({ "username": user.username, "email": user.email, "password": user.password }))
            .send()
            .await?;

        expect_status(response, StatusCode::OK).await?;
        Ok(user)
    }

    pub async fn with_ssh_key(server: &TestServer, username: &str) -> Result<TestUser> {
        let mut user = TestUser::register(server, username).await?;
        user.add_ssh_key().await?;

        Ok(user)
    }

    pub async fn add_ssh_key(&mut self) -> Result<()> {
        let (private_key, public_key) = generate_ssh_key(self.key_dir.path(), &self.username).await?;

        let response = self
            .client
            .put(self.url("/api/ssh-key"))
            .json(&json!({ "title": format!("{} test key", self.username), "key": public_key }))
            .send()
            .await?;

        expect_status(response, StatusCode::CREATED).await?;

        self.ssh_key = Some(private_key);
        Ok(())
    }

    pub async fn create_repo(&self, name: &str, visibility: Visibility) -> Result<String> {
        let response = self
            .client
            .post(self.url("/api/repo"))
            .json(&json!({
                "namespace": self.username,
                "name": name,
                "description": "",
                "visibility": visibility.as_str(),
                "defaultBranch": "main",
            }))
            .send()
            .await?;

        expect_status(response, StatusCode::OK).await?;
        Ok(format!("{}/{name}", self.username))
    }

    pub fn git(&self, dir: &Path) -> Git {
        let git = Git::new(dir);

        match &self.ssh_key {
            Some(key) => git.with_ssh_key(key),
            None => git,
        }
    }

    pub async fn create_issue(&self, repo: &str, title: &str, body: &str) -> Result<Issue> {
        let response = self
            .client
            .post(self.url(&format!("/api/repos/{repo}/issues")))
            .json(&json!({ "title": title, "body": body }))
            .send()
            .await?;

        expect_json(response, StatusCode::CREATED).await
    }

    pub async fn update_issue(&self, repo: &str, index: i32, body: Value) -> Result<()> {
        let response = self
            .client
            .patch(self.url(&format!("/api/repos/{repo}/issues/{index}")))
            .json(&body)
            .send()
            .await?;

        expect_status(response, StatusCode::OK).await?;
        Ok(())
    }

    pub async fn delete_issue(&self, repo: &str, index: i32) -> Result<()> {
        let response = self.client.delete(self.url(&format!("/api/repos/{repo}/issues/{index}"))).send().await?;

        expect_status(response, StatusCode::NO_CONTENT).await?;
        Ok(())
    }

    pub async fn add_comment(&self, repo: &str, index: i32, body: &str) -> Result<String> {
        let response = self
            .client
            .post(self.url(&format!("/api/repos/{repo}/issues/{index}/comments")))
            .json(&json!({ "body": body }))
            .send()
            .await?;

        let comment: Value = expect_json(response, StatusCode::CREATED).await?;
        Ok(comment["id"].as_str().context("comment response has no id")?.to_owned())
    }

    pub async fn edit_comment(&self, repo: &str, index: i32, comment_id: &str, body: &str) -> Result<()> {
        let response = self
            .client
            .patch(self.url(&format!("/api/repos/{repo}/issues/{index}/comments/{comment_id}")))
            .json(&json!({ "body": body }))
            .send()
            .await?;

        expect_status(response, StatusCode::OK).await?;
        Ok(())
    }

    pub async fn create_label(&self, repo: &str, name: &str) -> Result<()> {
        let response = self
            .client
            .post(self.url(&format!("/api/repos/{repo}/labels")))
            .json(&json!({ "name": name, "color": "#ff0000" }))
            .send()
            .await?;

        expect_status(response, StatusCode::OK).await?;
        Ok(())
    }

    pub async fn issue(&self, repo: &str, index: i32) -> Result<Value> {
        let response = self.client.get(self.url(&format!("/api/repos/{repo}/issues/{index}"))).send().await?;

        expect_json(response, StatusCode::OK).await
    }

    pub async fn timeline(&self, repo: &str, index: i32) -> Result<Vec<Value>> {
        let response = self.client.get(self.url(&format!("/api/repos/{repo}/issues/{index}/timeline"))).send().await?;

        expect_json(response, StatusCode::OK).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub index: i32,
    pub git_bug_id: String,
}

async fn expect_status(response: Response, expected: StatusCode) -> Result<String> {
    let status = response.status();
    let url = response.url().clone();
    let body = response.text().await?;

    ensure!(status == expected, "{url} returned {status} instead of {expected}: {body}");
    Ok(body)
}

async fn expect_json<T: DeserializeOwned>(response: Response, expected: StatusCode) -> Result<T> {
    let body = expect_status(response, expected).await?;
    serde_json::from_str(&body).with_context(|| format!("response is not the expected json: {body}"))
}

pub async fn generate_ssh_key(dir: &Path, comment: &str) -> Result<(PathBuf, String)> {
    let path = dir.join("id_ed25519");

    let output = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", comment, "-f"])
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .await?;

    ensure!(output.status.success(), "ssh-keygen failed: {}", String::from_utf8_lossy(&output.stderr));

    let public_key = fs::read_to_string(path.with_extension("pub")).await?;
    Ok((path, public_key.trim().to_owned()))
}

async fn run_with_timeout(command: &mut Command, input: Option<&[u8]>) -> Result<Output> {
    command.kill_on_drop(true).stdout(Stdio::piped()).stderr(Stdio::piped());
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });

    let mut child = command.spawn()?;

    let run = async {
        if let Some(input) = input {
            let mut stdin = child.stdin.take().context("child has no stdin")?;
            stdin.write_all(input).await?;
        }

        Ok(child.wait_with_output().await?)
    };

    timeout(COMMAND_TIMEOUT, run)
        .await
        .with_context(|| format!("{command:?} did not finish within {COMMAND_TIMEOUT:?}"))?
}

pub struct GitOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl From<Output> for GitOutput {
    fn from(output: Output) -> Self {
        GitOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

#[derive(Clone)]
pub struct Git {
    dir: PathBuf,
    ssh_key: Option<PathBuf>,
    protocol: Option<Protocol>,
}

impl Git {
    pub fn new(dir: &Path) -> Git {
        Git {
            dir: dir.to_owned(),
            ssh_key: None,
            protocol: None,
        }
    }

    #[must_use]
    pub fn with_ssh_key(mut self, key: &Path) -> Git {
        self.ssh_key = Some(key.to_owned());
        self
    }

    #[must_use]
    pub fn with_protocol(mut self, protocol: Protocol) -> Git {
        self.protocol = Some(protocol);
        self
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn command(&self) -> Command {
        let ssh_command = match &self.ssh_key {
            Some(key) => format!("{} -i {}", SSH_COMMAND, key.display()),
            None => format!("{SSH_COMMAND} -o PubkeyAuthentication=no"),
        };

        let mut command = Command::new("git");

        command
            .current_dir(&self.dir)
            .env_remove("SSH_AUTH_SOCK")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_NAME", AUTHOR_NAME)
            .env("GIT_AUTHOR_EMAIL", AUTHOR_EMAIL)
            .env("GIT_COMMITTER_NAME", AUTHOR_NAME)
            .env("GIT_COMMITTER_EMAIL", AUTHOR_EMAIL)
            .env("GIT_SSH_COMMAND", ssh_command)
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "init.defaultBranch")
            .env("GIT_CONFIG_VALUE_0", "main")
            .env("GIT_CONFIG_KEY_1", "protocol.version")
            .env("GIT_CONFIG_VALUE_1", self.protocol.unwrap_or(Protocol::V2).version());

        command
    }

    pub async fn run(&self, args: &[&str]) -> Result<GitOutput> {
        Ok(run_with_timeout(self.command().args(args), None).await?.into())
    }

    pub async fn run_with_input(&self, args: &[&str], input: &[u8]) -> Result<GitOutput> {
        Ok(run_with_timeout(self.command().args(args), Some(input)).await?.into())
    }

    pub async fn ok(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args).await?;
        ensure!(output.success, "git {args:?} failed:\n{}", output.stderr);

        Ok(output.stdout)
    }

    pub async fn fails(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args).await?;
        ensure!(!output.success, "git {args:?} unexpectedly succeeded:\n{}", output.stderr);

        Ok(output.stderr)
    }

    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.dir).await?;
        self.ok(&["init", "--quiet"]).await?;

        Ok(())
    }

    pub async fn clone_into(&self, remote: &str) -> Result<()> {
        self.clone_into_with(remote, &[]).await
    }

    pub async fn clone_into_with(&self, remote: &str, args: &[&str]) -> Result<()> {
        fs::create_dir_all(&self.dir).await?;
        self.ok(&[&["clone", "--quiet"], args, &[remote, "."]].concat()).await?;

        Ok(())
    }

    pub async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let path = self.dir.join(path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;
        Ok(())
    }

    pub async fn read_file(&self, path: &str) -> Result<String> {
        Ok(fs::read_to_string(self.dir.join(path)).await?)
    }

    pub async fn commit_all(&self, message: &str) -> Result<String> {
        self.ok(&["add", "--all"]).await?;
        self.ok(&["commit", "--quiet", "--message", message]).await?;

        self.rev_parse("HEAD").await
    }

    pub async fn commit_file(&self, path: &str, content: &str, message: &str) -> Result<String> {
        self.write_file(path, content.as_bytes()).await?;
        self.commit_all(message).await
    }

    pub async fn rev_parse(&self, rev: &str) -> Result<String> {
        Ok(self.ok(&["rev-parse", rev]).await?.trim().to_owned())
    }

    pub async fn rev_count(&self, rev: &str) -> Result<usize> {
        Ok(self.ok(&["rev-list", "--count", rev]).await?.trim().parse()?)
    }

    pub async fn ls_remote(&self, remote: &str) -> Result<BTreeMap<String, String>> {
        let stdout = self.ok(&["ls-remote", remote]).await?;

        stdout
            .lines()
            .map(|line| {
                let (oid, name) = line.split_once('\t').with_context(|| format!("malformed ls-remote line: {line}"))?;
                Ok((name.to_owned(), oid.to_owned()))
            })
            .collect()
    }

    pub async fn fsck(&self) -> Result<()> {
        self.ok(&["fsck", "--full", "--no-progress"]).await?;
        Ok(())
    }

    pub async fn seed(&self) -> Result<String> {
        self.init().await?;
        self.commit_file("README.md", "# test\n", "initial commit").await
    }

    pub async fn fast_import(&self, commits: usize, large_blobs: usize, blob_size: usize) -> Result<()> {
        self.init().await?;

        let output = self
            .run_with_input(&["fast-import", "--quiet"], &fast_import_stream(commits, large_blobs, blob_size))
            .await?;
        ensure!(output.success, "git fast-import failed:\n{}", output.stderr);

        Ok(())
    }
}

// git-bug talks to the remote through go-git, which ignores `GIT_SSH_COMMAND` and only authenticates via an agent
pub struct SshAuth {
    agent: Child,
    socket: PathBuf,
    known_hosts: PathBuf,
}

impl SshAuth {
    async fn start(dir: &Path, key: &Path, ssh_port: u16) -> Result<SshAuth> {
        let socket = dir.join("agent.sock");
        let known_hosts = dir.join("known_hosts");

        let agent = Command::new("ssh-agent")
            .arg("-D")
            .arg("-a")
            .arg(&socket)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("failed to spawn ssh-agent")?;

        let auth = SshAuth { agent, socket, known_hosts };

        let deadline = Instant::now() + AGENT_TIMEOUT;

        while !fs::try_exists(&auth.socket).await? {
            ensure!(Instant::now() < deadline, "ssh-agent did not create its socket within {AGENT_TIMEOUT:?}");
            sleep(Duration::from_millis(20)).await;
        }

        let output = run_with_timeout(Command::new("ssh-add").arg("-q").arg(key).env("SSH_AUTH_SOCK", &auth.socket), None).await?;
        ensure!(output.status.success(), "ssh-add failed: {}", String::from_utf8_lossy(&output.stderr));

        let output = run_with_timeout(Command::new("ssh-keyscan").args(["-p", &ssh_port.to_string(), "127.0.0.1"]), None).await?;
        ensure!(
            output.status.success() && !output.stdout.is_empty(),
            "ssh-keyscan failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        fs::write(&auth.known_hosts, &output.stdout).await?;
        Ok(auth)
    }
}

pub struct GitBug {
    pub git: Git,
    home: PathBuf,
    ssh: Option<SshAuth>,
}

impl GitBug {
    pub async fn set_remote(&self, remote: &str) -> Result<()> {
        if !self.git.run(&["remote", "set-url", "origin", remote]).await?.success {
            self.git.ok(&["remote", "add", "origin", remote]).await?;
        }

        Ok(())
    }

    fn command(&self) -> Command {
        let mut command = Command::new("git-bug");

        command
            .current_dir(self.git.dir())
            .env_clear()
            .env("PATH", env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0");

        if let Some(ssh) = &self.ssh {
            command.env("SSH_AUTH_SOCK", &ssh.socket).env("SSH_KNOWN_HOSTS", &ssh.known_hosts);
        }

        command
    }

    pub async fn run(&self, args: &[&str]) -> Result<GitOutput> {
        Ok(run_with_timeout(self.command().args(args), None).await?.into())
    }

    pub async fn ok(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args).await?;
        ensure!(output.success, "git-bug {args:?} failed:\n{}{}", output.stdout, output.stderr);

        Ok(output.stdout)
    }

    pub async fn fails(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args).await?;
        ensure!(!output.success, "git-bug {args:?} unexpectedly succeeded:\n{}{}", output.stdout, output.stderr);

        Ok(format!("{}{}", output.stdout, output.stderr))
    }

    pub async fn pull(&self) -> Result<()> {
        self.ok(&["pull", "origin"]).await?;
        Ok(())
    }

    pub async fn create_identity(&self, name: &str) -> Result<()> {
        self.ok(&["user", "new", "--non-interactive", "--name", name, "--email", &format!("{name}@example.com")])
            .await?;

        Ok(())
    }

    pub async fn create_bug(&self, title: &str) -> Result<()> {
        self.ok(&["bug", "new", "--non-interactive", "--title", title, "--message", "local"]).await?;
        Ok(())
    }

    pub async fn bugs(&self) -> Result<Vec<Value>> {
        let stdout = self.ok(&["bug", "--format", "json"]).await?;
        serde_json::from_str(&stdout).with_context(|| format!("git-bug bug output is not valid json: {stdout}"))
    }

    pub async fn show(&self, id: &str) -> Result<Value> {
        let stdout = self.ok(&["bug", "show", "--format", "json", id]).await?;
        serde_json::from_str(&stdout).with_context(|| format!("git-bug bug show output is not valid json: {stdout}"))
    }

    // `git bug user show` refuses to run until a local identity has been set, so the stored identity is read directly
    pub async fn identity(&self, id: &str) -> Result<Value> {
        let stdout = self.git.ok(&["cat-file", "blob", &format!("refs/identities/{id}:version")]).await?;
        serde_json::from_str(&stdout).with_context(|| format!("identity {id} is not valid json: {stdout}"))
    }

    pub async fn local_refs(&self, prefix: &str) -> Result<Vec<String>> {
        let stdout = self.git.ok(&["for-each-ref", "--format=%(refname)", prefix]).await?;
        Ok(stdout.lines().map(ToOwned::to_owned).collect())
    }
}

const SSH_COMMAND: &str = "ssh -F /dev/null -o IdentitiesOnly=yes -o IdentityAgent=none -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o BatchMode=yes -o LogLevel=ERROR";

fn fast_import_stream(commits: usize, large_blobs: usize, blob_size: usize) -> Vec<u8> {
    let mut stream = Vec::new();
    let mut state = 0x2545_f491_4f6c_dd1d_u64;

    let data = |stream: &mut Vec<u8>, content: &[u8]| {
        stream.extend_from_slice(format!("data {}\n", content.len()).as_bytes());
        stream.extend_from_slice(content);
        stream.push(b'\n');
    };

    for index in 0..commits {
        stream.extend_from_slice(b"commit refs/heads/main\n");
        stream.extend_from_slice(format!("committer {AUTHOR_NAME} <{AUTHOR_EMAIL}> {} +0000\n", 1_700_000_000 + index * 60).as_bytes());
        data(&mut stream, format!("commit {index}").as_bytes());

        let file = format!("M 100644 inline files/{}.txt\n", index % 50);
        stream.extend_from_slice(file.as_bytes());
        data(&mut stream, format!("revision {index}\n").repeat(index % 7 + 1).as_bytes());

        if index < large_blobs {
            let blob = (0..blob_size)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state.to_le_bytes()[0]
                })
                .collect::<Vec<u8>>();

            stream.extend_from_slice(format!("M 100644 inline blobs/{index}.bin\n").as_bytes());
            data(&mut stream, &blob);
        }

        stream.push(b'\n');
    }

    stream
}
