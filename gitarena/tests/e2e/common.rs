pub mod db;

use std::process::{Child, Command};
use rand::RngExt;
use serde_json::json;
use tempfile::{tempdir, TempDir};
use test_context::TestContext;

#[allow(unused)]
pub struct Harness {
    temp_dir: TempDir,
    port: u16,
    server: Child
}

impl TestContext for Harness {
    fn setup() -> Self {
        let port: u16 = rand::rng().random();
        let dir = tempdir().expect("failed to create temp dir");

        let server = Command::new(env!("CARGO_BIN_EXE_gitarena"))
            .env("BIND_ADDRESS", format!("127.0.0.1:{port}"))
            .spawn()
            .expect("failed to run gitarena");

        let register_request = json!({
            "username": "test_user",
            "email": "",
            "password": ""
        });
        
        Harness {
            temp_dir: dir,
            port,
            server,
        }
    }

    fn teardown(mut self) {
        self.server.kill().unwrap();
        self.temp_dir.close().unwrap();
    }
}

pub fn git(ctx: &Harness, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(ctx.temp_dir.path())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    if status.success() {
        return;
    }

    panic!("git exited with status code {}", status.code().unwrap());
}
