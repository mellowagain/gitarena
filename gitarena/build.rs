use vergen_gix::{Emitter, Gix};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gix = Gix::builder().sha(false).build();

    Emitter::default().add_instructions(&gix)?.emit()?;

    println!("cargo:rerun-if-changed=../migrations");
    Ok(())
}
