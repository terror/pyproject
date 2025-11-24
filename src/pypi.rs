use {
  log::debug,
  pep440_rs::Version,
  pep508_rs::PackageName,
  reqwest::blocking::Client,
  std::{
    collections::HashMap,
    env,
    str::FromStr,
    sync::{Mutex, OnceLock},
    time::Duration,
  },
};

#[cfg(not(test))]
use serde::Deserialize;

#[cfg_attr(test, allow(dead_code))]
pub(crate) struct PyPiClient {
  #[cfg_attr(test, allow(dead_code))]
  cache: Mutex<HashMap<String, Option<Version>>>,
  #[cfg_attr(test, allow(dead_code))]
  http: Client,
}

impl PyPiClient {
  #[cfg(not(test))]
  fn fetch_latest_version(&self, url: &str) -> Option<Version> {
    let response = match self.http.get(url).send() {
      Ok(response) => response,
      Err(error) => {
        debug!("failed to request {url}: {error}");
        return None;
      }
    };

    let response = match response.error_for_status() {
      Ok(response) => response,
      Err(error) => {
        debug!("received error from {url}: {error}");
        return None;
      }
    };

    let payload: PyPiResponse = match response.json() {
      Ok(payload) => payload,
      Err(error) => {
        debug!("failed to deserialize response from {url}: {error}");
        return None;
      }
    };

    Self::select_latest_version(payload)
  }

  #[cfg_attr(test, allow(clippy::unused_self))]
  pub(crate) fn latest_version(
    &self,
    package: &PackageName,
  ) -> Option<Version> {
    let name = package.to_string();

    #[cfg(test)]
    {
      if let Some(mocked) = mocked_latest_version(&name) {
        return Some(mocked);
      }

      None
    }

    #[cfg(not(test))]
    {
      let base_url = env::var("PYPROJECT_PYPI_BASE_URL")
        .unwrap_or_else(|_| "https://pypi.org".to_string())
        .trim_end_matches('/')
        .to_string();

      let cache_key = format!("{base_url}/{name}");

      if let Ok(cache) = self.cache.lock()
        && let Some(version) = cache.get(&cache_key)
      {
        return version.clone();
      }

      let url = format!("{base_url}/pypi/{name}/json");

      let latest = self.fetch_latest_version(&url);

      if let Ok(mut cache) = self.cache.lock() {
        cache.insert(cache_key, latest.clone());
      }

      latest
    }
  }

  fn new() -> Self {
    Self {
      cache: Mutex::new(HashMap::new()),
      http: Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(format!(
          "{}/{}",
          env!("CARGO_PKG_NAME"),
          env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap_or_else(|error| {
          debug!("failed to configure HTTP client: {error}");
          Client::new()
        }),
    }
  }

  #[cfg(not(test))]
  fn select_latest_version(payload: PyPiResponse) -> Option<Version> {
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

    latest_release
      .or(latest_prerelease)
      .or_else(|| Version::from_str(&payload.info.version).ok())
  }

  pub(crate) fn shared() -> &'static Self {
    static INSTANCE: OnceLock<PyPiClient> = OnceLock::new();

    INSTANCE.get_or_init(Self::new)
  }
}

#[cfg(not(test))]
#[derive(Debug, Deserialize)]
struct PyPiResponse {
  info: PackageInfo,
  releases: HashMap<String, Vec<ReleaseFile>>,
}

#[cfg(not(test))]
#[derive(Debug, Deserialize)]
struct PackageInfo {
  version: String,
}

#[cfg(not(test))]
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
