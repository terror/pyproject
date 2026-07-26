use super::*;

#[derive(Debug)]
pub struct Server(Arc<Inner>);

impl Server {
  #[must_use]
  pub fn capabilities() -> lsp::ServerCapabilities {
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
      code_action_provider: Some(lsp::CodeActionProviderCapability::Simple(
        true,
      )),
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

  #[must_use]
  pub fn new(client: Client) -> Self {
    Self(Arc::new(Inner::new(client)))
  }

  pub async fn run() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(Server::new);

    tower_lsp::Server::new(stdin, stdout, socket)
      .serve(service)
      .await;
  }
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
  async fn code_action(
    &self,
    params: lsp::CodeActionParams,
  ) -> Result<Option<lsp::CodeActionResponse>, jsonrpc::Error> {
    self.0.code_action(params).await
  }

  async fn completion(
    &self,
    params: lsp::CompletionParams,
  ) -> Result<Option<lsp::CompletionResponse>, jsonrpc::Error> {
    self.0.completion(params).await
  }

  async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
    if let Err(error) = self.0.did_change(params).await {
      tracing::error!(%error, "failed to update document");

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
      tracing::error!(%error, "failed to open document");

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
  async fn code_action(
    &self,
    params: lsp::CodeActionParams,
  ) -> Result<Option<lsp::CodeActionResponse>, jsonrpc::Error> {
    let documents = self.documents.read().await;

    let Some(document) = documents.get(&params.text_document.uri) else {
      return Ok(None);
    };

    Ok(Some(
      Quickfixer::new(&params, &document.diagnostics).collect(),
    ))
  }

  async fn completion(
    &self,
    params: lsp::CompletionParams,
  ) -> Result<Option<lsp::CompletionResponse>, jsonrpc::Error> {
    let started = Instant::now();

    let uri = params.text_document_position.text_document.uri;

    if !self.documents.read().await.contains_key(&uri) {
      return Ok(None);
    }

    let mut items = BUILTINS
      .iter()
      .map(|builtin| builtin.completion_item())
      .chain(
        include_str!("rule/classifiers.txt")
          .lines()
          .map(str::trim)
          .filter(|classifier| !classifier.is_empty())
          .map(|classifier| {
            Builtin::Value {
              name: classifier,
              description: "Trove classifier",
            }
            .completion_item()
          }),
      )
      .collect::<Vec<_>>();

    for schema in SCHEMAS {
      let schema_name = schema.tool.unwrap_or("pyproject");

      if let Some(tool) = schema.tool {
        let name = format!("tool.{tool}");

        let description = format!("{tool} configuration");

        items.push(
          Builtin::Table {
            name: &name,
            description: &description,
          }
          .completion_item(),
        );
      }

      let document = serde_json::from_str::<Value>(schema.contents)
        .map_err(|_| jsonrpc::Error::internal_error())?;

      let mut stack = vec![(&document, None)];

      while let Some((value, inherited_description)) = stack.pop() {
        let description = value
          .get("description")
          .and_then(Value::as_str)
          .or_else(|| value.get("title").and_then(Value::as_str))
          .or(inherited_description);

        if let Some(properties) =
          value.get("properties").and_then(Value::as_object)
        {
          for (name, property) in properties {
            let type_name = match property.get("type") {
              Some(Value::String(type_name)) => type_name.clone(),
              Some(Value::Array(types)) => types
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" | "),
              _ => [
                ("enum", "enum"),
                ("const", "constant"),
                ("oneOf", "variant"),
                ("anyOf", "variant"),
                ("$ref", "schema"),
              ]
              .into_iter()
              .find_map(|(key, type_name)| {
                property.get(key).is_some().then_some(type_name)
              })
              .unwrap_or("unknown")
              .into(),
            };

            let property_description = property
              .get("description")
              .and_then(Value::as_str)
              .or_else(|| property.get("title").and_then(Value::as_str))
              .or(description)
              .unwrap_or_default();

            let deprecated = property
              .get("deprecated")
              .and_then(Value::as_bool)
              .unwrap_or_default();

            let requires_quotes = name.chars().any(|character| {
              !character.is_ascii_alphanumeric()
                && !matches!(character, '-' | '_')
            });

            items.push(lsp::CompletionItem {
              detail: Some(format!("{type_name} ({schema_name})")),
              tags: deprecated
                .then_some(vec![lsp::CompletionItemTag::DEPRECATED]),
              insert_text: requires_quotes
                .then(|| serde_json::to_string(name))
                .and_then(Result::ok),
              ..Builtin::Key {
                name,
                type_name: &type_name,
                description: property_description,
              }
              .completion_item()
            });
          }
        }

        let values = value
          .get("enum")
          .and_then(Value::as_array)
          .into_iter()
          .flatten()
          .chain(value.get("const"))
          .chain(value.get("default"))
          .chain(
            value
              .get("examples")
              .and_then(Value::as_array)
              .into_iter()
              .flatten(),
          );

        for value in values {
          let label = match value {
            Value::String(value) => value.clone(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            _ => continue,
          };

          items.push(lsp::CompletionItem {
            detail: Some(format!("{schema_name} value")),
            insert_text: serde_json::to_string(value).ok(),
            ..Builtin::Value {
              name: &label,
              description: description.unwrap_or_default(),
            }
            .completion_item()
          });
        }

        match value {
          Value::Array(values) => {
            stack.extend(values.iter().map(|value| (value, description)));
          }
          Value::Object(object) => {
            stack.extend(object.values().map(|value| (value, description)));
          }
          _ => {}
        }
      }
    }

    let mut seen = HashSet::new();

    items.retain(|item| {
      seen.insert((
        item.label.clone(),
        format!("{:?}", item.kind),
        item.insert_text.clone().unwrap_or_default(),
      ))
    });

    tracing::debug!(
      uri = %uri,
      completion_count = items.len(),
      elapsed = ?started.elapsed(),
      "computed completions"
    );

    Ok(Some(lsp::CompletionResponse::Array(items)))
  }

  async fn did_change(
    &self,
    params: lsp::DidChangeTextDocumentParams,
  ) -> Result {
    let started = Instant::now();
    let uri = params.text_document.uri.clone();
    let version = params.text_document.version;
    let change_count = params.content_changes.len();

    let mut documents = self.documents.write().await;

    let Some(document) = documents.get_mut(&uri) else {
      return Ok(());
    };

    document.apply_change(params);

    document.analyze();

    let document_bytes = document.content.len_bytes();
    let diagnostic_count = document.diagnostics.len();

    drop(documents);

    tracing::debug!(
      uri = %uri,
      version,
      change_count,
      document_bytes,
      diagnostic_count,
      elapsed = ?started.elapsed(),
      "updated document"
    );

    self.publish_diagnostics(&uri).await;

    Ok(())
  }

  async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
    let uri = params.text_document.uri.clone();

    let removed = {
      let mut documents = self.documents.write().await;
      documents.remove(&uri).is_some()
    };

    tracing::debug!(uri = %uri, removed, "closed document");

    if removed {
      self.client.publish_diagnostics(uri, vec![], None).await;
    }
  }

  async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) -> Result {
    let started = Instant::now();
    let uri = params.text_document.uri.clone();

    let mut document = Document::from(params);

    document.analyze();

    let version = document.version;
    let document_bytes = document.content.len_bytes();
    let diagnostic_count = document.diagnostics.len();

    self.documents.write().await.insert(uri.clone(), document);

    tracing::debug!(
      uri = %uri,
      version,
      document_bytes,
      diagnostic_count,
      elapsed = ?started.elapsed(),
      "opened document"
    );

    self.publish_diagnostics(&uri).await;

    Ok(())
  }

  async fn formatting(
    &self,
    params: lsp::DocumentFormattingParams,
  ) -> Result<Option<Vec<lsp::TextEdit>>, jsonrpc::Error> {
    let started = Instant::now();
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
      tracing::debug!(
        uri = %uri,
        changed = false,
        document_bytes = original.len(),
        elapsed = ?started.elapsed(),
        "formatted document"
      );

      return Ok(Some(vec![]));
    }

    let edit = lsp::TextEdit {
      range: lsp::Range::new(lsp::Position::new(0, 0), end),
      new_text: formatted,
    };

    tracing::debug!(
      uri = %uri,
      changed = true,
      document_bytes = original.len(),
      elapsed = ?started.elapsed(),
      "formatted document"
    );

    Ok(Some(vec![edit]))
  }

  async fn hover(
    &self,
    params: lsp::HoverParams,
  ) -> Result<Option<lsp::Hover>, jsonrpc::Error> {
    let started = Instant::now();
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

    let hover = Resolver::new(document).resolve_hover(position);

    tracing::debug!(
      uri = %text_document.uri,
      line = position.line,
      character = position.character,
      found = hover.is_some(),
      elapsed = ?started.elapsed(),
      "resolved hover"
    );

    Ok(hover)
  }

  #[allow(clippy::unused_async)]
  async fn initialize(
    &self,
    _params: lsp::InitializeParams,
  ) -> Result<lsp::InitializeResult, jsonrpc::Error> {
    tracing::info!("Starting pyproject language server...");

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
    let started = Instant::now();

    if !self.initialized.load(Ordering::Relaxed) {
      return;
    }

    let (diagnostics, version) = {
      let documents = self.documents.read().await;

      let Some(document) = documents.get(uri) else {
        return;
      };

      let diagnostics = document
        .diagnostics
        .iter()
        .map(lsp::Diagnostic::from)
        .collect::<Vec<lsp::Diagnostic>>();

      (diagnostics, document.version)
    };

    let diagnostic_count = diagnostics.len();

    self
      .client
      .publish_diagnostics(uri.clone(), diagnostics, Some(version))
      .await;

    tracing::debug!(
      uri = %uri,
      version,
      diagnostic_count,
      elapsed = ?started.elapsed(),
      "published diagnostics"
    );
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    indoc::indoc,
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
  async fn code_action_replaces_non_normalized_project_name() -> Result {
    let uri = "file:///pyproject.toml";

    Test::new()?
      .request(InitializeRequest { id: 1 })
      .response(InitializeResponse { id: 1 })
      .notification(DidOpenNotification {
        uri,
        text: indoc! {
          r#"[project]
          name = "my-package"
          version = "1.0.0"

          [tool.pyproject.rules]
          project-name-normalization = "warning"
          "#
        },
      })
      .notification(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
          "textDocument": {
            "uri": uri,
            "version": 2
          },
          "contentChanges": [{
            "range": {
              "start": { "line": 1, "character": 7 },
              "end": { "line": 1, "character": 19 }
            },
            "text": "\"My_Package\""
          }]
        }
      }))
      .request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
          "textDocument": { "uri": uri },
          "range": {
            "start": { "line": 1, "character": 8 },
            "end": { "line": 1, "character": 18 }
          },
          "context": { "diagnostics": [] }
        }
      }))
      .response(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": [{
          "title": "Replace `My_Package` with `my-package`",
          "kind": "quickfix",
          "edit": {
            "changes": {
              uri: [{
                "range": {
                  "start": { "line": 1, "character": 8 },
                  "end": { "line": 1, "character": 18 }
                },
                "newText": "my-package"
              }]
            }
          }
        }]
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
