use config_struct::{Error, StructOptions, create_struct};

fn main() -> Result<(), Error> {
    create_struct(
        "config.toml",
        "src/config.rs",
        &StructOptions::serde_default()
    )
}
