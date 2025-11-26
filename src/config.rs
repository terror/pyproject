use super::*;

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Config {
  #[serde(default)]
  pub(crate) rules: HashMap<String, RuleConfig>,
}

impl Config {
  fn from_node(node: Node) -> Self {
    match serde_json::to_value(&node).and_then(serde_json::from_value) {
      Ok(config) => config,
      Err(error) => {
        warn!("failed to parse `[tool.pyproject]` configuration: {error}");
        Self::default()
      }
    }
  }

  pub(crate) fn from_tree(tree: &Parse) -> Self {
    let root = tree.clone().into_dom();

    let Ok(tool) = root.try_get("tool") else {
      return Self::default();
    };

    let Ok(pyproject) = tool.try_get("pyproject") else {
      return Self::default();
    };

    Self::from_node(pyproject)
  }

  pub(crate) fn rule_config(&self, id: &str) -> RuleConfig {
    self.rules.get(id).cloned().unwrap_or_default()
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

impl Default for RuleConfig {
  fn default() -> Self {
    Self::Settings { level: None }
  }
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
  ) -> Option<lsp::DiagnosticSeverity> {
    match self.level() {
      None => Some(default),
      Some(RuleLevel::Off) => None,
      Some(level) => Some(level.into()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
}
