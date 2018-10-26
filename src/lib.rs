#[macro_use]
extern crate log;
extern crate failure;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::process::Command;

use failure::{err_msg, format_err, Error};

mod cargo;
use cargo::BuildOpt;

const DEFAULT_CARGO_ARGS: &[&str] = &["--message-format=json", "--quiet"];

pub fn run<'a, 'b>(
    cargo_cmd_iter: impl Iterator<Item = &'a str>,
    mut cmd_iter: impl Iterator<Item = &'b str>,
) -> Result<(), Error> {
    let cargo_cmd = cargo::Cmd::from_strs(cargo_cmd_iter)?;
    let buildopts = cargo_cmd.run()?;

    // Select the wanted buildopt
    let buildopt = select_buildopt(buildopts.iter(), cargo_cmd.kind())?;
    let artifact_path = buildopt.filenames[0].to_str().ok_or(format_err!(""))?;

    // Separate the command from the arguments
    let cmd = cmd_iter.next().ok_or(err_msg("Empty with command"))?;
    let mut args: Vec<_> = cmd_iter.collect();

    // Try to replace {bin} in the arguments with the artifact path
    let replaced_bin = args
        .iter_mut()
        .find(|el| *el == &mut "{bin}")
        .map(|el| *el = artifact_path)
        .is_some();

    // If we did not find {bin} in the args, add the artifact path as the last argument
    if !replaced_bin {
        args.push(artifact_path);
    }

    debug!("Executing `{} {}`", cmd, args.join(" "));

    Command::new(cmd)
        .args(args)
        .spawn()
        .expect("Failed to spawn child process")
        .wait()?;

    Ok(())
}

/// Selects the buildopt which fits with the requirements
///
/// If there are multiple possible candidates, this will return an error
fn select_buildopt<'a>(
    opts: impl Iterator<Item = &'a BuildOpt>,
    _cmd_kind: cargo::CmdKind,
) -> Result<&'a BuildOpt, Error> {
    opts.last().ok_or(err_msg("Did not find any buildopts"))
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
}
