use super::*;

pub(crate) struct SchemaStore;

impl SchemaStore {
  fn client() -> &'static ReqwestClient {
    static CLIENT: OnceLock<ReqwestClient> = OnceLock::new();

    CLIENT.get_or_init(|| {
      ReqwestClient::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(concat!(
          env!("CARGO_PKG_NAME"),
          "/",
          env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("schema HTTP client configuration should be valid")
    })
  }

  fn load(url: &lsp::Url) -> Result<Value> {
    serde_json::from_str(&match url.scheme() {
      "file" => {
        let path = url
          .to_file_path()
          .map_err(|()| anyhow!("invalid schema file URL `{url}`"))?;

        fs::read_to_string(&path).with_context(|| {
          format!("failed to read schema `{}`", path.display())
        })?
      }
      "https" => Self::client()
        .get(url.as_str())
        .send()
        .and_then(Response::error_for_status)
        .and_then(Response::text)
        .with_context(|| format!("failed to download schema `{url}`"))?,
      scheme => bail!("unsupported schema URL scheme `{scheme}`"),
    })
    .with_context(|| format!("failed to parse schema `{url}`"))
  }
}

impl Retrieve for SchemaStore {
  fn retrieve(
    &self,
    uri: &Uri<String>,
  ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = lsp::Url::parse(uri.as_str())
      .with_context(|| format!("invalid schema URI `{uri}`"))
      .map_err(Error::into_boxed_dyn_error)?;

    url.set_fragment(None);

    let Some(schema) = SCHEMAS.iter().find(|schema| schema.url == url.as_str())
    else {
      return Self::load(&url).map_err(Error::into_boxed_dyn_error);
    };

    serde_json::from_str(schema.contents)
      .with_context(|| {
        format!("failed to parse bundled schema `{}`", schema.url)
      })
      .map_err(Error::into_boxed_dyn_error)
  }
}
