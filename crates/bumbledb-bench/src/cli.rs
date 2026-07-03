//! Hand-rolled command-line parsing (no clap — the quarantine allows
//! rusqlite only). Subcommands land with the PRDs that implement them;
//! today the scaffold parses `help`.

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Print usage and exit 0.
    Help,
}

/// The usage text.
#[must_use]
pub fn help() -> String {
    format!(
        "bumbledb-bench {}\n\
         \n\
         The benchmark and oracle suite (docs/benchmarks/).\n\
         \n\
         USAGE:\n\
         \x20 bumbledb-bench <COMMAND>\n\
         \n\
         COMMANDS:\n\
         \x20 help    print this text\n",
        env!("CARGO_PKG_VERSION")
    )
}

/// Parses raw arguments (without the program name).
///
/// # Errors
///
/// A human-readable message naming the offending token.
pub fn parse(args: &[String]) -> Result<Cmd, String> {
    match args {
        [] => Ok(Cmd::Help),
        [cmd, rest @ ..] if cmd == "help" => {
            if let [extra, ..] = rest {
                return Err(format!("unexpected argument after `help`: `{extra}`"));
            }
            Ok(Cmd::Help)
        }
        [cmd, ..] => Err(format!("unknown command `{cmd}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn help_parses() {
        assert_eq!(parse(&argv(&["help"])), Ok(Cmd::Help));
        assert_eq!(parse(&[]), Ok(Cmd::Help));
    }

    #[test]
    fn garbage_names_the_offending_token() {
        let err = parse(&argv(&["frobnicate"])).unwrap_err();
        assert!(err.contains("frobnicate"), "{err}");
        let err = parse(&argv(&["help", "me"])).unwrap_err();
        assert!(err.contains("me"), "{err}");
    }

    #[test]
    fn help_text_names_the_binary_and_version() {
        let text = help();
        assert!(text.contains("bumbledb-bench"));
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }
}
