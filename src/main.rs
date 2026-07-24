use {
  anyhow::{Error, anyhow, bail},
  arguments::Arguments,
  ariadne::{Color, Label, Report, ReportKind, sources},
  clap::Parser,
  env_logger::Env,
  owo_colors::OwoColorize,
  pyproject::{
    Analyzer, BUILTINS, Builtin, Document, Quickfixer, Resolver, RopeExt,
  },
  server::Server,
  similar::TextDiff,
  std::{
    backtrace::BacktraceStatus,
    collections::BTreeMap,
    env, fs,
    path::PathBuf,
    process,
    sync::{
      Arc,
      atomic::{AtomicBool, Ordering},
    },
  },
  subcommand::Subcommand,
  tokio::sync::RwLock,
  tower_lsp::{Client, LanguageServer, LspService, jsonrpc, lsp_types as lsp},
};

mod arguments;
mod server;
mod subcommand;

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
