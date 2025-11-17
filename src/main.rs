use {
  document::Document,
  env_logger::Env,
  ropey::Rope,
  server::Server,
  std::{
    collections::BTreeMap,
    sync::{
      Arc, RwLock,
      atomic::{AtomicBool, Ordering},
    },
  },
  tower_lsp::{Client, LanguageServer, LspService, jsonrpc, lsp_types as lsp},
};

mod document;
mod server;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
  let env = Env::default().default_filter_or("info");
  env_logger::Builder::from_env(env).init();
  Server::run().await;
}
