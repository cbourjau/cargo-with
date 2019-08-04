use std::process::Command;

use failure::{err_msg, Error};

pub(crate) struct WithCmd<'a> {
    // Raw command given as first argument; the {bin} placeholder is
    // not yet replaced with the expanded value. The first element is
    // the executable.
    split_cmd: Vec<&'a str>,
}

impl<'a> WithCmd<'a> {
    /// Parse the command string which was passed in as the first
    /// argument. Currently, we just split on whitespaces which is not
    /// correct if there are quotes
    pub fn new(raw: &'a str, trailing_args: &[&'a str]) -> Self {
        // Example of a raw at this point "echo {bin} something"
        // Splitting on whitspaces is bad but simple
        let mut split_raw: Vec<_> = raw.split_whitespace().collect();
        // Make sure that we have {bin} and {args} somewhere. We look
        // for it in the original string to not get into trouble with
        // a whitespaces.
        if !raw.contains("{bin}") {
            split_raw.push("{bin}");
        }
        if !raw.contains("{args}") {
            split_raw.push("{args}");
        }
        // Construct final split args and replace {args}
        let mut split_cmd = vec![];
        for el in split_raw {
            if el == "{args}" {
                split_cmd.extend_from_slice(&trailing_args);
            } else {
                split_cmd.push(el);
            }
        }
        Self { split_cmd }
    }

    /// Produce the ready-to-execute `Command` struct with all
    /// occurrences of {bin} and {args} replaced
    pub fn child_command(&self, bin_path: &str) -> Result<Command, Error> {
        if let Some((bin, args)) = self.split_cmd.split_first() {
            let replaced_args = args.iter().map(|el| el.replace("{bin}", bin_path));
            let mut cmd = Command::new(bin.replace("{bin}", bin_path));
            cmd.args(replaced_args);
            Ok(cmd)
        } else {
            Err(err_msg("No child command given."))
        }
    }
}
