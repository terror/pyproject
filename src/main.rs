use {
  anyhow::Error,
  document::Document,
  env_logger::Env,
  rope_ext::RopeExt,
  ropey::Rope,
  server::Server,
  std::{
    collections::BTreeMap,
    sync::{
      Arc,
      atomic::{AtomicBool, Ordering},
    },
  },
  tokio::sync::RwLock,
  tower_lsp::{Client, LanguageServer, LspService, jsonrpc, lsp_types as lsp},
};

#[cfg(test)]
use {indoc::indoc, range::Range};

mod document;
mod range;
mod rope_ext;
mod server;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
  let env = Env::default().default_filter_or("info");
  env_logger::Builder::from_env(env).init();
  Server::run().await;
}
