#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate env_logger;
extern crate failure;
extern crate serde;
extern crate serde_json;

use std::process::Command;
use std::str;

use clap::{App, AppSettings, Arg, SubCommand};
use failure::{err_msg, Error};

// const CARGO_TOML: &'static str = "Cargo.toml";
const COMMAND_NAME: &str = "with";
const COMMAND_DESCRIPTION: &str =
    "A third-party cargo extension to run the build artifacts through tools like `gdb`";

#[derive(Deserialize, Debug)]
struct BuildOpt {
    features: Vec<String>,
    filenames: Vec<std::path::PathBuf>,
    fresh: bool,
    package_id: String,
    profile: Profile,
    reason: String,
    target: Target,
}

#[derive(Deserialize, Debug)]
struct Profile {
    debug_assertions: bool,
    debuginfo: Option<u32>,
    opt_level: String,
    overflow_checks: bool,
    test: bool,
}

#[derive(Deserialize, Debug)]
struct Target {
    crate_types: Vec<String>,
    edition: String,
    kind: Vec<String>,
    name: String,
    src_path: std::path::PathBuf,
}

#[derive(Debug)]
struct CargoCmd<'a> {
    cmd: &'a [String],
    downstream_args: Vec<String>,
}

impl<'a> CargoCmd<'a> {
    fn new(args: &'a [String]) -> Result<Self, Error> {
        debug!("Cargo subcommand: {}", args.join(" "));
        if !args.starts_with(&["run".to_string()]) && !args.starts_with(&["test".to_string()]) {
            Err(err_msg(
                "Only the 'run' and 'test' cargo commands are supported",
            ))?;
        }
        let mut iter = args.split(|s| s == "--");
        let cmd = iter
            .next()
            .ok_or_else(|| err_msg("Invalid cargo command"))?;
        let downstream_args: Vec<_> = iter.flatten().cloned().collect();
        Ok(CargoCmd {
            cmd,
            downstream_args,
        })
    }

    /// Builds the new artifact. Replaces the cargo-command 'run' by 'build', in order to avoid the execution.
    fn create_artifact(&self) -> Result<std::path::PathBuf, Error> {
        let cargo_sub = if self.cmd[0] == "run" {
            "build"
        } else {
            self.cmd[0].as_str()
        };
        let args = [cargo_sub.to_string(), "--message-format=json".to_string()];
        let args = args.into_iter().chain(self.cmd[1..].iter());

        debug!(
            "Executing `cargo {:?}`",
            args.clone()
                .cloned()
                .collect::<Vec<_>>()
                .as_slice()
                .join(" ")
        );
        let build_out = Command::new("cargo")
            .args(args)
            .output()
            .expect("cargo command failed :(");
        if !build_out.status.success() {
            return Err(err_msg("Failed to run cargo command. Try running the original cargo command (without cargo-with)"));
        }
        let artifacts = str::from_utf8(&build_out.stdout).unwrap().lines()
                 // FIXME: There are plenty of errors here! This should really be better handled!
                 .flat_map(serde_json::from_str::<BuildOpt>)
                 .collect::<Vec<_>>();
        // We take the last artifact, but this is really just a guess hoping for the best!
        artifacts
            .last()
            .and_then(|best_guess| best_guess.filenames.get(0))
            .cloned()
            .ok_or_else(|| err_msg("Failed to guess binary file name"))
    }
}

fn process_matches(matches: &clap::ArgMatches) -> Result<(), Error> {
    // The original cargo command
    let matches = matches.subcommand_matches(COMMAND_NAME).unwrap();
    let cargo_cmd = matches
        .values_of("cargo-cmd")
        .ok_or_else(|| err_msg("Failed to parse the cargo command producing the artifact"))?
        .map(String::from)
        .collect::<Vec<_>>();
    let cargo_cmd = CargoCmd::new(&cargo_cmd)?;
    // This is the best guess for the artifact...
    let artifact = cargo_cmd.create_artifact()?;
    let artifact = artifact.to_str().unwrap();

    // The string describing how to envoke the child process
    let mut with_cmd: Vec<_> = matches
        .value_of("with-cmd")
        .unwrap()
        .trim()
        .split(' ')
        .collect();

    // add {bin} and {args} if not present
    if !with_cmd.contains(&"{bin}") {
        with_cmd.push("{bin}");
    }
    if !with_cmd.contains(&"{args}") {
        with_cmd.push("{args}");
    }

    let with_cmd = with_cmd
        .iter()
        .map(|&el| if el == "{bin}" { artifact } else { el });
    let args = cargo_cmd.downstream_args;
    let with_cmd = with_cmd
        .flat_map(|el| {
            if el == "{args}" {
                args.to_owned()
            } else {
                vec![el.to_string()]
            }
        }).map(|s| s.to_string())
        .collect::<Vec<_>>();

    debug!(
        "Executing `{:?}`",
        Command::new(with_cmd[0].to_owned()).args(with_cmd[1..].iter())
    );
    Command::new(with_cmd[0].to_owned())
        .args(with_cmd[1..].iter())
        .spawn()
        .expect("Failed to spawn child process")
        .wait()?;
    Ok(())
}

fn create_app<'a, 'b>() -> App<'a, 'b> {
    let usage =
        concat!(
            "<with-cmd> 'Command executed with the cargo-created binary. Use {bin} to denote the binary ",
            "and {args} to denote the arguments passed through cargo following \'--\'; if omitted the ",
            "{bin} and {args} is added as the last arguments'");
    App::new(COMMAND_NAME)
        .about(COMMAND_DESCRIPTION)
        // We have to lie about our binary name since this will be a third party
        // subcommand for cargo, this trick learned from cargo-outdated
        .bin_name("cargo")
        // We use a subcommand because parsed after `cargo` is sent to the third party plugin
        // which will be interpreted as a subcommand/positional arg by clap
        .subcommand(SubCommand::with_name(COMMAND_NAME)
                    .about(COMMAND_DESCRIPTION)
                    .arg(Arg::from_usage(&usage))
                    .arg(clap::Arg::from_usage("<cargo-cmd> 'The cargo commands `test` or `run`'")
                         .raw(true))
        )
        .settings(&[AppSettings::SubcommandRequired])
}

fn main() -> Result<(), Error> {
    env_logger::init();

    let app = create_app();
    let matches = app.get_matches();
    debug!("CLI matches: {:#?}", matches);
    process_matches(&matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ops() {
        let json = "
{\"features\":[],\"filenames\":[\"/home/christian/repos/rust/cargo-dbg/target/debug/cargo_dbg-813f65328e31d537\"],\"fresh\":true,\"package_id\":\"cargo-dbg 0.1.0 (path+file:///home/christian/repos/rust/cargo-dbg)\",\"profile\":{\"debug_assertions\":true,\"debuginfo\":2,\"opt_level\":\"0\",\"overflow_checks\":true,\"test\":true},\"reason\":\"compiler-artifact\",\"target\":{\"crate_types\":[\"bin\"],\"edition\":\"2015\",\"kind\":[\"bin\"],\"name\":\"cargo-dbg\",\"src_path\":\"/home/christian/repos/rust/cargo-dbg/src/main.rs\"}
}";
        let _opts: BuildOpt = serde_json::from_str(json).unwrap();
    }

    #[test]
    fn parse_args() {
        "cargo with \"rr record {}\" -- run --release";
        let app = create_app();
        let _matches = app.get_matches_from(vec![
            "cargo",
            "with",
            "gdb --args {bin} {args}",
            "--",
            "test",
            "--release",
            "--",
            "test2",
        ]);
    }
}
