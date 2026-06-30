use vergen_gitcl::{Build, Emitter, Gitcl, Rustc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = Build::all_build();
    let gitcl = Gitcl::all_git();
    let rustc = Rustc::all_rustc();

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gitcl)?
        .add_instructions(&rustc)?
        .emit()?;

    println!("cargo:rerun-if-changed=../migrations");

    Ok(())
}
