use super::*;

pub(crate) struct SchemaStore;

impl SchemaStore {
  pub(crate) fn builtin_validator() -> Result<Validator> {
    let store = Self::new();

    jsonschema::options()
      .with_retriever(store)
      .build(Self::root())
      .map_err(Error::new)
  }

  fn client() -> &'static ReqwestClient {
    static CLIENT: OnceLock<ReqwestClient> = OnceLock::new();

    CLIENT.get_or_init(|| {
      ReqwestClient::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(format!(
          "{}/{}",
          env!("CARGO_PKG_NAME"),
          env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap_or_else(|_| ReqwestClient::new())
    })
  }

  pub(crate) fn documents() -> &'static HashMap<&'static str, Value> {
    static DOCUMENTS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();

    DOCUMENTS.get_or_init(|| {
      SCHEMAS
        .iter()
        .map(|schema| (schema.url, Self::parse_schema(schema)))
        .collect()
    })
  }

  fn load(uri: &str) -> Result<Value> {
    let uri = Self::without_fragment(uri)?;

    let url = lsp::Url::parse(&uri)
      .map_err(|error| anyhow!("invalid schema URL `{uri}`: {error}"))?;

    let contents = match url.scheme() {
      "file" => {
        let path = url
          .to_file_path()
          .map_err(|()| anyhow!("invalid schema file URL `{uri}`"))?;

        fs::read_to_string(path)?
      }
      "https" => Self::client()
        .get(&uri)
        .send()
        .map_err(|error| anyhow!("failed to fetch schema `{uri}`: {error}"))?
        .error_for_status()
        .map_err(|error| anyhow!("failed to fetch schema `{uri}`: {error}"))?
        .text()
        .map_err(|error| anyhow!("failed to read schema `{uri}`: {error}"))?,
      scheme => bail!("unsupported schema URL scheme `{scheme}`"),
    };

    serde_json::from_str::<Value>(&contents)
      .map_err(|error| anyhow!("failed to parse schema `{uri}`: {error}"))
  }

  fn new() -> Self {
    Self
  }

  fn parse_schema(schema: &Schema) -> Value {
    serde_json::from_str(schema.contents).unwrap_or_else(|error| {
      panic!("failed to parse bundled schema {}: {error}", schema.url)
    })
  }

  pub(crate) fn root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();

    ROOT.get_or_init(|| Self::root_with(Self::tool_properties()))
  }

  fn root_for(config: &Config) -> Value {
    let mut tool_properties = Self::tool_properties();

    for (tool, url) in &config.schemas {
      tool_properties.insert(tool.clone(), json!({ "$ref": url }));
    }

    Self::root_with(tool_properties)
  }

  fn root_with(tool_properties: Map<String, Value>) -> Value {
    json!({
      "$schema": "http://json-schema.org/draft-07/schema#",
      "type": "object",
      "additionalProperties": true,
      "properties": {
        "tool": {
          "type": "object",
          "additionalProperties": true,
          "properties": tool_properties,
        }
      }
    })
  }

  fn tool_properties() -> Map<String, Value> {
    SCHEMAS
      .iter()
      .filter_map(|schema| schema.tool.map(|tool| (tool, schema.url)))
      .map(|(tool, url)| (tool.to_string(), json!({ "$ref": url })))
      .collect()
  }

  pub(crate) fn validator(config: &Config) -> Result<Validator> {
    let store = Self::new();

    let root = Self::root_for(config);

    jsonschema::options()
      .with_retriever(store)
      .build(&root)
      .map_err(Error::new)
  }

  fn without_fragment(uri: &str) -> Result<String> {
    let mut url = lsp::Url::parse(uri)
      .map_err(|error| anyhow!("invalid schema URL `{uri}`: {error}"))?;

    url.set_fragment(None);

    Ok(url.to_string())
  }
}

impl Retrieve for SchemaStore {
  fn retrieve(
    &self,
    uri: &Uri<String>,
  ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let uri = Self::without_fragment(uri.as_str())
      .map_err(|error| error.to_string())?;

    Self::documents().get(uri.as_str()).cloned().map_or_else(
      || Self::load(&uri).map_err(|error| error.to_string().into()),
      Ok,
    )
  }
}

#[cfg(test)]
mod tests {
  use {super::*, tempfile::TempDir};

  fn file_url(path: &Path) -> String {
    lsp::Url::from_file_path(path).unwrap().to_string()
  }

  fn write_schema(tempdir: &TempDir, path: &str, properties: Value) -> String {
    let path = tempdir.path().join(path);
    let url = file_url(&path);

    fs::write(
      &path,
      json!({
        "$id": url,
        "type": "object",
        "additionalProperties": false,
        "properties": properties
      })
      .to_string(),
    )
    .unwrap();

    url
  }

  #[test]
  fn loads_schema_for_configured_tool() {
    let tempdir = TempDir::new().unwrap();
    let url = write_schema(
      &tempdir,
      "foo.json",
      json!({
        "enabled": { "type": "boolean" }
      }),
    );
    let mut config = Config::default();

    config.add_schema(&format!("foo={url}")).unwrap();

    let validator = SchemaStore::validator(&config).unwrap();

    assert!(
      validator.is_valid(&json!({ "tool": { "foo": { "enabled": true } } }))
    );
    assert!(
      !validator.is_valid(&json!({ "tool": { "foo": { "unknown": true } } }))
    );
  }

  #[test]
  fn loads_transitive_schema_references() {
    let tempdir = TempDir::new().unwrap();
    let child = write_schema(
      &tempdir,
      "child.json",
      json!({
        "enabled": { "type": "boolean" }
      }),
    );
    let parent = write_schema(
      &tempdir,
      "parent.json",
      json!({
        "configuration": { "$ref": child }
      }),
    );
    let mut config = Config::default();

    config.add_schema(&format!("foo={parent}")).unwrap();

    let validator = SchemaStore::validator(&config).unwrap();

    assert!(validator.is_valid(&json!({
      "tool": { "foo": { "configuration": { "enabled": true } } }
    })));
    assert!(!validator.is_valid(&json!({
      "tool": { "foo": { "configuration": { "enabled": "foo" } } }
    })));
  }

  #[test]
  fn rejects_http_schema_urls() {
    assert_eq!(
      SchemaStore::load("http://example.com/foo.json")
        .unwrap_err()
        .to_string(),
      "unsupported schema URL scheme `http`"
    );
  }
}
