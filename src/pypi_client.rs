use super::*;

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

#[cfg_attr(test, allow(dead_code))]
pub(crate) struct PyPiClient {
  base_url: String,
  cache: Mutex<HashMap<String, Version>>,
  http: ReqwestClient,
}

impl PyPiClient {
  fn fetch_latest_version(
    &self,
    package: &PackageName,
    url: &str,
  ) -> Result<Version> {
    let response = self
      .http
      .get(url)
      .send()
      .map_err(|source| Error::PyPiRequest { source })?;

    let response = response
      .error_for_status()
      .map_err(|source| Error::PyPiResponse { source })?;

    let payload = response
      .json::<PyPiResponse>()
      .map_err(|source| Error::PyPiPayload { source })?;

    Self::select_latest_version(package, payload)
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
  ) -> Result<Version> {
    let name = package.to_string();

    #[cfg(test)]
    {
      if let Some(mocked) = mocked_latest_version(&name) {
        return Ok(mocked);
      }

      return Err(Error::NoPyPiReleases { package: name });
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

    let latest = self.fetch_latest_version(package, &url)?;

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
    package: &PackageName,
    payload: PyPiResponse,
  ) -> Result<Version> {
    let max_version = |current: Option<Version>, candidate: Version| {
      Some(match current {
        Some(version) => version.max(candidate),
        None => candidate,
      })
    };

    let (latest_release, latest_prerelease) = payload
      .releases
      .into_iter()
      .filter(|(_, files)| files.iter().any(|file| !file.yanked))
      .filter_map(|(raw_version, _)| Version::from_str(&raw_version).ok())
      .fold((None, None), |(release, prerelease), version| {
        if version.any_prerelease() {
          (release, max_version(prerelease, version))
        } else {
          (max_version(release, version), prerelease)
        }
      });

    latest_release
      .or(latest_prerelease)
      .or_else(|| Version::from_str(&payload.info.version).ok())
      .ok_or_else(|| Error::NoPyPiReleases {
        package: package.to_string(),
      })
  }

  pub(crate) fn shared() -> &'static Self {
    static INSTANCE: OnceLock<PyPiClient> = OnceLock::new();

    INSTANCE.get_or_init(Self::new)
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn falls_back_to_info_version() {
    let version = PyPiClient::select_latest_version(
      &"foo".parse().unwrap(),
      PyPiResponse {
        info: PackageInfo {
          version: "1.0.0".to_string(),
        },
        releases: HashMap::new(),
      },
    )
    .unwrap();

    assert_eq!(version, "1.0.0".parse().unwrap());
  }

  #[test]
  fn selects_latest_prerelease_without_releases() {
    let version = PyPiClient::select_latest_version(
      &"foo".parse().unwrap(),
      PyPiResponse {
        info: PackageInfo {
          version: "0.1.0".to_string(),
        },
        releases: HashMap::from([
          ("1.0.0".to_string(), vec![ReleaseFile { yanked: true }]),
          ("2.0.0a1".to_string(), vec![ReleaseFile { yanked: false }]),
          ("invalid".to_string(), vec![ReleaseFile { yanked: false }]),
        ]),
      },
    )
    .unwrap();

    assert_eq!(version, "2.0.0a1".parse().unwrap());
  }

  #[test]
  fn selects_latest_release_over_prerelease() {
    let version = PyPiClient::select_latest_version(
      &"foo".parse().unwrap(),
      PyPiResponse {
        info: PackageInfo {
          version: "0.1.0".to_string(),
        },
        releases: HashMap::from([
          ("1.0.0".to_string(), vec![ReleaseFile { yanked: false }]),
          ("2.0.0a1".to_string(), vec![ReleaseFile { yanked: false }]),
          ("1.1.0".to_string(), vec![ReleaseFile { yanked: false }]),
        ]),
      },
    )
    .unwrap();

    assert_eq!(version, "1.1.0".parse().unwrap());
  }
}
