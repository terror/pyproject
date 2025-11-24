use super::*;

#[cfg_attr(test, allow(dead_code))]
#[derive(Debug)]
pub(crate) enum PyPiError {
  Deserialize(ReqwestError),
  NoReleases(String),
  Request(ReqwestError),
  Status(ReqwestError),
}

impl fmt::Display for PyPiError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Deserialize(error) => {
        write!(f, "failed to parse response: {error}")
      }
      Self::NoReleases(package) => {
        write!(f, "no releases found for `{package}`")
      }
      Self::Request(error) => write!(f, "request failed: {error}"),
      Self::Status(error) => write!(f, "unexpected response: {error}"),
    }
  }
}

impl std::error::Error for PyPiError {}

#[derive(Debug, Deserialize)]
struct PyPiResponse {
  info: PackageInfo,
  releases: HashMap<String, Vec<ReleaseFile>>,
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
  version: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseFile {
  #[serde(default)]
  yanked: bool,
}

#[cfg(test)]
static MOCKED_VERSIONS: OnceLock<Mutex<HashMap<String, Option<Version>>>> =
  OnceLock::new();

#[cfg(test)]
fn mocked_latest_version(package: &str) -> Option<Version> {
  let Ok(versions) = MOCKED_VERSIONS
    .get_or_init(|| Mutex::new(HashMap::new()))
    .lock()
  else {
    return None;
  };

  versions.get(package).cloned().flatten()
}

#[cfg(test)]
pub(crate) fn set_mock_latest_version(package: &str, version: Option<&str>) {
  let version = version.map(|value| {
    Version::from_str(value).unwrap_or_else(|error| {
      panic!("invalid mocked version `{value}`: {error}")
    })
  });

  if let Ok(mut versions) = MOCKED_VERSIONS
    .get_or_init(|| Mutex::new(HashMap::new()))
    .lock()
  {
    versions.insert(package.to_string(), version);
  }
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) struct PyPiClient {
  base_url: String,
  cache: Mutex<HashMap<String, Version>>,
  http: ReqwestClient,
}

impl PyPiClient {
  fn fetch_latest_version(&self, url: &str) -> Result<Version, PyPiError> {
    let response = self.http.get(url).send().map_err(PyPiError::Request)?;

    let response = response.error_for_status().map_err(PyPiError::Status)?;

    let payload: PyPiResponse =
      response.json().map_err(PyPiError::Deserialize)?;

    Self::select_latest_version(payload)
  }

  #[cfg_attr(test, allow(clippy::unused_self))]
  pub(crate) fn latest_version(
    &self,
    package: &PackageName,
  ) -> Option<Version> {
    self.latest_version_result(package).ok()
  }

  #[cfg_attr(test, allow(clippy::unused_self))]
  #[cfg_attr(test, allow(unreachable_code))]
  pub(crate) fn latest_version_result(
    &self,
    package: &PackageName,
  ) -> Result<Version, PyPiError> {
    let name = package.to_string();

    #[cfg(test)]
    {
      // Tests rely on deterministic mocks and should not hit the network.
      if let Some(mocked) = mocked_latest_version(&name) {
        return Ok(mocked);
      }

      return Err(PyPiError::NoReleases(name));
    }

    let cache_key = format!("{}/{}", self.base_url, name);

    match self.cache.lock() {
      Ok(cache) => {
        if let Some(version) = cache.get(&cache_key) {
          return Ok(version.clone());
        }
      }
      Err(error) => {
        debug!("failed to lock PyPI cache: {error}");
      }
    }

    let url = format!("{}/pypi/{}/json", self.base_url, name);

    let latest = self.fetch_latest_version(&url)?;

    if let Ok(mut cache) = self.cache.lock() {
      cache.insert(cache_key, latest.clone());
    } else {
      debug!("failed to lock PyPI cache for insert");
    }

    Ok(latest)
  }

  fn new() -> Self {
    let base_url = env::var("PYPROJECT_PYPI_BASE_URL")
      .unwrap_or_else(|_| "https://pypi.org".to_string())
      .trim_end_matches('/')
      .to_string();

    let http = ReqwestClient::builder()
      .timeout(Duration::from_secs(5))
      .user_agent(format!(
        "{}/{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
      ))
      .build()
      .unwrap_or_else(|error| {
        debug!("failed to configure HTTP client: {error}");
        ReqwestClient::new()
      });

    Self {
      base_url,
      cache: Mutex::new(HashMap::new()),
      http,
    }
  }

  fn select_latest_version(
    payload: PyPiResponse,
  ) -> Result<Version, PyPiError> {
    let mut latest_release = None;
    let mut latest_prerelease = None;

    for (raw_version, files) in payload.releases {
      if files.iter().all(|file| file.yanked) {
        continue;
      }

      let Ok(version) = Version::from_str(&raw_version) else {
        continue;
      };

      if version.any_prerelease() {
        if latest_prerelease
          .as_ref()
          .is_none_or(|current| version > *current)
        {
          latest_prerelease = Some(version);
        }
      } else if latest_release
        .as_ref()
        .is_none_or(|current| version > *current)
      {
        latest_release = Some(version);
      }
    }

    if let Some(version) = latest_release.or(latest_prerelease) {
      return Ok(version);
    }

    Version::from_str(&payload.info.version)
      .map_err(|_| PyPiError::NoReleases(payload.info.version))
  }

  pub(crate) fn shared() -> &'static Self {
    static INSTANCE: OnceLock<PyPiClient> = OnceLock::new();

    INSTANCE.get_or_init(Self::new)
  }
}
