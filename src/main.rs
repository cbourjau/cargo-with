use clap::{App, AppSettings, Arg, SubCommand};
use failure::{err_msg, Error};
use log::debug;

mod cargo;
mod runner;

use crate::runner::runner;

const COMMAND_NAME: &str = "with";
const COMMAND_DESCRIPTION: &str =
    "A third-party cargo extension to run the build artifacts through tools like `gdb`";

// Make a separate runner to print errors using Display instead of Debug
fn main() -> Result<(), Error> {
    env_logger::init();

    let app = create_app();
    let matches = app.get_matches();

    debug!("CLI matches: {:#?}", matches);

    let (cargo_cmd_iter, cmd_iter) = process_matches(&matches)?;
    runner(cargo_cmd_iter, cmd_iter)
}

fn process_matches<'a>(
    matches: &'a clap::ArgMatches<'_>,
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
        .ok_or_else(|| err_msg("Failed to parse the correct subcommand"))?;

    let cargo_cmd_iter = matches
        .values_of("cargo-cmd")
        .ok_or_else(|| err_msg("Failed to parse the cargo command producing the artifact"))?;

    // The string describing how to envoke the child process
    let cmd_iter = matches
        .value_of("with-cmd")
        .ok_or_else(|| err_msg("Failed to parse the command to run with the artifact"))?
        .trim()
        .split(' ');

    Ok((cargo_cmd_iter, cmd_iter))
}

fn create_app<'a, 'b>() -> App<'a, 'b> {
    let with_usage = concat!(
        "<with-cmd> 'Command executed with the cargo-created binary. ",
        "The placeholders {bin} {args} denote the path to the binary and additional arguments.",
        "If omitted `{bin} {args}` is appended to `with-cmd`'"
    );
    let cargo_usage = "<cargo-cmd> 'The Cargo subcommand starting with `test`, `run`, or `bench`'";
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
                .arg(Arg::from_usage(&with_usage))
                .arg(clap::Arg::from_usage(cargo_usage).raw(true))
                .after_help(r#"
EXAMPLES:
   cargo with echo -- run
   cargo with "gdb --args" -- run
   cargo with "echo {args} {bin}" -- test -- myargs
"#
                )
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
