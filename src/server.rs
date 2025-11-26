use super::*;

#[derive(Debug)]
pub(crate) struct Server(Arc<Inner>);

impl Server {
  pub(crate) fn capabilities() -> lsp::ServerCapabilities {
    lsp::ServerCapabilities {
      completion_provider: Some(lsp::CompletionOptions {
        resolve_provider: Some(false),
        trigger_characters: Some(vec![
          "[".to_string(),
          ".".to_string(),
          "=".to_string(),
          "\"".to_string(),
          "'".to_string(),
          ",".to_string(),
        ]),
        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
        all_commit_characters: None,
        completion_item: None,
      }),
      hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
      document_formatting_provider: Some(lsp::OneOf::Left(true)),
      text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
        lsp::TextDocumentSyncOptions {
          open_close: Some(true),
          change: Some(lsp::TextDocumentSyncKind::INCREMENTAL),
          will_save: None,
          will_save_wait_until: None,
          save: Some(
            lsp::SaveOptions {
              include_text: Some(false),
            }
            .into(),
          ),
        },
      )),
      ..Default::default()
    }
  }

  pub(crate) fn new(client: Client) -> Self {
    Self(Arc::new(Inner::new(client)))
  }

  pub(crate) async fn run() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(Server::new);

    tower_lsp::Server::new(stdin, stdout, socket)
      .serve(service)
      .await;
  }
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
  async fn completion(
    &self,
    params: lsp::CompletionParams,
  ) -> Result<Option<lsp::CompletionResponse>, jsonrpc::Error> {
    self.0.completion(params).await
  }

  async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
    if let Err(error) = self.0.did_change(params).await {
      self
        .0
        .client
        .log_message(lsp::MessageType::ERROR, error)
        .await;
    }
  }

  async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
    self.0.did_close(params).await;
  }

  async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
    if let Err(error) = self.0.did_open(params).await {
      self
        .0
        .client
        .log_message(lsp::MessageType::ERROR, error)
        .await;
    }
  }

  async fn formatting(
    &self,
    params: lsp::DocumentFormattingParams,
  ) -> Result<Option<Vec<lsp::TextEdit>>, jsonrpc::Error> {
    self.0.formatting(params).await
  }

  async fn hover(
    &self,
    params: lsp::HoverParams,
  ) -> Result<Option<lsp::Hover>, jsonrpc::Error> {
    self.0.hover(params).await
  }

  #[allow(clippy::unused_async)]
  async fn initialize(
    &self,
    params: lsp::InitializeParams,
  ) -> Result<lsp::InitializeResult, jsonrpc::Error> {
    self.0.initialize(params).await
  }

  async fn initialized(&self, params: lsp::InitializedParams) {
    self.0.initialized(params).await;
  }

  async fn shutdown(&self) -> Result<(), jsonrpc::Error> {
    Ok(())
  }
}

#[derive(Debug)]
struct Inner {
  client: Client,
  documents: RwLock<BTreeMap<lsp::Url, Document>>,
  initialized: AtomicBool,
}

impl Inner {
  fn annotation_description(entry: Option<&Value>) -> Option<String> {
    entry
      .and_then(Value::as_object)
      .and_then(|annotations| annotations.get("description"))
      .and_then(Value::as_str)
      .map(str::to_string)
  }

  async fn completion(
    &self,
    params: lsp::CompletionParams,
  ) -> Result<Option<lsp::CompletionResponse>, jsonrpc::Error> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let documents = self.documents.read().await;

    let Some(document) = documents.get(&uri) else {
      return Ok(None);
    };

    let completions = Completions::new(document, position);

    let items = completions.completions();

    if items.is_empty() {
      return Ok(None);
    }

    Ok(Some(lsp::CompletionResponse::Array(items)))
  }

  fn description_from_schema_location(location: &str) -> Option<String> {
    let (schema_url, fragment) = location
      .split_once('#')
      .map_or(("", location), |(url, fragment)| (url, fragment));

    let schema = if schema_url.is_empty() {
      SchemaStore::root()
    } else {
      SchemaStore::documents().get(schema_url)?
    };

    let mut pointer = if fragment.is_empty() {
      String::new()
    } else if fragment.starts_with('/') {
      fragment.to_string()
    } else {
      format!("/{fragment}")
    };

    loop {
      if let Some(schema_value) = schema.pointer(&pointer)
        && let Some(description) =
          schema_value.get("description").and_then(Value::as_str)
      {
        return Some(description.to_string());
      }

      if pointer.is_empty() {
        return None;
      }

      if let Some(idx) = pointer.rfind('/') {
        if idx == 0 {
          pointer.clear();
        } else {
          pointer.truncate(idx);
        }
      } else {
        return None;
      }
    }
  }

  fn descriptions_for_instance(
    instance: &Value,
    validator: &Validator,
  ) -> HashMap<String, String> {
    let mut descriptions = HashMap::new();

    let evaluation = validator.evaluate(instance);

    for entry in evaluation.iter_annotations() {
      let location = entry
        .absolute_keyword_location
        .map_or(entry.schema_location, Uri::as_str);

      if let Some(description) =
        Self::annotation_description(Some(entry.annotations.value()))
          .or_else(|| Self::description_from_schema_location(location))
      {
        descriptions
          .entry(entry.instance_location.as_str().to_string())
          .or_insert(description);
      }
    }

    descriptions
  }

  async fn did_change(
    &self,
    params: lsp::DidChangeTextDocumentParams,
  ) -> Result {
    let uri = params.text_document.uri.clone();

    let mut documents = self.documents.write().await;

    let Some(document) = documents.get_mut(&uri) else {
      return Ok(());
    };

    document.apply_change(params);

    drop(documents);

    self.publish_diagnostics(&uri).await;

    Ok(())
  }

  async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
    let uri = params.text_document.uri.clone();

    let removed = {
      let mut documents = self.documents.write().await;
      documents.remove(&uri).is_some()
    };

    if removed {
      self.client.publish_diagnostics(uri, vec![], None).await;
    }
  }

  async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) -> Result {
    let uri = params.text_document.uri.clone();

    self
      .documents
      .write()
      .await
      .insert(uri.clone(), Document::from(params));

    self.publish_diagnostics(&uri).await;

    Ok(())
  }

  async fn formatting(
    &self,
    params: lsp::DocumentFormattingParams,
  ) -> Result<Option<Vec<lsp::TextEdit>>, jsonrpc::Error> {
    let uri = params.text_document.uri;

    let documents = self.documents.read().await;

    let Some(document) = documents.get(&uri) else {
      return Ok(None);
    };

    let original = document.content.to_string();

    let end = document
      .content
      .byte_to_lsp_position(document.content.len_bytes());

    drop(documents);

    let formatted =
      taplo::formatter::format(&original, taplo::formatter::Options::default());

    if formatted == original {
      return Ok(Some(vec![]));
    }

    let edit = lsp::TextEdit {
      range: lsp::Range::new(lsp::Position::new(0, 0), end),
      new_text: formatted,
    };

    Ok(Some(vec![edit]))
  }

  async fn hover(
    &self,
    params: lsp::HoverParams,
  ) -> Result<Option<lsp::Hover>, jsonrpc::Error> {
    let lsp::HoverParams {
      text_document_position_params:
        lsp::TextDocumentPositionParams {
          position,
          text_document,
        },
      ..
    } = params;

    let documents = self.documents.read().await;

    let Some(document) = documents.get(&text_document.uri) else {
      return Ok(None);
    };

    let Ok((instance, pointers)) = PointerMap::build(document) else {
      return Ok(None);
    };

    let Some(pointer) = pointers.pointer_for_position(position) else {
      return Ok(None);
    };

    let Ok(validator) = SchemaRule::validator() else {
      return Ok(None);
    };

    let descriptions = Self::descriptions_for_instance(&instance, validator);

    let Some(description) = descriptions.get(pointer.as_str()) else {
      return Ok(None);
    };

    let range = pointers.range_for_pointer(&pointer).span(&document.content);

    Ok(Some(lsp::Hover {
      contents: lsp::HoverContents::Markup(lsp::MarkupContent {
        kind: lsp::MarkupKind::Markdown,
        value: description.clone(),
      }),
      range: Some(range),
    }))
  }

  #[allow(clippy::unused_async)]
  async fn initialize(
    &self,
    _params: lsp::InitializeParams,
  ) -> Result<lsp::InitializeResult, jsonrpc::Error> {
    log::info!("Starting pyproject language server...");

    Ok(lsp::InitializeResult {
      capabilities: Server::capabilities(),
      server_info: Some(lsp::ServerInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
      }),
    })
  }

  async fn initialized(&self, _: lsp::InitializedParams) {
    self
      .client
      .log_message(
        lsp::MessageType::INFO,
        &format!("{} initialized", env!("CARGO_PKG_NAME")),
      )
      .await;

    self.initialized.store(true, Ordering::Relaxed);
  }

  fn new(client: Client) -> Self {
    Self {
      client,
      documents: RwLock::new(BTreeMap::new()),
      initialized: AtomicBool::new(false),
    }
  }

  async fn publish_diagnostics(&self, uri: &lsp::Url) {
    if !self.initialized.load(Ordering::Relaxed) {
      return;
    }

    let documents = self.documents.read().await;

    if let Some(document) = documents.get(uri) {
      let analyzer = Analyzer::new(document);

      let diagnostics = analyzer
        .analyze()
        .into_iter()
        .map(Into::into)
        .collect::<Vec<lsp::Diagnostic>>();

      self
        .client
        .publish_diagnostics(uri.clone(), diagnostics, Some(document.version))
        .await;
    }
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    pretty_assertions::assert_eq,
    serde_json::{Value, json},
    std::env,
    tower_lsp::LspService,
    tower_test::mock::Spawn,
  };

  #[derive(Debug)]
  struct Test {
    requests: Vec<Value>,
    responses: Vec<Option<Value>>,
    service: Spawn<LspService<Server>>,
  }

  impl Test {
    fn new() -> Result<Self> {
      let (service, _) = LspService::new(Server::new);

      Ok(Self {
        requests: Vec::new(),
        responses: Vec::new(),
        service: Spawn::new(service),
      })
    }

    fn notification<T: IntoValue>(mut self, notification: T) -> Self {
      self.requests.push(notification.into_value());
      self.responses.push(None);
      self
    }

    fn request<T: IntoValue>(mut self, request: T) -> Self {
      self.requests.push(request.into_value());
      self
    }

    fn response<T: IntoValue>(mut self, response: T) -> Self {
      self.responses.push(Some(response.into_value()));
      self
    }

    async fn run(mut self) -> Result {
      for (request, expected_response) in
        self.requests.iter().zip(self.responses.iter())
      {
        let response = self
          .service
          .call(serde_json::from_value(request.clone())?)
          .await?;

        if let Some(expected) = expected_response {
          assert_eq!(
            *expected,
            response
              .map(|value| serde_json::to_value(value).unwrap())
              .unwrap()
          );
        } else {
          assert!(response.is_none(), "expected no response for notification");
        }
      }

      Ok(())
    }
  }

  trait IntoValue {
    fn into_value(self) -> Value;
  }

  impl IntoValue for Value {
    fn into_value(self) -> Value {
      self
    }
  }

  #[derive(Debug)]
  struct InitializeRequest {
    id: i64,
  }

  impl IntoValue for InitializeRequest {
    fn into_value(self) -> Value {
      json!({
        "jsonrpc": "2.0",
        "id": self.id,
        "method": "initialize",
        "params": {
          "capabilities": {}
        },
      })
    }
  }

  #[derive(Debug)]
  struct InitializeResponse {
    id: i64,
  }

  impl IntoValue for InitializeResponse {
    fn into_value(self) -> Value {
      json!({
        "jsonrpc": "2.0",
        "id": self.id,
        "result": {
          "serverInfo": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION")
          },
          "capabilities": Server::capabilities()
        },
      })
    }
  }

  #[derive(Debug)]
  struct DidOpenNotification<'a> {
    text: &'a str,
    uri: &'a str,
  }

  impl IntoValue for DidOpenNotification<'_> {
    fn into_value(self) -> Value {
      json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
          "textDocument": {
            "uri": self.uri,
            "languageId": "toml",
            "version": 1,
            "text": self.text
          }
        }
      })
    }
  }

  #[derive(Debug)]
  struct HoverRequest<'a> {
    character: u32,
    id: i64,
    line: u32,
    uri: &'a str,
  }

  impl IntoValue for HoverRequest<'_> {
    fn into_value(self) -> Value {
      json!({
        "jsonrpc": "2.0",
        "id": self.id,
        "method": "textDocument/hover",
        "params": {
          "textDocument": {
            "uri": self.uri
          },
          "position": {
            "line": self.line,
            "character": self.character
          }
        }
      })
    }
  }

  #[derive(Debug)]
  struct HoverResponse<'a> {
    content: &'a str,
    end_char: u32,
    end_line: u32,
    id: i64,
    kind: &'a str,
    start_char: u32,
    start_line: u32,
  }

  impl IntoValue for HoverResponse<'_> {
    fn into_value(self) -> Value {
      json!({
        "jsonrpc": "2.0",
        "id": self.id,
        "result": {
          "contents": {
            "kind": self.kind,
            "value": self.content
          },
          "range": {
            "start": {
              "line": self.start_line,
              "character": self.start_char
            },
            "end": {
              "line": self.end_line,
              "character": self.end_char
            }
          }
        }
      })
    }
  }

  #[tokio::test]
  async fn initialize() -> Result {
    Test::new()?
      .request(InitializeRequest { id: 1 })
      .response(InitializeResponse { id: 1 })
      .run()
      .await
  }

  #[tokio::test]
  async fn initialize_once() -> Result {
    Test::new()?
      .request(InitializeRequest { id: 1 })
      .response(InitializeResponse { id: 1 })
      .request(InitializeRequest { id: 1 })
      .response(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
          "code": -32600,
          "message": "Invalid request"
        }
      }))
      .run()
      .await
  }

  #[tokio::test]
  async fn shutdown() -> Result {
    Test::new()?
      .request(InitializeRequest { id: 1 })
      .response(InitializeResponse { id: 1 })
      .request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
      }))
      .response(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": null
      }))
      .run()
      .await
  }

  #[tokio::test]
  async fn hover_returns_schema_description() -> Result {
    let uri = "file:///pyproject.toml";

    Test::new()?
      .request(InitializeRequest { id: 1 })
      .response(InitializeResponse { id: 1 })
      .notification(DidOpenNotification {
        uri,
        text: indoc! {
          r#"[tool.poetry]
          name = "demo"
          "#
        },
      })
      .request(HoverRequest {
        id: 2,
        uri,
        line: 1,
        character: 1,
      })
      .response(HoverResponse {
        id: 2,
        content: "Package name.",
        kind: "markdown",
        start_line: 1,
        start_char: 0,
        end_line: 1,
        end_char: 13,
      })
      .run()
      .await
  }
}
