use failure::{err_msg, format_err, Error};

use std::process::Command;
use std::{iter, str};

use super::DEFAULT_CARGO_ARGS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmdKind {
    Run,
    Test,
}

impl CmdKind {
    /// Turns a string into a CmdKind
    fn from_str(s: &str) -> Option<Self> {
        use self::CmdKind::*;
        match s {
            "run" => Some(Run),
            "test" => Some(Test),
            _ => None,
        }
    }
    /// Returns the respective command kind as a command to pass to
    /// artifact generation
    fn as_artifact_cmd(&self) -> &'static str {
        match *self {
            CmdKind::Run => "build",
            CmdKind::Test => "test",
        }
    }
}

#[derive(Debug)]
pub struct Cmd<'a> {
    kind: CmdKind,
    args: Vec<&'a str>,
}

impl<'a> Cmd<'a> {
    /// Create a command from the given strings
    pub fn from_strs(strs: impl IntoIterator<Item = &'a str>) -> Result<Self, Error> {
        let mut strs = strs.into_iter();

        let kind = strs
            .next()
            .ok_or(err_msg("Empty cargo command"))
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
    pub fn kind(&self) -> CmdKind {
        self.kind
    }
    /// Get the arguments which would be passed to `cargo`
    ///
    /// Includes the type of command (e.g `test`, `run`) and the default
    /// arguments (`DEFAULT_CARGO_ARGS`).
    fn args(&self) -> impl Iterator<Item = &str> + Clone {
        iter::once(self.kind.as_artifact_cmd())
            .chain(DEFAULT_CARGO_ARGS.iter().map(|s| *s))
            .chain(self.args.iter().map(|s| *s))
    }
    /// Turn the arguements into a space separated string
    fn args_str(&self) -> String {
        self.args().fold(String::new(), |mut acc, arg| {
            if !acc.is_empty() {
                acc.push(' ');
            }
            acc += arg;
            acc
        })
    }

    /// Run the cargo command and get the output back as a vector
    pub fn run(&self) -> Result<Vec<BuildOpt>, Error> {
        debug!("Executing `cargo {}`", self.args_str());

        let build_out = Command::new("cargo")
            .args(self.args())
            .output()
            .map_err(|_| format_err!("Unable to run cargo command: `cargo {}`", self.args_str()))?;

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
                    self.args_str()
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
pub struct BuildOpt {
    pub features: Vec<String>,
    pub filenames: Vec<std::path::PathBuf>,
    pub fresh: bool,
    pub package_id: String,
    pub profile: Profile,
    pub reason: String,
    pub target: Target,
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub debug_assertions: bool,
    pub debuginfo: Option<u32>,
    pub opt_level: String,
    pub overflow_checks: bool,
    pub test: bool,
}

/// Most possible targetkinds taken from
/// [`TargetKind`](https://docs.rs/cargo/0.31.0/cargo/core/manifest/enum.TargetKind.html).
/// See the implementation of `Serialize` for `TargetKind` to see how the enum
/// is serialized (does not serialize as one would expect based on type
/// signature).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
pub struct Target {
    pub crate_types: Vec<String>,
    pub edition: String,
    pub kind: Vec<TargetKind>,
    pub name: String,
    pub src_path: std::path::PathBuf,
}
