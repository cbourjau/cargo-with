use failure::{err_msg, Error};
use log::debug;
use void::Void;

use std::process::Command;

use crate::cargo;

/// `cargo_cmd_iter` is an iterator over the cargo subcommand with arguments
/// `cmd_iter` is an iterator over the the command to run the binary with
pub(crate) fn runner<'a>(
    mut cargo_cmd_iter: impl Iterator<Item = &'a str> + Clone,
    mut cmd_iter: impl Iterator<Item = &'a str> + Clone,
) -> Result<Void, Error> {
    // The cargo subcommand including arguments
    let subcmd_str: Vec<_> = cargo_cmd_iter
        .by_ref()
        .take_while(|el| *el != "--")
        .collect();

    // The remaining elements are the arguments to the binary ({args})
    let args_after_cargo_cmd = cargo_cmd_iter;

    // Make and run the cargo subcommand
    let cargo_cmd = cargo::Cmd::from_strs(subcmd_str)?;
    let buildopts = cargo_cmd.run()?;

    // Select the wanted buildopt
    let buildopt = cargo::select_buildopt(&buildopts, cargo_cmd.kind())?;
    let artifact = buildopt.artifact()?;
    let artifact_str = artifact
        .to_str()
        .ok_or_else(|| err_msg("Filename of artifact contains non-valid UTF-8 characters"))?;

    // The name of the binary to run on the artifact
    let cmd = cmd_iter
        .next()
        .ok_or_else(|| err_msg("Empty with command"))?;

    // To ensure that we can always handle situations where the user puts quotes
    // around the special arguments, we rather treat the arguments as a string and use search and
    // replace to append the artifact string and additional arguments. This is a detour that could
    // perhaps be done in a more elegant way.
    let mut args_str = cmd_iter.collect::<Vec<_>>().join(" ");
    if args_str.contains("{bin}") {
        args_str = args_str.replace("{bin}", artifact_str);
    } else {
        args_str.push(' ');
        args_str.push_str(artifact_str);
    }
    if args_str.contains("{args}") {
        args_str = args_str.replace(
            "{args}",
            &args_after_cargo_cmd.collect::<Vec<_>>().join(" "),
        );
    } else {
        args_str.push(' ');
        args_str.push_str(&args_after_cargo_cmd.collect::<Vec<_>>().join(" "));
    }
    let expanded_args = args_str.split_whitespace();

    debug!(
        "Executing `{} {}`",
        cmd,
        expanded_args.clone().collect::<Vec<_>>().join(" ")
    );

    exec(Command::new(cmd).args(expanded_args))
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
