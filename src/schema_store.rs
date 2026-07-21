use super::*;

pub(crate) struct SchemaStore;

impl SchemaStore {
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

  fn load(uri: &str) -> Result<Value> {
    let uri = Self::without_fragment(uri)?;

    let url = lsp::Url::parse(&uri)?;

    let contents = match url.scheme() {
      "file" => fs::read_to_string(
        url
          .to_file_path()
          .map_err(|_| anyhow!("invalid schema file URL `{uri}`"))?,
      )?,
      "https" => Self::client()
        .get(&uri)
        .send()?
        .error_for_status()?
        .text()?,
      scheme => bail!("unsupported schema URL scheme `{scheme}`"),
    };

    serde_json::from_str::<Value>(&contents)
      .map_err(|error| anyhow!("failed to parse schema `{uri}`: {error}"))
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
      .map_err(Error::into_boxed_dyn_error)?;

    if let Some(schema) = SCHEMAS.iter().find(|schema| schema.url == uri) {
      return serde_json::from_str(schema.contents).map_err(|error| {
        anyhow!("failed to parse bundled schema {}: {error}", schema.url)
          .into_boxed_dyn_error()
      });
    }

    Self::load(&uri).map_err(Error::into_boxed_dyn_error)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
