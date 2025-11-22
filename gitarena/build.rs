use vergen::{Config, vergen};

fn main() {
    vergen(Config::default()).unwrap();
    println!("cargo:rerun-if-changed=migrations");
}
