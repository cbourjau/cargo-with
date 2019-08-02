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

    let mut expanded_args = vec![];
    let mut found_bin = false;
    let mut found_args = false;
    for arg in cmd_iter {
        if arg == "{args}" {
            found_args = true;
            expanded_args.extend(args_after_cargo_cmd.clone().map(|arg| arg.to_owned()));
        } else {
            found_bin |= arg.contains("{bin}");
            expanded_args.push(arg.replace("{bin}", artifact_str));
        }
    }
    if !found_bin {
        expanded_args.push(artifact_str.to_owned());
    }
    if !found_args {
        expanded_args.extend(args_after_cargo_cmd.clone().map(|arg| arg.to_owned()));
    }

    debug!("Executing `{} {}`", cmd, expanded_args.join(" "));

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
