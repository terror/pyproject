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

pub(crate) struct PyPiClient {
  base_url: String,
  cache: Mutex<HashMap<String, Version>>,
  http: ReqwestClient,
}

impl PyPiClient {
  pub(crate) fn latest_version(
    &self,
    package: &PackageName,
  ) -> Option<Version> {
    let name = package.to_string();

    let cache_key = format!("{}/{}", self.base_url, name);

    if let Some(version) = self
      .cache
      .lock()
      .inspect_err(|error| debug!("failed to lock PyPI cache: {error}"))
      .ok()
      .and_then(|cache| cache.get(&cache_key).cloned())
    {
      return Some(version);
    }

    let payload = self
      .http
      .get(format!("{}/pypi/{}/json", self.base_url, name))
      .send()
      .ok()?
      .error_for_status()
      .ok()?
      .json::<PyPiResponse>()
      .ok()?;

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

    let latest = latest_release
      .or(latest_prerelease)
      .or_else(|| Version::from_str(&payload.info.version).ok())?;

    if let Ok(mut cache) = self.cache.lock() {
      cache.insert(cache_key, latest.clone());
    } else {
      debug!("failed to lock PyPI cache for insert");
    }

    Some(latest)
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

  pub(crate) fn shared() -> &'static Self {
    static INSTANCE: OnceLock<PyPiClient> = OnceLock::new();

    INSTANCE.get_or_init(Self::new)
  }
}

#[cfg(test)]
mod tests {
  use {super::*, mockito::Server};

  #[test]
  fn latest_version() {
    #[track_caller]
    fn case(body: &str, expected: &str) {
      let mut server = Server::new();

      let mock = server
        .mock("GET", "/pypi/foo/json")
        .with_body(body)
        .create();

      let client = PyPiClient {
        base_url: server.url(),
        cache: Mutex::new(HashMap::new()),
        http: ReqwestClient::new(),
      };

      let package = "foo".parse().unwrap();

      assert_eq!(
        client.latest_version(&package),
        Some(expected.parse().unwrap())
      );

      assert_eq!(
        client.latest_version(&package),
        Some(expected.parse().unwrap())
      );

      mock.assert();
    }

    case(
      r#"{
        "info": { "version": "1.0.0" },
        "releases": {}
      }"#,
      "1.0.0",
    );

    case(
      r#"{
        "info": { "version": "0.1.0" },
        "releases": {
          "1.0.0": [{ "yanked": true }],
          "2.0.0a1": [{ "yanked": false }],
          "invalid": [{ "yanked": false }]
        }
      }"#,
      "2.0.0a1",
    );

    case(
      r#"{
        "info": { "version": "0.1.0" },
        "releases": {
          "1.0.0": [{ "yanked": false }],
          "2.0.0a1": [{ "yanked": false }],
          "1.1.0": [{ "yanked": false }]
        }
      }"#,
      "1.1.0",
    );
  }
}
