//! `nrmeasure`: the neurorust measurement capture and provenance harness CLI.

use clap::{Parser, Subcommand};

mod cmd;
mod rundir;
mod tools;

/// Measurement capture and provenance harness for neurorust.
#[derive(Parser, Debug)]
#[command(
    name = "nrmeasure",
    version,
    about = "Measurement capture and provenance harness for neurorust"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Execute a measurement run and write a stamped run directory
    Run(cmd::run::Args),
    /// Validate every run directory under measurements against its manifest
    Verify(cmd::verify::Args),
    /// Build a manifest for a capture taken before the harness existed
    Reconstruct(cmd::reconstruct::Args),
    /// Append runs to the metrics series and compare against the baseline
    Series(cmd::series::Args),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Run(args) => cmd::run::run(args),
        Command::Verify(args) => cmd::verify::run(args),
        Command::Reconstruct(args) => cmd::reconstruct::run(args),
        Command::Series(args) => cmd::series::run(args),
    };

    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn help_text_is_ascii() {
        let help = Cli::command().render_help().to_string();
        assert!(
            help.bytes().all(|b| b < 0x80),
            "help text contains a non-ASCII byte: {help:?}"
        );
    }

    #[test]
    fn help_text_lists_all_four_subcommands() {
        let help = Cli::command().render_help().to_string();
        for name in ["run", "verify", "reconstruct", "series"] {
            assert!(help.contains(name), "help text missing subcommand {name:?}");
        }
    }
}
