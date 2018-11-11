#[macro_use]
extern crate log;
extern crate failure;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::cell::Cell;
use std::iter::once;
use std::process::Command;

use failure::{err_msg, format_err, Error};

mod cargo;
use cargo::{BuildOpt, TargetKind};

const DEFAULT_CARGO_ARGS: &[&str] = &["--message-format=json", "--quiet"];

/// `cargo_cmd_iter` is an iterator over the cargo subcommand with arguments
/// `cmd_iter` is an iterator over the the command to run the binary with
pub fn run<'a>(
    mut cargo_cmd_iter: impl Iterator<Item = &'a str> + Clone,
    mut cmd_iter: impl Iterator<Item = &'a str> + Clone,
) -> Result<(), Error> {
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
    let buildopt = select_buildopt(&buildopts, cargo_cmd.kind())?;
    let artifact_path = buildopt.filenames[0].to_str().ok_or(err_msg(
        "Filename of artifact contains non-valid UTF-8 characters",
    ))?;

    // The name of the binary to run on the artifact
    let cmd = cmd_iter.next().ok_or(err_msg("Empty with command"))?;

    // The remaining elements are the arguments to the binary
    let args = cmd_iter;

    // Variables to check if we found the bin and args in the command
    // arguments. This prevents the need to search through the string to check
    // if {bin}/{args} exists
    let found_bin = Cell::new(false);
    let found_args = Cell::new(false);

    let mut expanded_args: Vec<_> = args
        // We have to use a box because impl Trait is not supported in closures
        .flat_map(|el| -> Box<Iterator<Item = &str>> {
            match el {
                "{bin}" => {
                    found_bin.set(true);
                    Box::new(once(artifact_path))
                }
                "{args}" => {
                    found_args.set(true);
                    Box::new(args_after_cargo_cmd.clone())
                }
                _ => Box::new(once(el)),
            }
        })
        .collect();

    // If we did not find {bin}/{args} we append the bin and args to the end of
    // the arguments
    if !found_bin.get() {
        expanded_args.push(artifact_path);
    }
    if !found_args.get() {
        // Using `expanded_args.extend(cargo_cmd_iter)` gives a lifetime error, hence we
        // rather push one element at a time
        for arg in args_after_cargo_cmd {
            expanded_args.push(arg);
        }
    }

    debug!("Executing `{} {}`", cmd, expanded_args.join(" "));

    Command::new(cmd)
        .args(expanded_args)
        .spawn()
        .expect("Failed to spawn child process")
        .wait()?;

    Ok(())
}

/// Selects the buildopt which fits with the requirements
///
/// If there are multiple possible candidates, this will return an error
fn select_buildopt<'a>(
    opts: impl IntoIterator<Item = &'a BuildOpt>,
    cmd_kind: cargo::CmdKind,
) -> Result<&'a BuildOpt, Error> {
    let opts = opts.into_iter();

    // Target kinds we want to look for
    let look_for = &[TargetKind::Bin, TargetKind::Example, TargetKind::Test];
    let is_test = cmd_kind == cargo::CmdKind::Test;

    // Find candidates with the possible target types
    let mut candidates = opts
        .filter(|opt| {
            // When run as a test we only care about the binary where the profile
            // is set as `test`
            if is_test {
                opt.profile.test
            } else {
                opt.target
                    .kind
                    .iter()
                    .any(|kind| look_for.iter().any(|lkind| lkind == kind))
            }
        })
        .peekable();

    // Get the first candidate
    let first = candidates
        .next()
        .ok_or(err_msg("Found no possible candidates"))?;

    // We found more than one candidate
    if candidates.peek().is_some() {
        // Make a error string including all the possible candidates
        let candidates_str = candidates
            .map(|opt| format!("\t- {} ({})", opt.target.name, opt.target.kind[0]))
            .collect::<Vec<_>>()
            .join("\n");

        if is_test {
            Err(format_err!("Found more than one possible candidate:\n\n\t- {} ({})\n{}\n\nPlease use `--test`, `--example`, `--bin` or `--lib` to specify exactly what binary you want to examine", first.target.name, first.target.kind[0], candidates_str))?
        } else {
            Err(format_err!("Found more than one possible candidate:\n\n\t- {} ({})\n{}\n\nPlease use `--example` or `--bin` to specify exactly what binary you want to examine", first.target.name, first.target.kind[0], candidates_str))?
        }
    }
    Ok(first)
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
