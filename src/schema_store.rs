use super::*;

pub(crate) struct SchemaStore {
  documents: HashMap<String, Value>,
}

#[derive(Debug, Default)]
pub(crate) struct SchemaSources {
  pub(crate) plugins: Vec<String>,
  pub(crate) stores: Vec<String>,
  tools: Vec<String>,
}

impl SchemaSources {
  pub(crate) fn add_tool(&mut self, specification: &str) {
    self.tools.push(specification.to_string());
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.plugins.is_empty() && self.stores.is_empty() && self.tools.is_empty()
  }
}

impl From<&Config> for SchemaSources {
  fn from(config: &Config) -> Self {
    Self {
      plugins: config.schema.plugin.clone(),
      stores: config.schema.store.clone(),
      tools: config.schema.tool.clone(),
    }
  }
}

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
    static CACHE: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();

    let uri = Self::without_fragment(uri)?;

    if let Ok(cache) = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock()
      && let Some(schema) = cache.get(&uri)
    {
      return Ok(schema.clone());
    }

    let url = lsp::Url::parse(&uri)
      .map_err(|error| anyhow!("invalid schema URL `{uri}`: {error}"))?;

    let contents = match url.scheme() {
      "file" => {
        let path = url
          .to_file_path()
          .map_err(|()| anyhow!("invalid schema file URL `{uri}`"))?;

        fs::read_to_string(path)?
      }
      "http" | "https" => Self::client()
        .get(&uri)
        .send()
        .map_err(|error| anyhow!("failed to fetch schema `{uri}`: {error}"))?
        .error_for_status()
        .map_err(|error| anyhow!("failed to fetch schema `{uri}`: {error}"))?
        .text()
        .map_err(|error| anyhow!("failed to read schema `{uri}`: {error}"))?,
      scheme => bail!("unsupported schema URL scheme `{scheme}`"),
    };

    let schema = serde_json::from_str::<Value>(&contents)
      .map_err(|error| anyhow!("failed to parse schema `{uri}`: {error}"))?;

    if let Ok(mut cache) =
      CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock()
    {
      cache.insert(uri, schema.clone());
    }

    Ok(schema)
  }

  fn load_plugin(
    &mut self,
    url: &str,
    tool_properties: &mut Map<String, Value>,
  ) -> Result {
    let plugin = Self::load(url)?;
    let Some(tools) = plugin.get("tools").and_then(Value::as_object) else {
      bail!("schema plugin `{url}` does not define `tools`");
    };

    let schemas = plugin
      .get("schemas")
      .and_then(Value::as_array)
      .map(|schemas| {
        schemas
          .iter()
          .map(|schema| {
            schema.as_str().map(str::to_string).ok_or_else(|| {
              anyhow!("schema plugin `{url}` has a non-string schema URL")
            })
          })
          .collect::<Result<Vec<_>>>()
      })
      .transpose()?
      .unwrap_or_default();

    for schema in schemas {
      let schema = Self::resolve_reference(url, &schema)?;
      self.register_schema(&schema)?;
    }

    for (tool, schema) in tools {
      let Some(schema) = schema.as_str() else {
        bail!("schema plugin `{url}` has a non-string tool schema URL");
      };

      let schema = Self::resolve_reference(url, schema)?;

      tool_properties.insert(tool.clone(), json!({ "$ref": schema }));
    }

    Ok(())
  }

  fn new() -> Self {
    Self {
      documents: Self::documents()
        .iter()
        .map(|(url, schema)| ((*url).to_string(), schema.clone()))
        .collect(),
    }
  }

  fn parse_schema(schema: &Schema) -> Value {
    serde_json::from_str(schema.contents).unwrap_or_else(|error| {
      panic!("failed to parse bundled schema {}: {error}", schema.url)
    })
  }

  fn register_schema(&mut self, url: &str) -> Result {
    let schema = Self::load(url)?;
    let url = Self::without_fragment(url)?;

    self.documents.insert(url, schema.clone());

    if let Some(id) = schema.get("$id").and_then(Value::as_str) {
      self.documents.insert(id.to_string(), schema);
    }

    Ok(())
  }

  fn resolve_reference(base: &str, reference: &str) -> Result<String> {
    let base = lsp::Url::parse(base)
      .map_err(|error| anyhow!("invalid schema store URL `{base}`: {error}"))?;

    base
      .join(reference)
      .map(|url| url.to_string())
      .map_err(|error| {
        anyhow!("invalid schema reference `{reference}`: {error}")
      })
  }

  pub(crate) fn root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();

    ROOT.get_or_init(|| Self::root_with(Self::tool_properties()))
  }

  fn root_for(&mut self, sources: &SchemaSources) -> Result<Value> {
    let mut tool_properties = Self::tool_properties();

    for url in &sources.stores {
      let schema = Self::load(url)?;
      let Some(properties) = schema
        .pointer("/properties/tool/properties")
        .and_then(Value::as_object)
      else {
        bail!(
          "schema store `{url}` does not define `properties.tool.properties`"
        );
      };

      for (tool, schema) in properties {
        let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
          continue;
        };

        let reference = Self::resolve_reference(url, reference)?;

        tool_properties.insert(tool.clone(), json!({ "$ref": reference }));
      }
    }

    for url in &sources.plugins {
      self.load_plugin(url, &mut tool_properties)?;
    }

    for specification in &sources.tools {
      let Some((tool, url)) = specification.split_once('=') else {
        bail!("tool schema must use the form `TOOL=URL`");
      };

      if tool.is_empty() || url.is_empty() {
        bail!("tool schema must use the form `TOOL=URL`");
      }

      tool_properties.insert(tool.to_string(), json!({ "$ref": url }));
    }

    Ok(Self::root_with(tool_properties))
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

  pub(crate) fn validator(sources: &SchemaSources) -> Result<Validator> {
    let mut store = Self::new();

    let root = store.root_for(sources)?;

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

    self.documents.get(&uri).cloned().map_or_else(
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

  fn write_schema(tempdir: &TempDir, path: &str) -> String {
    let path = tempdir.path().join(path);
    let url = file_url(&path);

    fs::write(
      &path,
      json!({
        "$id": url,
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "enabled": { "type": "boolean" }
        }
      })
      .to_string(),
    )
    .unwrap();

    url
  }

  #[test]
  fn loads_schema_for_configured_tool() {
    let tempdir = TempDir::new().unwrap();
    let url = write_schema(&tempdir, "foo.json");
    let mut sources = SchemaSources::default();

    sources.add_tool(&format!("foo={url}"));

    let validator = SchemaStore::validator(&sources).unwrap();

    assert!(
      validator.is_valid(&json!({ "tool": { "foo": { "enabled": true } } }))
    );
    assert!(
      !validator.is_valid(&json!({ "tool": { "foo": { "unknown": true } } }))
    );
  }

  #[test]
  fn loads_schemas_from_plugin() {
    let tempdir = TempDir::new().unwrap();
    write_schema(&tempdir, "foo.json");
    let plugin_path = tempdir.path().join("plugin.json");
    let plugin_url = file_url(&plugin_path);

    fs::write(
      &plugin_path,
      json!({
        "tools": {
          "foo": "foo.json"
        }
      })
      .to_string(),
    )
    .unwrap();

    let sources = SchemaSources {
      plugins: vec![plugin_url],
      ..Default::default()
    };
    let validator = SchemaStore::validator(&sources).unwrap();

    assert!(
      validator.is_valid(&json!({ "tool": { "foo": { "enabled": true } } }))
    );
    assert!(
      !validator.is_valid(&json!({ "tool": { "foo": { "unknown": true } } }))
    );
  }

  #[test]
  fn loads_schemas_from_store() {
    let tempdir = TempDir::new().unwrap();
    let schema_url = write_schema(&tempdir, "foo.json");
    let store_path = tempdir.path().join("store.json");
    let store_url = file_url(&store_path);

    fs::write(
      &store_path,
      json!({
        "properties": {
          "tool": {
            "properties": {
              "foo": { "$ref": schema_url }
            }
          }
        }
      })
      .to_string(),
    )
    .unwrap();

    let sources = SchemaSources {
      stores: vec![store_url],
      ..Default::default()
    };
    let validator = SchemaStore::validator(&sources).unwrap();

    assert!(
      validator.is_valid(&json!({ "tool": { "foo": { "enabled": true } } }))
    );
    assert!(
      !validator.is_valid(&json!({ "tool": { "foo": { "unknown": true } } }))
    );
  }
}
