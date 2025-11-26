use super::*;

#[derive(Debug)]
enum CompletionContext {
  /// In an array item context.
  ArrayItem { path: Vec<String>, prefix: String },
  /// In a key position within a table.
  Key { path: Vec<String>, prefix: String },
  /// Inside a table header: `[` prefix or `[[` prefix.
  TableHeader { prefix: String },
  /// Unknown/unsupported context.
  Unknown,
  /// In a value position after `=`.
  Value { path: Vec<String>, prefix: String },
}

pub(crate) struct Completions<'a> {
  document: &'a Document,
  position: lsp::Position,
}

impl<'a> Completions<'a> {
  fn analyze_context(&self) -> CompletionContext {
    let content = self.document.content.to_string();

    let (line_idx, char_idx) = (
      self.position.line as usize,
      self.position.character as usize,
    );

    let lines = content.lines().collect::<Vec<&str>>();

    if line_idx >= lines.len() {
      return CompletionContext::Unknown;
    }

    let line = lines[line_idx];

    let line_prefix = if char_idx <= line.len() {
      &line[..char_idx]
    } else {
      line
    };

    if let Some(ctx) = Self::check_table_header(line_prefix) {
      return ctx;
    }

    let current_table = Self::find_current_table(&lines, line_idx);

    if let Some(ctx) =
      Self::check_key_value_context(line_prefix, &current_table)
    {
      return ctx;
    }

    CompletionContext::Unknown
  }

  fn array_item_completions(
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let prefix = prefix.to_lowercase();

    match path.join(".").as_str() {
      "project.classifiers" => Self::classifier_completions(&prefix),
      "project.dynamic" => Self::dynamic_field_completions(&prefix),
      "build-system.requires" => Self::build_requires_completions(&prefix),
      "project.keywords" => Vec::new(),
      "project.dependencies" | "project.optional-dependencies" => {
        Self::dependency_completions(&prefix)
      }
      _ => Self::schema_array_item_completions(path, &prefix),
    }
  }

  fn build_backend_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let backends = [
      ("hatchling.build", "Hatchling - Modern Python build backend"),
      (
        "setuptools.build_meta",
        "Setuptools - Traditional Python build backend",
      ),
      ("flit_core.buildapi", "Flit - Simple PEP 517 build backend"),
      ("pdm.backend", "PDM - Modern Python package manager backend"),
      (
        "poetry.core.masonry.api",
        "Poetry - Python packaging and dependency management",
      ),
      (
        "maturin",
        "Maturin - Build backend for Rust Python extensions",
      ),
      (
        "scikit_build_core.build",
        "Scikit-build-core - CMake-based build system",
      ),
      (
        "meson-python",
        "Meson-python - Meson build system for Python",
      ),
    ];

    backends
      .iter()
      .filter(|(name, _)| Self::matches_prefix(name, prefix))
      .map(|(name, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::VALUE),
        detail: Some((*desc).to_string()),
        insert_text: Some(format!("\"{name}\"")),
        insert_text_format: Some(lsp::InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
      })
      .collect()
  }

  fn build_requires_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let packages = [
      ("hatchling", "Modern Python build backend"),
      ("setuptools>=61.0", "Setuptools with pyproject.toml support"),
      ("wheel", "Wheel package format support"),
      ("flit_core>=3.4", "Flit build backend"),
      ("pdm-backend", "PDM build backend"),
      ("poetry-core>=1.0.0", "Poetry build backend"),
      ("maturin>=1.0", "Rust extension build backend"),
      ("scikit-build-core>=0.4", "CMake build backend"),
      ("meson-python", "Meson build system"),
      ("cython>=3.0", "Cython compilation support"),
    ];

    packages
      .iter()
      .filter(|(name, _)| Self::matches_prefix(name, prefix))
      .map(|(name, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::MODULE),
        detail: Some((*desc).to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn build_system_key_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let keys = [
      ("requires", "array", "Build dependencies (PEP 508 strings)"),
      ("build-backend", "string", "The build backend to use"),
      (
        "backend-path",
        "array",
        "Paths to add to sys.path for the backend",
      ),
    ];

    Self::filter_keys(&keys, prefix)
  }

  fn check_key_value_context(
    line_prefix: &str,
    current_table: &[String],
  ) -> Option<CompletionContext> {
    let trimmed = line_prefix.trim_start();

    if trimmed.starts_with('#') || trimmed.starts_with('[') {
      return None;
    }

    if let Some(eq_pos) = line_prefix.rfind('=') {
      let after_eq = &line_prefix[eq_pos + 1..];
      let trimmed_after = after_eq.trim_start();

      let before_eq = &line_prefix[..eq_pos];
      let key = before_eq.trim().trim_matches('"').trim_matches('\'');

      let mut path = current_table.to_vec();
      path.push(key.to_string());

      if let Some(array_content) = trimmed_after.strip_prefix('[') {
        return Some(CompletionContext::ArrayItem {
          path,
          prefix: Self::extract_array_item_prefix(array_content),
        });
      }

      return Some(CompletionContext::Value {
        path,
        prefix: trimmed_after
          .trim_matches('"')
          .trim_matches('\'')
          .to_string(),
      });
    }

    Some(CompletionContext::Key {
      path: current_table.to_vec(),
      prefix: trimmed.to_string(),
    })
  }

  fn check_table_header(line_prefix: &str) -> Option<CompletionContext> {
    let trimmed = line_prefix.trim_start();

    if let Some(stripped) = trimmed.strip_prefix("[[") {
      return Some(CompletionContext::TableHeader {
        prefix: stripped.trim_start().to_string(),
      });
    }

    if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
      let prefix = trimmed[1..].trim_start();

      if !prefix.contains(']') {
        return Some(CompletionContext::TableHeader {
          prefix: prefix.to_string(),
        });
      }
    }

    None
  }

  fn classifier_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    Self::classifiers()
      .iter()
      .filter(|c| Self::matches_prefix(c, prefix))
      .take(100)
      .map(|c| lsp::CompletionItem {
        label: (*c).to_string(),
        kind: Some(lsp::CompletionItemKind::ENUM_MEMBER),
        insert_text: Some(format!("\"{c}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn classifiers() -> &'static Vec<&'static str> {
    static CLASSIFIERS: OnceLock<Vec<&'static str>> = OnceLock::new();

    CLASSIFIERS.get_or_init(|| {
      include_str!("rule/classifiers.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
    })
  }

  pub(crate) fn completions(&self) -> Vec<lsp::CompletionItem> {
    let context = self.analyze_context();

    match context {
      CompletionContext::TableHeader { prefix } => {
        Self::table_header_completions(&prefix)
      }
      CompletionContext::Key { path, prefix } => {
        Self::key_completions(&path, &prefix)
      }
      CompletionContext::Value { path, prefix } => {
        Self::value_completions(&path, &prefix)
      }
      CompletionContext::ArrayItem { path, prefix } => {
        Self::array_item_completions(&path, &prefix)
      }
      CompletionContext::Unknown => Vec::new(),
    }
  }

  fn dependency_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let packages = [
      ("requests", "HTTP library for Python"),
      ("numpy", "Numerical computing library"),
      ("pandas", "Data analysis library"),
      ("pytest", "Testing framework"),
      ("black", "Code formatter"),
      ("ruff", "Fast Python linter"),
      ("mypy", "Static type checker"),
      ("click", "CLI framework"),
      ("fastapi", "Modern web framework"),
      ("flask", "Web microframework"),
      ("django", "Web framework"),
      ("sqlalchemy", "Database toolkit"),
      ("pydantic", "Data validation library"),
      ("httpx", "Async HTTP client"),
      ("rich", "Terminal formatting library"),
      ("typer", "CLI builder"),
    ];

    packages
      .iter()
      .filter(|(name, _)| Self::matches_prefix(name, prefix))
      .map(|(name, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::MODULE),
        detail: Some((*desc).to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn dynamic_field_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let fields = [
      "version",
      "description",
      "readme",
      "license",
      "license-files",
      "authors",
      "maintainers",
      "keywords",
      "classifiers",
      "urls",
      "dependencies",
      "optional-dependencies",
      "scripts",
      "gui-scripts",
      "entry-points",
    ];

    fields
      .iter()
      .filter(|f| Self::matches_prefix(f, prefix))
      .map(|f| lsp::CompletionItem {
        label: (*f).to_string(),
        kind: Some(lsp::CompletionItemKind::ENUM_MEMBER),
        detail: Some("Dynamic field".to_string()),
        insert_text: Some(format!("\"{f}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn enum_completions(
    enum_values: &[Value],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    enum_values
      .iter()
      .filter_map(|v| {
        let s = match v {
          Value::String(s) => s.clone(),
          Value::Bool(b) => b.to_string(),
          Value::Number(n) => n.to_string(),
          _ => return None,
        };

        if Self::matches_prefix(&s, prefix) {
          Some(lsp::CompletionItem {
            label: s.clone(),
            kind: Some(lsp::CompletionItemKind::ENUM_MEMBER),
            insert_text: Some(format!("\"{s}\"")),
            ..Default::default()
          })
        } else {
          None
        }
      })
      .collect()
  }

  fn extract_array_item_prefix(content: &str) -> String {
    let last_separator = content.rfind(',').map_or(0, |i| i + 1);

    let item_content = &content[last_separator..];

    item_content
      .trim()
      .trim_start_matches('"')
      .trim_start_matches('\'')
      .to_string()
  }

  fn filter_keys(
    keys: &[(&str, &str, &str)],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    keys
      .iter()
      .filter(|(name, _, _)| Self::matches_prefix(name, prefix))
      .map(|(name, type_str, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::PROPERTY),
        detail: Some((*type_str).to_string()),
        documentation: Some(lsp::Documentation::MarkupContent(
          lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: (*desc).to_string(),
          },
        )),
        insert_text: Some((*name).to_string()),
        ..Default::default()
      })
      .collect()
  }

  fn find_current_table(lines: &[&str], current_line: usize) -> Vec<String> {
    for i in (0..=current_line).rev() {
      let line = lines[i].trim();

      if line.is_empty() || line.starts_with('#') {
        continue;
      }

      if line.starts_with("[[") && line.ends_with("]]") {
        let path = &line[2..line.len() - 2];
        return path.split('.').map(|s| s.trim().to_string()).collect();
      }

      if line.starts_with('[') && line.ends_with(']') && !line.starts_with("[[")
      {
        let path = &line[1..line.len() - 1];
        return path.split('.').map(|s| s.trim().to_string()).collect();
      }
    }

    Vec::new()
  }

  fn get_schema_for_pointer(pointer: &str) -> Option<(Value, bool)> {
    let path = pointer.trim_start_matches('/');

    if path.starts_with("tool/") || path == "tool" {
      let parts = path.split('/').collect::<Vec<&str>>();

      if parts.len() >= 2 {
        let tool_name = parts[1];

        for schema in SCHEMAS {
          if schema.tool == Some(tool_name) {
            return serde_json::from_str(schema.contents)
              .ok()
              .map(|schema| (schema, true));
          }
        }
      }
    }

    Some((SchemaStore::root().clone(), false))
  }

  fn get_type_string(schema: &Value) -> String {
    if let Some(type_val) = schema.get("type") {
      match type_val {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
          .iter()
          .filter_map(Value::as_str)
          .collect::<Vec<_>>()
          .join(" | "),
        _ => "unknown".to_string(),
      }
    } else if schema.get("enum").is_some() {
      "enum".to_string()
    } else if schema.get("oneOf").is_some() || schema.get("anyOf").is_some() {
      "variant".to_string()
    } else {
      "unknown".to_string()
    }
  }

  fn key_completions(
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();

    let prefix = prefix.to_lowercase();

    match path.join(".").as_str() {
      "" => {
        items.extend(Self::root_key_completions(&prefix));
      }
      "project" => {
        items.extend(Self::project_key_completions(&prefix));
      }
      "build-system" => {
        items.extend(Self::build_system_key_completions(&prefix));
      }
      "tool" => {
        items.extend(Self::tool_key_completions(&prefix));
      }
      _ => {
        items.extend(Self::schema_key_completions(path, &prefix));
      }
    }

    items
  }

  fn license_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let licenses = [
      ("MIT", "MIT License"),
      ("Apache-2.0", "Apache License 2.0"),
      ("GPL-3.0-only", "GNU General Public License v3.0 only"),
      (
        "GPL-3.0-or-later",
        "GNU General Public License v3.0 or later",
      ),
      (
        "BSD-3-Clause",
        "BSD 3-Clause \"New\" or \"Revised\" License",
      ),
      ("BSD-2-Clause", "BSD 2-Clause \"Simplified\" License"),
      ("ISC", "ISC License"),
      ("MPL-2.0", "Mozilla Public License 2.0"),
      (
        "LGPL-3.0-only",
        "GNU Lesser General Public License v3.0 only",
      ),
      ("Unlicense", "The Unlicense"),
      ("CC0-1.0", "Creative Commons Zero v1.0 Universal"),
      (
        "AGPL-3.0-only",
        "GNU Affero General Public License v3.0 only",
      ),
    ];

    licenses
      .iter()
      .filter(|(name, _)| Self::matches_prefix(name, prefix))
      .map(|(name, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::VALUE),
        detail: Some((*desc).to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn matches_prefix(value: &str, prefix: &str) -> bool {
    prefix.is_empty() || value.to_ascii_lowercase().starts_with(prefix)
  }

  pub(crate) fn new(document: &'a Document, position: lsp::Position) -> Self {
    Self { document, position }
  }

  fn pointer_from_path(path: &[String]) -> String {
    if path.is_empty() {
      String::new()
    } else {
      format!("/{}", path.join("/"))
    }
  }

  fn project_key_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let keys = [
      ("name", "string", "The name of the project (required)"),
      ("version", "string", "The version of the project"),
      ("description", "string", "A short summary description"),
      (
        "readme",
        "string/table",
        "Path to README or inline readme config",
      ),
      (
        "requires-python",
        "string",
        "Python version requirement (PEP 440)",
      ),
      (
        "license",
        "string/table",
        "License expression or license file config",
      ),
      ("license-files", "array", "Paths/globs for license files"),
      ("authors", "array", "List of author entries with name/email"),
      (
        "maintainers",
        "array",
        "List of maintainer entries with name/email",
      ),
      ("keywords", "array", "Keywords for the project"),
      ("classifiers", "array", "Trove classifiers for the project"),
      ("urls", "table", "Project URLs (homepage, repository, etc.)"),
      (
        "dependencies",
        "array",
        "Runtime dependencies (PEP 508 strings)",
      ),
      (
        "optional-dependencies",
        "table",
        "Optional dependency groups",
      ),
      ("scripts", "table", "Console script entry points"),
      ("gui-scripts", "table", "GUI script entry points"),
      ("entry-points", "table", "Other entry point groups"),
      (
        "dynamic",
        "array",
        "Fields that are dynamically set by the build backend",
      ),
    ];

    Self::filter_keys(&keys, prefix)
  }

  fn readme_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let values = [
      ("README.md", "Markdown readme file"),
      ("README.rst", "reStructuredText readme file"),
      ("README.txt", "Plain text readme file"),
    ];

    values
      .iter()
      .filter(|(name, _)| Self::matches_prefix(name, prefix))
      .map(|(name, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::FILE),
        detail: Some((*desc).to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn requires_python_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let versions = [
      (">=3.9", "Python 3.9 or later"),
      (">=3.10", "Python 3.10 or later"),
      (">=3.11", "Python 3.11 or later"),
      (">=3.12", "Python 3.12 or later"),
      (">=3.13", "Python 3.13 or later"),
      (">=3.9,<4", "Python 3.9 to 3.x (recommended)"),
      (">=3.10,<4", "Python 3.10 to 3.x (recommended)"),
      (">=3.11,<4", "Python 3.11 to 3.x (recommended)"),
      (">=3.12,<4", "Python 3.12 to 3.x (recommended)"),
    ];

    versions
      .iter()
      .filter(|(name, _)| Self::matches_prefix(name, prefix))
      .map(|(name, desc)| lsp::CompletionItem {
        label: (*name).to_string(),
        kind: Some(lsp::CompletionItemKind::VALUE),
        detail: Some((*desc).to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  fn root_key_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let keys = [
      ("project", "table", "PEP 621 project metadata table"),
      (
        "build-system",
        "table",
        "PEP 517 build system configuration",
      ),
      ("tool", "table", "Tool-specific configuration sections"),
      ("dependency-groups", "table", "PEP 735 dependency groups"),
    ];

    Self::filter_keys(&keys, prefix)
  }

  fn schema_array_item_completions(
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let Some(schema) = Self::schema_for_path(path) else {
      return Vec::new();
    };

    let enum_values = schema
      .get("items")
      .and_then(|items| items.get("enum"))
      .and_then(Value::as_array);

    enum_values
      .map(|values| Self::enum_completions(values, prefix))
      .unwrap_or_default()
  }

  fn schema_for_path(path: &[String]) -> Option<Value> {
    let pointer = Self::pointer_from_path(path);

    let (schema, is_tool_schema) = Self::get_schema_for_pointer(&pointer)?;

    let effective_path = if is_tool_schema && path.len() >= 2 {
      &path[2..]
    } else {
      path
    };

    Self::traverse_schema(&schema, effective_path).cloned()
  }

  fn schema_key_completions(
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();

    if let Some(properties) = Self::schema_properties_at_path(path) {
      for (key, value) in properties {
        if Self::matches_prefix(&key, prefix) {
          let description = value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");

          let type_str = Self::get_type_string(&value);

          items.push(lsp::CompletionItem {
            label: key.clone(),
            kind: Some(lsp::CompletionItemKind::PROPERTY),
            detail: Some(type_str),
            documentation: if description.is_empty() {
              None
            } else {
              Some(lsp::Documentation::MarkupContent(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: description.to_string(),
              }))
            },
            insert_text: Some(key),
            ..Default::default()
          });
        }
      }
    }

    items
  }

  fn schema_properties_at_path(path: &[String]) -> Option<Map<String, Value>> {
    let pointer = Self::pointer_from_path(path);

    let (schema, is_tool_schema) = Self::get_schema_for_pointer(&pointer)?;

    let effective_path = if is_tool_schema && path.len() >= 2 {
      &path[2..]
    } else {
      path
    };

    Self::traverse_schema(&schema, effective_path)?
      .get("properties")
      .and_then(Value::as_object)
      .cloned()
  }

  fn schema_value_completions(
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let Some(schema) = Self::schema_for_path(path) else {
      return Vec::new();
    };

    schema
      .get("enum")
      .and_then(Value::as_array)
      .map(|values| Self::enum_completions(values, prefix))
      .unwrap_or_default()
  }

  fn table_header_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();

    let prefix = prefix.to_lowercase();

    let sections = [
      ("project", "PEP 621 project metadata"),
      ("project.scripts", "Console script entry points"),
      ("project.gui-scripts", "GUI script entry points"),
      ("project.entry-points", "Entry point groups"),
      (
        "project.optional-dependencies",
        "Optional dependency groups",
      ),
      ("project.urls", "Project URLs"),
      ("build-system", "PEP 517 build system configuration"),
      ("dependency-groups", "PEP 735 dependency groups"),
      ("tool", "Tool-specific configuration"),
    ];

    for (name, description) in sections {
      if Self::matches_prefix(name, &prefix) {
        items.push(lsp::CompletionItem {
          label: name.to_string(),
          kind: Some(lsp::CompletionItemKind::MODULE),
          detail: Some(description.to_string()),
          insert_text: Some(name.to_string()),
          ..Default::default()
        });
      }
    }

    for schema in SCHEMAS {
      if let Some(tool) = schema.tool {
        let full_path = format!("tool.{tool}");

        if Self::matches_prefix(&full_path, &prefix) {
          items.push(lsp::CompletionItem {
            label: full_path.clone(),
            kind: Some(lsp::CompletionItemKind::MODULE),
            detail: Some(format!("{tool} configuration")),
            insert_text: Some(full_path),
            ..Default::default()
          });
        }
      }
    }

    items
  }

  fn tool_key_completions(prefix: &str) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();

    for schema in SCHEMAS {
      if let Some(tool) = schema.tool
        && Self::matches_prefix(tool, prefix)
      {
        items.push(lsp::CompletionItem {
          label: tool.to_string(),
          kind: Some(lsp::CompletionItemKind::PROPERTY),
          detail: Some(format!("{tool} configuration section")),
          insert_text: Some(tool.to_string()),
          ..Default::default()
        });
      }
    }

    items
  }

  fn traverse_schema<'schema>(
    mut current: &'schema Value,
    path: &[String],
  ) -> Option<&'schema Value> {
    for segment in path {
      if let Some(props) = current.get("properties")
        && let Some(prop) = props.get(segment)
      {
        current = prop;
        continue;
      }

      if let Some(additional) = current.get("additionalProperties")
        && additional.is_object()
      {
        current = additional;
        continue;
      }

      return None;
    }

    Some(current)
  }

  fn value_completions(
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let path_str = path.join(".");

    let prefix = prefix.to_lowercase();

    match path_str.as_str() {
      "build-system.build-backend" => Self::build_backend_completions(&prefix),
      "project.readme" => Self::readme_completions(&prefix),
      "project.license" => Self::license_completions(&prefix),
      "project.requires-python" => Self::requires_python_completions(&prefix),
      _ => Self::schema_value_completions(path, &prefix),
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, indoc::indoc};

  fn completions(content: &str, line: u32, character: u32) -> Vec<String> {
    let document = Document::from(content);

    let position = lsp::Position { line, character };

    let completions = Completions::new(&document, position);

    let mut completions = completions
      .completions()
      .into_iter()
      .map(|completion| completion.label)
      .collect::<Vec<String>>();

    completions.sort();

    completions
  }

  #[test]
  fn completes_table_headers() {
    assert_eq!(
      completions("[", 0, 1),
      vec![
        "build-system",
        "dependency-groups",
        "project",
        "project.entry-points",
        "project.gui-scripts",
        "project.optional-dependencies",
        "project.scripts",
        "project.urls",
        "tool",
        "tool.black",
        "tool.cibuildwheel",
        "tool.hatch",
        "tool.maturin",
        "tool.mypy",
        "tool.pdm",
        "tool.poe",
        "tool.poetry",
        "tool.pyright",
        "tool.pytest",
        "tool.repo-review",
        "tool.ruff",
        "tool.scikit-build",
        "tool.setuptools",
        "tool.setuptools_scm",
        "tool.taskipy",
        "tool.tombi",
        "tool.tox",
        "tool.ty",
        "tool.uv",
      ]
    );
  }

  #[test]
  fn completes_table_headers_with_prefix() {
    assert_eq!(
      completions("[pro", 0, 4),
      vec![
        "project",
        "project.entry-points",
        "project.gui-scripts",
        "project.optional-dependencies",
        "project.scripts",
        "project.urls",
      ]
    );
  }

  #[test]
  fn completes_tool_table_headers() {
    assert_eq!(
      completions("[tool.", 0, 6),
      vec![
        "tool.black",
        "tool.cibuildwheel",
        "tool.hatch",
        "tool.maturin",
        "tool.mypy",
        "tool.pdm",
        "tool.poe",
        "tool.poetry",
        "tool.pyright",
        "tool.pytest",
        "tool.repo-review",
        "tool.ruff",
        "tool.scikit-build",
        "tool.setuptools",
        "tool.setuptools_scm",
        "tool.taskipy",
        "tool.tombi",
        "tool.tox",
        "tool.ty",
        "tool.uv",
      ]
    );
  }

  #[test]
  fn completes_project_keys() {
    let content = indoc! {
      r"
      [project]

      "
    };

    assert_eq!(
      completions(content, 1, 0),
      vec![
        "authors",
        "classifiers",
        "dependencies",
        "description",
        "dynamic",
        "entry-points",
        "gui-scripts",
        "keywords",
        "license",
        "license-files",
        "maintainers",
        "name",
        "optional-dependencies",
        "readme",
        "requires-python",
        "scripts",
        "urls",
        "version",
      ]
    );
  }

  #[test]
  fn completes_project_keys_with_prefix() {
    let content = indoc! {
      r"
      [project]
      de
      "
    };

    assert_eq!(
      completions(content, 1, 2),
      vec!["dependencies", "description"]
    );
  }

  #[test]
  fn completes_build_backend_values() {
    let content = indoc! {
      r"
      [build-system]
      build-backend =
      "
    };

    let labels = completions(content, 1, 16);

    assert_eq!(
      labels,
      vec![
        "flit_core.buildapi",
        "hatchling.build",
        "maturin",
        "meson-python",
        "pdm.backend",
        "poetry.core.masonry.api",
        "scikit_build_core.build",
        "setuptools.build_meta",
      ]
    );
  }

  #[test]
  fn completes_license_values() {
    let content = indoc! {
      r#"
      [project]
      name = "test"
      license =
      "#
    };

    assert_eq!(
      completions(content, 2, 10),
      vec![
        "AGPL-3.0-only",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "CC0-1.0",
        "GPL-3.0-only",
        "GPL-3.0-or-later",
        "ISC",
        "LGPL-3.0-only",
        "MIT",
        "MPL-2.0",
        "Unlicense",
      ]
    );
  }

  #[test]
  fn completes_classifiers_in_array() {
    let content = indoc! {
      r#"
      [project]
      name = "test"
      classifiers = ["Development
      "#
    };

    let labels = completions(content, 2, 28);

    assert!(!labels.is_empty());

    assert!(
      labels
        .iter()
        .all(|label| label.starts_with("Development Status ::"))
    );
  }

  #[test]
  fn completes_dynamic_fields() {
    let content = indoc! {
      r#"
      [project]
      name = "test"
      dynamic = ["
      "#
    };

    assert_eq!(
      completions(content, 2, 12),
      vec![
        "authors",
        "classifiers",
        "dependencies",
        "description",
        "entry-points",
        "gui-scripts",
        "keywords",
        "license",
        "license-files",
        "maintainers",
        "optional-dependencies",
        "readme",
        "scripts",
        "urls",
        "version",
      ]
    );
  }

  #[test]
  fn completes_build_system_keys() {
    let content = indoc! {
      r"
      [build-system]

      "
    };

    assert_eq!(
      completions(content, 1, 0),
      vec!["backend-path", "build-backend", "requires"]
    );
  }

  #[test]
  fn completes_requires_python() {
    let content = indoc! {
      r#"
      [project]
      name = "test"
      requires-python =
      "#
    };

    assert_eq!(
      completions(content, 2, 18),
      vec![
        ">=3.10",
        ">=3.10,<4",
        ">=3.11",
        ">=3.11,<4",
        ">=3.12",
        ">=3.12,<4",
        ">=3.13",
        ">=3.9",
        ">=3.9,<4",
      ]
    );
  }

  #[test]
  fn completes_tool_keys() {
    let content = indoc! {
      r"
      [tool]

      "
    };

    assert_eq!(
      completions(content, 1, 0),
      vec![
        "black",
        "cibuildwheel",
        "hatch",
        "maturin",
        "mypy",
        "pdm",
        "poe",
        "poetry",
        "pyright",
        "pytest",
        "repo-review",
        "ruff",
        "scikit-build",
        "setuptools",
        "setuptools_scm",
        "taskipy",
        "tombi",
        "tox",
        "ty",
        "uv",
      ]
    );
  }

  #[test]
  fn completes_tool_black_keys() {
    let content = indoc! {
      r"
      [tool.black]

      "
    };

    assert_eq!(
      completions(content, 1, 0),
      vec![
        "check",
        "code",
        "color",
        "diff",
        "enable-unstable-feature",
        "exclude",
        "extend-exclude",
        "fast",
        "force-exclude",
        "include",
        "ipynb",
        "line-length",
        "preview",
        "pyi",
        "python-cell-magics",
        "quiet",
        "required-version",
        "skip-magic-trailing-comma",
        "skip-source-first-line",
        "skip-string-normalization",
        "target-version",
        "unstable",
        "verbose",
        "workers",
      ]
    );
  }

  #[test]
  fn completes_empty_returns_all_options() {
    let content = indoc! {
      r#"
      [project]
      name = "test"
      license = ""
      "#
    };

    let labels = completions(content, 2, 11);

    assert_eq!(
      labels,
      vec![
        "AGPL-3.0-only",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "CC0-1.0",
        "GPL-3.0-only",
        "GPL-3.0-or-later",
        "ISC",
        "LGPL-3.0-only",
        "MIT",
        "MPL-2.0",
        "Unlicense",
      ]
    );
  }
}
