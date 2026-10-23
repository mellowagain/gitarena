use gitarena_issues::author::Author;
use serde_json::Value;
use std::process::Command;
use tempfile::{TempDir, tempdir};
use uuid::Uuid;

/// Temporary repository to run the `git-bug` CLI in
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

    /// Run `git-bug` with `args` and return its stdout, panicking if it fails
    pub fn run(&self, args: &[&str]) -> String {
        let out = Command::new("git-bug")
            .args(args)
            .current_dir(self.dir.path())
            .output()
            .expect("git-bug to work");
        assert!(out.status.success(), "git-bug {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));

        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn show(&self, bug_id: &str) -> Value {
        let output = self.run(&["bug", "show", "--format", "json", bug_id]);
        serde_json::from_str(&output).unwrap_or_else(|err| panic!("git-bug show output is not valid JSON: {err}\nOutput:\n{output}"))
    }
}

pub fn make_author() -> Author {
    Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000)
}
