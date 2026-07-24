#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to convert document to JSON: {source}")]
  DocumentJson {
    #[source]
    source: serde_json::Error,
  },
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error("no releases found for `{package}`")]
  NoPyPiReleases { package: String },
  #[error("failed to parse PyPI response: {source}")]
  PyPiPayload {
    #[source]
    source: reqwest::Error,
  },
  #[error("PyPI request failed: {source}")]
  PyPiRequest {
    #[source]
    source: reqwest::Error,
  },
  #[error("PyPI returned an unsuccessful response: {source}")]
  PyPiResponse {
    #[source]
    source: reqwest::Error,
  },
  #[error("failed to compile bundled schemas: {error}")]
  SchemaCompile { error: String },
}
