use std::iter::once;
use std::process::Command;

use failure::{err_msg, Error};

use cargo;

/// `cargo_cmd_iter` is an iterator over the cargo subcommand with arguments
/// `cmd_iter` is an iterator over the the command to run the binary with
pub(crate) fn runner<'a>(
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
    let buildopt = cargo::select_buildopt(&buildopts, cargo_cmd.kind())?;
    let artifact = buildopt.artifact()?;
    let artifact_str = artifact
        .to_str().ok_or(err_msg(
            "Filename of artifact contains non-valid UTF-8 characters",
        ))?;

    // The name of the binary to run on the artifact
    let cmd = cmd_iter.next().ok_or(err_msg("Empty with command"))?;

    // The remaining elements are the arguments to the binary
    // Since we will have to search for {bin} and {args} we just
    // collect it into Vec here for simplicity.
    let mut args: Vec<&str> = cmd_iter.collect();
    if args.iter().find(|&&s| s == "{bin}").is_none() {
        args.push("{bin}");
    }
    if args.iter().find(|&&s| s == "{args}").is_none() {
        args.push("{args}");
    }
    // Replace the {bin} and {args} placeholders
    let expanded_args: Vec<_> = args.into_iter()
        // We have to use a box because impl Trait is not supported in closures
        .flat_map(|s| -> Box<Iterator<Item = &str>> {
            match s {
            "{bin}" => Box::new(once(artifact_str)),
            "{args}" => Box::new(args_after_cargo_cmd.clone()),
            _ => Box::new(once(s)),
            }}).
        collect();

    debug!("Executing `{} {}`", cmd, expanded_args.join(" "));

    Command::new(cmd)
        .args(expanded_args)
        .spawn()
        .expect("Failed to spawn child process")
        .wait()?;

    Ok(())
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
