use tower_lsp::{
  Client, LanguageServer, LspService, Server, jsonrpc::Result, lsp_types::*,
};

#[derive(Debug)]
struct Backend {
  client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
  async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult::default())
  }

  async fn initialized(&self, _: InitializedParams) {
    self
      .client
      .log_message(MessageType::INFO, "server initialized")
      .await;
  }

  async fn shutdown(&self) -> Result<()> {
    Ok(())
  }
}

#[tokio::main]
async fn main() {
  let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
  let (service, socket) = LspService::new(|client| Backend { client });
  Server::new(stdin, stdout, socket).serve(service).await;
}
