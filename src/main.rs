use std::process::Command;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use failure::{err_msg, Error};
use log::debug;
use void::{unreachable, Void};

mod cargo_command;
mod with_command;

use crate::cargo_command::CargoCmd;
use crate::with_command::WithCmd;

const COMMAND_NAME: &str = "with";
const COMMAND_DESCRIPTION: &str =
    "A third-party cargo extension to run the build artifacts through tools like `gdb`";

fn main() {
    match try_main() {
        Ok(v) => unreachable(v),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

// Make a separate runner to print errors using Display instead of Debug
fn try_main() -> Result<Void, Error> {
    env_logger::init();

    let app = create_app();
    let matches = app.get_matches();

    debug!("CLI matches: {:#?}", matches);

    let (with_cmd, cargo_cmd) = process_matches(&matches)?;
    // TODO: This should also be a void return type
    let artifact_path = cargo_cmd.run()?.artifact()?;
    let artifact = artifact_path
        .to_str()
        .ok_or_else(|| err_msg("Binary path is not valid utf-8"))?;
    let mut finalized_with_cmd = with_cmd.child_command(artifact)?;
    exec(&mut finalized_with_cmd)
}

/// Process command line arguments. The input is split up into three
/// logical units:
/// `<cargo-with-cmd> -- <cargo-command> -- <user-args>`
/// Thus, the command `cargo with echo -- run -- my-args` is split up into
/// `[echo]`, `[run]`, `[my-args]`
fn process_matches<'a>(matches: &'a ArgMatches<'_>) -> Result<(WithCmd<'a>, CargoCmd<'a>), Error> {
    // A prelude to work around the fact that this is run as `cargo
    // with` and not `cargo-with`
    let matches = matches
        .subcommand_matches(COMMAND_NAME)
        .ok_or_else(|| err_msg("Failed to parse the correct subcommand"))?;
    // The string describing how to envoke the child process
    let raw_with_cmd = matches
        .value_of("with-cmd")
        .ok_or_else(|| err_msg("Failed to parse the command to run with the artifact"))?
        .trim();
    // everything after the first `--`
    let mut cargo_cmd_and_args = matches
        .values_of("cargo-cmd")
        .ok_or_else(|| err_msg("Failed to parse the cargo command producing the artifact"))?;
    let cargo_cmd = cargo_cmd_and_args.by_ref().take_while(|&el| el != "--");
    let cargo_cmd = CargoCmd::from_strs(cargo_cmd)?;
    let trailing_args: Vec<_> = cargo_cmd_and_args.collect();
    let with_cmd = WithCmd::new(raw_with_cmd, &trailing_args);
    Ok((with_cmd, cargo_cmd))
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
                .after_help(
                    r#"
EXAMPLES:
   cargo with echo -- run
   cargo with "gdb --args" -- run
   cargo with "echo {args} {bin}" -- test -- myargs
"#,
                ),
        )
        .settings(&[AppSettings::SubcommandRequired])
}

#[cfg(unix)]
fn exec(command: &mut Command) -> Result<Void, Error> {
    use std::os::unix::process::CommandExt;
    Err(command.exec())?
}

#[cfg(not(unix))]
fn exec(command: &mut Command) -> Result<Void, Error> {
    std::process::exit(
        command
            .status()?
            .code()
            .expect("Process terminated by signal"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    // Evocations for test projects which are expected to succeed
    const TEST_PROJECTS: &[(&str, &[&[&str]])] = &[
        (
            "./example_projects/simple-binary/",
            &[
                &["cargo", "with", "echo", "--", "run"],
                &["cargo", "with", "echo {bin}", "--", "run"],
                &["cargo", "with", "echo {bin} {args}", "--", "run"],
                &["cargo", "with", "echo", "--", "run"],
            ],
        ),
        (
            "./example_projects/simple-library/",
            &[
                &["cargo", "with", "echo", "--", "test"],
                &["cargo", "with", "echo {bin}", "--", "test"],
                &["cargo", "with", "echo {bin} {args}", "--", "test"],
                &["cargo", "with", "echo", "--", "test"],
            ],
        ),
        (
            "./example_projects/benchmark/",
            &[&["cargo", "with", "echo", "--", "bench"]],
        ),
    ];
    // Evocations for test projects which are expected to fail
    const TEST_PROJECTS_FAILS: &[(&str, &[&[&str]])] = &[(
        "/home/christian/repos/rust/cargo-dbg/example_projects/simple-binary/",
        &[
            &["cargo", "with", "echo"],
            &["cargo", "with", "echo", "--"],
            &["cargo", "with", "not-a-command"],
            &["cargo", "with", "echo", "--", "not-a-cargo-command"],
            &["cargo", "with", "not-a-command", "--", "run"],
        ],
    )];

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

    #[test]
    fn exec_test_project_success() {
        for (project_dir, evocs) in TEST_PROJECTS {
            dbg!(project_dir);
            for evoc in *evocs {
                println!("Running {:?}", evoc);
                let matches = create_app().get_matches_from(*evoc);
                let (with_cmd, cargo_cmd) = process_matches(&matches).unwrap();
                let artifact_path = cargo_cmd.run().unwrap().artifact().unwrap();
                let artifact = artifact_path
                    .to_str()
                    .ok_or_else(|| err_msg("Binary path is not valid utf-8"))
                    .unwrap();
                let mut with_cmd = with_cmd.child_command(artifact).unwrap();
                with_cmd.current_dir(project_dir);
                assert!(with_cmd.status().unwrap().success());
            }
        }
    }

    #[test]
    fn exec_test_project_fails() {
        for (project_dir, evocs) in TEST_PROJECTS_FAILS {
            for evoc in *evocs {
                println!("Running {:?}", evoc);
                if let Ok(matches) = create_app().get_matches_from_safe(*evoc) {
                    if let Ok((with_cmd, cargo_cmd)) = process_matches(&matches) {
                        let artifact_path = cargo_cmd.run().unwrap().artifact().unwrap();
                        let artifact = artifact_path
                            .to_str()
                            .ok_or_else(|| err_msg("Binary path is not valid utf-8"))
                            .unwrap();
                        let mut with_cmd = with_cmd.child_command(artifact).unwrap();
                        with_cmd.current_dir(project_dir);

                        assert!(with_cmd.output().is_err());
                    }
                }
            }
        }
    }
}
