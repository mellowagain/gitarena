use gitarena_issues::author::Author;
use gitarena_issues::id::BugId;
use serde_json::Value;
use std::process::Command;
use tempfile::{TempDir, tempdir};
use uuid::Uuid;

pub struct GitBug {
    dir: TempDir,
    pub repo: gix::Repository,
}

impl GitBug {
    pub fn new() -> Self {
        let dir = tempdir().expect("to be able to create a temp dir");
        let repo = gix::init(dir.path()).expect("to init a git repository in the temp dir");

        GitBug { dir, repo }
    }

    pub fn run(&self, args: &[&str]) -> String {
        let out = Command::new("git-bug")
            .args(args)
            .current_dir(self.dir.path())
            .output()
            .expect("git-bug to work");
        assert!(out.status.success(), "git-bug {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));

        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn git(&self, args: &[&str]) {
        let status = Command::new("git").args(args).current_dir(self.dir.path()).status().expect("git to work");
        assert!(status.success(), "git {args:?} failed");
    }

    pub fn add_origin(&self, upstream: &GitBug) {
        self.git(&["remote", "add", "origin", &upstream.dir.path().to_string_lossy()]);
    }

    pub fn show(&self, bug_id: &BugId) -> Value {
        let output = self.run(&["bug", "show", "--format", "json", &bug_id.to_string()]);
        serde_json::from_str(&output).unwrap_or_else(|err| panic!("git-bug show output is not valid JSON: {err}\nOutput:\n{output}"))
    }
}

pub fn make_author() -> Author {
    Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000)
}
