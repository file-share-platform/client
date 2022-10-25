use clap::ArgEnum;
use clap_complete::{generate_to, Shell};
use std::env;

include!("src/cli.rs");

fn main() -> std::io::Result<()> {
    // generate autocomplete scripts for various shells
    let outdir = match env::var_os("OUT_DIR") {
        Some(outdir) => outdir,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "OUT_DIR not set",
            ))
        }
    };

    let mut cmd = build_cli();

    //export shell completions
    let outdir = std::path::Path::new(&outdir).join("../../../shell_completions");
    std::fs::create_dir_all(&outdir)?;

    for shell in Shell::value_variants() {
        generate_to(*shell, &mut cmd, "riptide", &outdir)?;
    }

    println!(
        "cargo:warning=completion files have been generated in: {:?}",
        outdir
    );
    println!("cargo:rerun-if-changed=src/cli.rs");

    Ok(())
}
