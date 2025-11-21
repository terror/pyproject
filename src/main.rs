use {
  anyhow::Error,
  arguments::Arguments,
  clap::Parser,
  document::Document,
  env_logger::Env,
  rope_ext::RopeExt,
  ropey::Rope,
  server::Server,
  std::{
    backtrace::BacktraceStatus,
    collections::BTreeMap,
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

#[cfg(test)]
use {indoc::indoc, range::Range};

mod arguments;
mod document;
mod range;
mod rope_ext;
mod server;
mod subcommand;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
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
