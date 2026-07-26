use {
  anyhow::{Error, anyhow, bail},
  arguments::Arguments,
  ariadne::{Color, Label, Report, ReportKind, sources},
  clap::Parser,
  owo_colors::OwoColorize,
  pyproject::{
    Analyzer, BUILTINS, Builtin, Document, Quickfixer, Resolver, RopeExt,
    SCHEMAS,
  },
  serde_json::Value,
  server::Server,
  similar::TextDiff,
  std::{
    backtrace::BacktraceStatus,
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    io::stderr,
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
  tracing_subscriber::{EnvFilter, filter::LevelFilter},
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

  let filter = EnvFilter::builder()
    .with_default_directive(LevelFilter::INFO.into())
    .from_env_lossy();

  tracing_subscriber::fmt()
    .with_writer(stderr)
    .with_env_filter(filter)
    .init();

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
