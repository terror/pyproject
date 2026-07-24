use {
  crate::{arguments::Arguments, subcommand::Subcommand},
  anyhow::{Error, anyhow, bail},
  ariadne::{Color, Label, Report, ReportKind, sources},
  clap::Parser,
  env_logger::Env,
  owo_colors::OwoColorize,
  similar::TextDiff,
  std::{backtrace::BacktraceStatus, env, fs, path::PathBuf, process},
  tower_lsp::lsp_types as lsp,
};

mod arguments;
mod subcommand;

use pyproject::*;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
  if env::var_os("NO_COLOR").is_some() {
    yansi::disable();
  }

  let env = Env::default().default_filter_or("info");

  env_logger::Builder::from_env(env).init();

  if let Err(error) = Arguments::parse().run().await {
    eprintln!("error: {error}");

    for (i, error) in error.chain().skip(1).enumerate() {
      if i == 0 {
        eprintln!();
        eprintln!("because:");
      }

      eprintln!("- {error}");
    }

    let backtrace = error.backtrace();

    if backtrace.status() == BacktraceStatus::Captured {
      eprintln!("backtrace:");
      eprintln!("{backtrace}");
    }

    process::exit(1);
  }
}
