#[macro_use]
extern crate log;
extern crate cargo_with;
extern crate clap;
extern crate env_logger;
extern crate failure;

use clap::{App, AppSettings, Arg, SubCommand};
use failure::{err_msg, Error};

const COMMAND_NAME: &str = "with";
const COMMAND_DESCRIPTION: &str =
    "A third-party cargo extension to run the build artifacts through tools like `gdb`";

fn runner() -> Result<(), Error> {
    env_logger::init();

    let app = create_app();
    let matches = app.get_matches();

    debug!("CLI matches: {:#?}", matches);

    let (cargo_cmd_iter, cmd_iter) = process_matches(&matches)?;

    cargo_with::run(cargo_cmd_iter, cmd_iter)
}

// Make a separate runner to print errors using Display instead of Debug
fn main() {
    if let Err(e) = runner() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn process_matches<'a>(
    matches: &'a clap::ArgMatches,
) -> Result<
    (
        impl Iterator<Item = &'a str> + Clone,
        impl Iterator<Item = &'a str> + Clone,
    ),
    Error,
> {
    // The original cargo command
    let matches = matches
        .subcommand_matches(COMMAND_NAME)
        .ok_or(err_msg("Failed to parse the correct subcommand"))?;

    let cargo_cmd_iter = matches.values_of("cargo-cmd").ok_or(err_msg(
        "Failed to parse the cargo command producing the artifact",
    ))?;

    // The string describing how to envoke the child process
    let cmd_iter = matches
        .value_of("with-cmd")
        .ok_or(err_msg(
            "Failed to parse the command to run with the artifact",
        ))?
        .trim()
        .split(' ');

    Ok((cargo_cmd_iter, cmd_iter))
}

fn create_app<'a, 'b>() -> App<'a, 'b> {
    let usage = concat!(
        "<with-cmd> 'Command executed with the cargo-created binary. ",
        "Use {bin} to denote the binary. ",
        "If omitted the {bin} is added as the last argument'"
    );
    App::new(COMMAND_NAME)
        .about(COMMAND_DESCRIPTION)
        // We have to lie about our binary name since this will be a third party
        // subcommand for cargo, this trick learned from cargo-outdated
        .bin_name("cargo")
        // We use a subcommand because parsed after `cargo` is sent to the third party plugin
        // which will be interpreted as a subcommand/positional arg by clap
        .subcommand(
            SubCommand::with_name(COMMAND_NAME)
                .about(COMMAND_DESCRIPTION)
                .arg(Arg::from_usage(&usage))
                .arg(
                    clap::Arg::from_usage("<cargo-cmd> 'The cargo subcommand `test` or `run`'")
                        .raw(true),
                ),
        )
        .settings(&[AppSettings::SubcommandRequired])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args() {
        let app = create_app();
        let _matches = app.get_matches_from(vec![
            "cargo",
            "with",
            "gdb --args {bin}",
            "--",
            "test",
            "--release",
            "--",
            "test2",
        ]);
    }
}
