use super::*;

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Config {
  #[serde(default)]
  pub(crate) rules: HashMap<String, RuleConfig>,
  #[serde(default)]
  pub(crate) schemas: HashMap<String, String>,
}

impl Config {
  pub(crate) fn add_schema(&mut self, specification: &str) -> Result {
    let Some((tool, url)) = specification.split_once('=') else {
      bail!("schema must use the form `TOOL=URL`");
    };

    if tool.is_empty() || url.is_empty() {
      bail!("schema must use the form `TOOL=URL`");
    }

    self.schemas.insert(tool.to_string(), url.to_string());

    Ok(())
  }

  pub(crate) fn rule_config(&self, id: &str) -> RuleConfig {
    self.rules.get(id).cloned().unwrap_or_default()
  }
}

impl From<Node> for Config {
  fn from(node: Node) -> Self {
    match serde_json::to_value(&node).and_then(serde_json::from_value) {
      Ok(config) => config,
      Err(error) => {
        warn!("failed to parse `[tool.pyproject]` configuration: {error}");
        Self::default()
      }
    }
  }
}

impl From<&Parse> for Config {
  fn from(tree: &Parse) -> Self {
    let root = tree.clone().into_dom();

    let Ok(tool) = root.try_get("tool") else {
      return Self::default();
    };

    let Ok(pyproject) = tool.try_get("pyproject") else {
      return Self::default();
    };

    Self::from(pyproject)
  }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RuleLevel {
  Error,
  Hint,
  #[serde(alias = "info")]
  Information,
  Off,
  Warning,
}

impl From<RuleLevel> for lsp::DiagnosticSeverity {
  fn from(value: RuleLevel) -> Self {
    match value {
      RuleLevel::Error => Self::ERROR,
      RuleLevel::Hint | RuleLevel::Off => Self::HINT,
      RuleLevel::Information => Self::INFORMATION,
      RuleLevel::Warning => Self::WARNING,
    }
  }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum RuleConfig {
  Level(RuleLevel),
  Settings {
    #[serde(default)]
    level: Option<RuleLevel>,
  },
}

impl RuleConfig {
  pub(crate) fn level(&self) -> Option<RuleLevel> {
    match self {
      RuleConfig::Level(level) => Some(*level),
      RuleConfig::Settings { level } => *level,
    }
  }

  pub(crate) fn severity(
    &self,
    default: lsp::DiagnosticSeverity,
    default_level: Option<RuleLevel>,
  ) -> Option<lsp::DiagnosticSeverity> {
    match self.level().or(default_level) {
      None => Some(default),
      Some(RuleLevel::Off) => None,
      Some(level) => Some(level.into()),
    }
  }
}

impl Default for RuleConfig {
  fn default() -> Self {
    Self::Settings { level: None }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn custom_schema_overrides_configuration() {
    let mut config: Config = serde_json::from_value(json!({
      "schemas": {
        "foo": "file:///foo.json"
      }
    }))
    .unwrap();

    config.add_schema("foo=file:///bar.json").unwrap();

    assert_eq!(
      config.schemas.get("foo"),
      Some(&"file:///bar.json".to_string())
    );
  }

  #[test]
  fn parses_rule_config_from_string() {
    let config: Config = serde_json::from_value(json!({
      "rules": {
        "demo": "warning"
      }
    }))
    .unwrap();

    assert_eq!(config.rule_config("demo").level(), Some(RuleLevel::Warning));
  }

  #[test]
  fn parses_rule_config_from_table() {
    let config: Config = serde_json::from_value(json!({
      "rules": {
        "demo": { "level": "hint" }
      }
    }))
    .unwrap();

    assert_eq!(config.rule_config("demo").level(), Some(RuleLevel::Hint));
  }

  #[test]
  fn parses_schema_mapping() {
    let config: Config = serde_json::from_value(json!({
      "schemas": {
        "foo": "file:///foo.json"
      }
    }))
    .unwrap();

    assert_eq!(
      config.schemas.get("foo"),
      Some(&"file:///foo.json".to_string())
    );
  }
}
