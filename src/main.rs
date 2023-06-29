use camino::Utf8PathBuf;
use clap::crate_authors;
use clap::{ArgGroup, CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{generate, Shell as CompletionShell};
use color_eyre::eyre::Result;
use shadow_rs::shadow;
use std::fs;
use std::io;

// https://crates.io/crates/shadow-rs
shadow!(build);

mod cmd;
mod defaults;
mod errors;

use self::cmd::{apply_defaults, dump, process_path};
use crate::errors::DefaultsError as E;

#[derive(Parser, Debug)]
#[clap(
    author=crate_authors!(),
    version=build::PKG_VERSION,
    long_version=build::CLAP_LONG_VERSION,
    about="Generate and apply macOS defaults.",
    subcommand_required=true,
    arg_required_else_help=true,
)]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    /// Don’t actually run anything.
    #[arg(short, long)]
    dry_run: bool,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    /// Clap subcommand to run.
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Set macOS defaults in plist files.
    Apply {
        /// Sets the input file or path to use.
        #[arg(required = true, value_hint = ValueHint::FilePath)]
        path: Utf8PathBuf,
    },

    /// Generate shell completions to stdout.
    Completions {
        #[clap(value_enum)]
        shell: CompletionShell,
    },

    /// Dump existing defaults as YAML.
    #[clap(group(
    ArgGroup::new("dump")
        .required(true)
        .args(&["domain", "global_domain"]),
    ))]
    Dump {
        /// Read from the current host.
        #[arg(short, long)]
        current_host: bool,

        /// Read from the global domain.
        #[clap(short, long)]
        global_domain: bool,

        /// Domain to generate.
        #[clap(short, long)]
        domain: Option<String>,

        /// Path to YAML file for dump output.
        #[arg(value_hint = ValueHint::FilePath)]
        path: Option<Utf8PathBuf>,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = CLI::parse();

    env_logger::Builder::new().filter_level(cli.verbose.log_level_filter()).init();

    match cli.command {
        Commands::Apply { path } => {
            for p in process_path(path)? {
                fs::metadata(&p).map_err(|e| E::FileRead { path: p.to_owned(), source: e })?;

                apply_defaults(&p)?;
            }

            Ok(())
        }
        Commands::Completions { shell } => {
            generate(shell, &mut CLI::command(), "macos-defaults", &mut io::stdout().lock());

            Ok(())
        }
        Commands::Dump {
            current_host,
            path,
            global_domain,
            domain,
        } => dump(current_host, path, global_domain, domain),
    }
}
