use failure::{err_msg, format_err, Error};
use log::debug;
use serde::Deserialize;

use std::path::PathBuf;
use std::process::Command;
use std::{iter, str};

const DEFAULT_CARGO_ARGS: &[&str] = &["--message-format=json", "--quiet"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CmdKind {
    Run,
    Test,
    Bench,
}

impl CmdKind {
    /// Turns a string into a CmdKind
    fn from_str(s: &str) -> Option<Self> {
        use self::CmdKind::*;
        match s {
            "run" => Some(Run),
            "test" => Some(Test),
            "bench" => Some(Bench),
            _ => None,
        }
    }
    /// Returns the respective command kind as a command to pass to
    /// artifact generation
    fn as_artifact_cmd(self) -> &'static str {
        match self {
            CmdKind::Run => "build",
            CmdKind::Test => "test",
            CmdKind::Bench => "bench",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Cmd<'a> {
    kind: CmdKind,
    args: Vec<&'a str>,
}

impl<'a> Cmd<'a> {
    /// Create a command from the given strings
    pub(crate) fn from_strs(strs: impl IntoIterator<Item = &'a str>) -> Result<Self, Error> {
        let mut strs = strs.into_iter();

        let kind = strs
            .next()
            .ok_or_else(|| err_msg("Empty cargo command"))
            .and_then(|kind_str| {
                CmdKind::from_str(kind_str).ok_or({
                    format_err!("Unable to convert '{}' into a cargo subcommand", kind_str)
                })
            })?;

        Ok(Cmd {
            kind,
            args: strs.collect(),
        })
    }
    pub(crate) fn kind(&self) -> CmdKind {
        self.kind
    }
    /// Get the arguments which would be passed to `cargo`
    ///
    /// Includes the type of command (e.g `test`, `run`), the default arguments
    /// (`DEFAULT_CARGO_ARGS`) and the `--no-run` flag if we are trying to compiling
    /// tests/benchmarks. `--no-run` ensures that we do not run the resulting binary when compiling
    /// tests/benchmarks.
    fn args(&self) -> impl Iterator<Item = &str> + Clone {
        let no_run_flag = match self.kind {
            CmdKind::Test | CmdKind::Bench => Some("--no-run"),
            _ => None,
        };

        iter::once(self.kind.as_artifact_cmd())
            .chain(DEFAULT_CARGO_ARGS.iter().cloned())
            .chain(no_run_flag)
            .chain(self.args.iter().cloned())
    }

    /// Run the cargo command and get the output back as a vector
    pub(crate) fn run(&self) -> Result<Vec<BuildOpt>, Error> {
        debug!(
            "Executing `cargo {}`",
            self.args().collect::<Vec<_>>().join(" ")
        );

        let build_out = Command::new("cargo")
            .args(self.args())
            .output()
            .map_err(|_| {
                format_err!(
                    "Unable to run cargo command: `cargo {}`",
                    self.args().collect::<Vec<_>>().join(" ")
                )
            })?;

        if !build_out.status.success() {
            Err(format_err!(
                "{}\n{}\nCargo subcommand failed. Try running the original cargo command (without cargo-with)",
                str::from_utf8(&build_out.stderr).unwrap(),
                str::from_utf8(&build_out.stdout).unwrap()
            ))?;
        }

        let opts = str::from_utf8(&build_out.stdout)
            .map_err(|_| {
                format_err!(
                    "Output of `cargo {}` contained invalid UTF-8 characters",
                    self.args().collect::<Vec<_>>().join(" ")
                )
            })?
            .lines()
            // FIXME: There are plenty of errors here! This should really be better handled!
            .flat_map(serde_json::from_str::<BuildOpt>)
            .collect();

        Ok(opts)
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct BuildOpt {
    features: Vec<String>,
    filenames: Vec<PathBuf>,
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

/// Most possible targetkinds taken from
/// [`TargetKind`](https://docs.rs/cargo/0.31.0/cargo/core/manifest/enum.TargetKind.html).
/// See the implementation of `Serialize` for `TargetKind` to see how the enum
/// is serialized (does not serialize as one would expect based on type
/// signature).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum TargetKind {
    Example,
    Test,
    Bin,
    Lib,
    Rlib,
    Dylib,
    ProcMacro,
    Bench,
    CustomBuild,
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match *self {
            TargetKind::Example => "example",
            TargetKind::Test => "test",
            TargetKind::Bin => "bin",
            TargetKind::Lib => "lib",
            TargetKind::Rlib => "rlib",
            TargetKind::Dylib => "dylib",
            TargetKind::ProcMacro => "proc-macro",
            TargetKind::Bench => "bench",
            TargetKind::CustomBuild => "custom-build",
        };
        write!(f, "{}", name)
    }
}

#[derive(Deserialize, Debug)]
struct Target {
    crate_types: Vec<String>,
    edition: String,
    kind: Vec<TargetKind>,
    name: String,
    src_path: PathBuf,
}

/// Selects the buildopt which fits with the requirements
///
/// If there are multiple possible candidates, this will return an error
pub(crate) fn select_buildopt<'a>(
    opts: impl IntoIterator<Item = &'a BuildOpt>,
    cmd_kind: CmdKind,
) -> Result<&'a BuildOpt, Error> {
    // Target kinds we want to look for
    let look_for = &[TargetKind::Bin, TargetKind::Example, TargetKind::Test];

    // Find candidates with the possible target types
    let candidates: Vec<_> = opts
        .into_iter()
        .filter(|opt| {
            // When run as a test or bench we only care about the
            // binary where the profile is set as `test`
            match cmd_kind {
                CmdKind::Test | CmdKind::Bench => opt.profile.test,
                CmdKind::Run => opt
                    .target
                    .kind
                    .iter()
                    .any(|kind| look_for.iter().any(|lkind| lkind == kind)),
            }
        })
        .collect();
    // We expect exactly one candidate; everything else is an error
    match candidates.as_slice() {
        [] => Err(err_msg("No suitable build artifacts found.")),
        [the_one] => Ok(the_one),
        the_many => Err(format_err!(
            "Found several artifact candidates: \n :{:?}",
            the_many
        )),
    }
}

impl BuildOpt {
    /// Best guess for the build artifact associated with this `BuildOpt`
    pub(crate) fn artifact(&self) -> Result<PathBuf, Error> {
        Ok(self.filenames[0].clone())
    }
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
