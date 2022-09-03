use std::path::PathBuf;

use clap::{Arg, Command};

pub fn build_cli() -> Command<'static> {
    Command::new("Riptide")
        .name("riptide")
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("Fast and easy file sharing over the internet, through a simple cli.")
        .arg(
            Arg::new("time")
                .help("Set how many hours to share the file for")
                .short('t')
                .long("time")
                .default_value("24")
                .takes_value(true)
                .value_name("HOURS")
                .forbid_empty_values(true)
                .value_parser(clap::value_parser!(i64).range(1..8760)),
        )
        .arg(
            Arg::new("remove")
                .help("Remove the file share indicated by this id by index or id")
                .short('r')
                .long("remove")
                .takes_value(true)
                .value_name("ID")
                .allow_hyphen_values(true)
                .allow_invalid_utf8(false)
                .value_parser(clap::value_parser!(u64))
                .validator(|data| -> Result<(), &'static str> {
                    // check that id is a valid u32
                    let _: u32 = match data.parse() {
                        Ok(data) => data,
                        Err(_) => return Err("ID must be a valid u32"),
                    };

                    // POLL the database and check if this id exists
                    // if it does, return Ok(())
                    // if it doesn't, return Err("Invalid id")
                    //TODO

                    Ok(())
                }),
        )
        .arg(
            Arg::new("list")
                .help("List all currently shared files")
                .short('l')
                .long("list")
                .takes_value(false),
        )
        .arg(
            Arg::new("file")
                .help("Name of the file to share")
                .required(false)
                .index(1)
                .allow_invalid_utf8(false)
                .value_parser(clap::value_parser!(PathBuf)),
        )
}

#[test]
fn verify_cmd() {
    build_cli().debug_assert();
}
