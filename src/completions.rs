use super::*;

/// Context-aware completions engine for pyproject.toml files.
/// Provides completions based on JSON Schema, position in document,
/// and semantic understanding of the pyproject.toml structure.
pub(crate) struct Completions<'a> {
  document: &'a Document,
  position: lsp::Position,
}

impl<'a> Completions<'a> {
  pub(crate) fn new(document: &'a Document, position: lsp::Position) -> Self {
    Self { document, position }
  }

  /// Generate completions for the current cursor position.
  pub(crate) fn completions(&self) -> Vec<lsp::CompletionItem> {
    let context = self.analyze_context();

    match context {
      CompletionContext::TableHeader { prefix } => {
        self.table_header_completions(&prefix)
      }
      CompletionContext::Key { path, prefix } => {
        self.key_completions(&path, &prefix)
      }
      CompletionContext::Value { path, prefix } => {
        self.value_completions(&path, &prefix)
      }
      CompletionContext::ArrayItem { path, prefix } => {
        self.array_item_completions(&path, &prefix)
      }
      CompletionContext::Unknown => Vec::new(),
    }
  }

  /// Analyze the document context at the current position.
  fn analyze_context(&self) -> CompletionContext {
    let content = self.document.content.to_string();
    let line_idx = self.position.line as usize;
    let char_idx = self.position.character as usize;

    let lines: Vec<&str> = content.lines().collect();

    if line_idx >= lines.len() {
      return CompletionContext::Unknown;
    }

    let line = lines[line_idx];
    let line_prefix = if char_idx <= line.len() {
      &line[..char_idx]
    } else {
      line
    };

    // Check if we're in a table header: [table] or [[array]]
    if let Some(ctx) = self.check_table_header(line_prefix) {
      return ctx;
    }

    // Determine current table path from preceding table headers
    let current_table = self.find_current_table(&lines, line_idx);

    // Check if we're in a key position (before =) or value position (after =)
    if let Some(ctx) = self.check_key_value_context(line_prefix, &current_table)
    {
      return ctx;
    }

    CompletionContext::Unknown
  }

  /// Check if we're editing a table header.
  fn check_table_header(&self, line_prefix: &str) -> Option<CompletionContext> {
    let trimmed = line_prefix.trim_start();

    // Check for [[ (array of tables header)
    if trimmed.starts_with("[[") {
      let prefix = trimmed[2..].trim_start();
      return Some(CompletionContext::TableHeader {
        prefix: prefix.to_string(),
      });
    }

    // Check for [ (table header)
    if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
      let prefix = trimmed[1..].trim_start();
      // Make sure we're not past the closing bracket
      if !prefix.contains(']') {
        return Some(CompletionContext::TableHeader {
          prefix: prefix.to_string(),
        });
      }
    }

    None
  }

  /// Find the current table path by looking at preceding table headers.
  fn find_current_table(
    &self,
    lines: &[&str],
    current_line: usize,
  ) -> Vec<String> {
    for i in (0..=current_line).rev() {
      let line = lines[i].trim();

      // Skip empty lines and comments
      if line.is_empty() || line.starts_with('#') {
        continue;
      }

      // Check for array of tables header [[table.path]]
      if line.starts_with("[[") && line.ends_with("]]") {
        let path = &line[2..line.len() - 2];
        return path.split('.').map(|s| s.trim().to_string()).collect();
      }

      // Check for table header [table.path]
      if line.starts_with('[') && line.ends_with(']') && !line.starts_with("[[")
      {
        let path = &line[1..line.len() - 1];
        return path.split('.').map(|s| s.trim().to_string()).collect();
      }

      // If we find a key-value pair on this line and we're before it, we might be at root
      if line.contains('=') && i == current_line {
        continue;
      }
    }

    Vec::new() // Root level
  }

  /// Check if we're in a key or value position.
  fn check_key_value_context(
    &self,
    line_prefix: &str,
    current_table: &[String],
  ) -> Option<CompletionContext> {
    let trimmed = line_prefix.trim_start();

    // Skip if this is a comment or table header
    if trimmed.starts_with('#') || trimmed.starts_with('[') {
      return None;
    }

    // Check if we're after an = sign (value context)
    if let Some(eq_pos) = line_prefix.rfind('=') {
      let after_eq = &line_prefix[eq_pos + 1..];
      let trimmed_after = after_eq.trim_start();

      // Extract the key name before the =
      let before_eq = &line_prefix[..eq_pos];
      let key = before_eq.trim().trim_matches('"').trim_matches('\'');

      let mut path = current_table.to_vec();
      path.push(key.to_string());

      // Check if we're in an array context
      if trimmed_after.starts_with('[') {
        // Inside an array
        let array_content = &trimmed_after[1..];
        let prefix = self.extract_array_item_prefix(array_content);
        return Some(CompletionContext::ArrayItem { path, prefix });
      }

      // Regular value context
      let prefix = trimmed_after.trim_matches('"').trim_matches('\'');
      return Some(CompletionContext::Value {
        path,
        prefix: prefix.to_string(),
      });
    }

    // We're in a key context (before = or on a new line)
    let prefix = trimmed.to_string();
    Some(CompletionContext::Key {
      path: current_table.to_vec(),
      prefix,
    })
  }

  /// Extract prefix for array item completion.
  fn extract_array_item_prefix(&self, content: &str) -> String {
    // Find the last comma or opening bracket
    let last_separator = content.rfind(',').map(|i| i + 1).unwrap_or(0);
    let item_content = &content[last_separator..];
    item_content
      .trim()
      .trim_start_matches('"')
      .trim_start_matches('\'')
      .to_string()
  }

  /// Generate completions for table headers.
  fn table_header_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();
    let prefix_lower = prefix.to_lowercase();

    // Standard pyproject.toml sections
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
      if name.to_lowercase().starts_with(&prefix_lower)
        || prefix_lower.is_empty()
      {
        items.push(lsp::CompletionItem {
          label: name.to_string(),
          kind: Some(lsp::CompletionItemKind::MODULE),
          detail: Some(description.to_string()),
          insert_text: Some(name.to_string()),
          ..Default::default()
        });
      }
    }

    // Tool sections from available schemas
    for schema in SCHEMAS {
      if let Some(tool) = schema.tool {
        let full_path = format!("tool.{tool}");
        if full_path.to_lowercase().starts_with(&prefix_lower)
          || prefix_lower.is_empty()
        {
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

  /// Generate completions for keys within a table.
  fn key_completions(
    &self,
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();
    let prefix_lower = prefix.to_lowercase();

    let path_str = path.join(".");

    match path_str.as_str() {
      "" => {
        // Root level
        items.extend(self.root_key_completions(&prefix_lower));
      }
      "project" => {
        items.extend(self.project_key_completions(&prefix_lower));
      }
      "build-system" => {
        items.extend(self.build_system_key_completions(&prefix_lower));
      }
      "tool" => {
        items.extend(self.tool_key_completions(&prefix_lower));
      }
      _ => {
        // Try to get completions from schema
        items.extend(self.schema_key_completions(path, &prefix_lower));
      }
    }

    items
  }

  /// Root level key completions.
  fn root_key_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
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

    self.filter_keys(&keys, prefix)
  }

  /// Project table key completions.
  fn project_key_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
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

    self.filter_keys(&keys, prefix)
  }

  /// Build system key completions.
  fn build_system_key_completions(
    &self,
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let keys = [
      ("requires", "array", "Build dependencies (PEP 508 strings)"),
      ("build-backend", "string", "The build backend to use"),
      (
        "backend-path",
        "array",
        "Paths to add to sys.path for the backend",
      ),
    ];

    self.filter_keys(&keys, prefix)
  }

  /// Tool section key completions.
  fn tool_key_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();

    for schema in SCHEMAS {
      if let Some(tool) = schema.tool {
        if tool.to_lowercase().starts_with(prefix) || prefix.is_empty() {
          items.push(lsp::CompletionItem {
            label: tool.to_string(),
            kind: Some(lsp::CompletionItemKind::PROPERTY),
            detail: Some(format!("{tool} configuration section")),
            insert_text: Some(tool.to_string()),
            ..Default::default()
          });
        }
      }
    }

    items
  }

  /// Get key completions from JSON schema.
  fn schema_key_completions(
    &self,
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();

    // Handle tool-specific schemas specially
    if path.len() >= 2 && path[0] == "tool" {
      let tool_name = &path[1];

      // Find schema for this tool
      for schema in SCHEMAS {
        if schema.tool == Some(tool_name.as_str()) {
          if let Ok(schema_value) =
            serde_json::from_str::<Value>(schema.contents)
          {
            // If we're deeper than [tool.<name>], navigate into the schema
            let sub_path = &path[2..];
            let properties = if sub_path.is_empty() {
              schema_value
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
            } else {
              self.navigate_to_properties(&schema_value, sub_path)
            };

            if let Some(props) = properties {
              for (key, value) in props {
                if key.to_lowercase().starts_with(prefix) || prefix.is_empty() {
                  let description = value
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                  let type_str = self.get_type_string(&value);

                  items.push(lsp::CompletionItem {
                    label: key.clone(),
                    kind: Some(lsp::CompletionItemKind::PROPERTY),
                    detail: Some(type_str),
                    documentation: if description.is_empty() {
                      None
                    } else {
                      Some(lsp::Documentation::MarkupContent(
                        lsp::MarkupContent {
                          kind: lsp::MarkupKind::Markdown,
                          value: description.to_string(),
                        },
                      ))
                    },
                    insert_text: Some(key),
                    ..Default::default()
                  });
                }
              }
            }
          }
          break;
        }
      }

      return items;
    }

    // Build JSON pointer from path
    let pointer = if path.is_empty() {
      String::new()
    } else {
      format!("/{}", path.join("/"))
    };

    // Try to find properties in the schema
    if let Some(properties) = self.get_schema_properties(&pointer) {
      for (key, value) in properties {
        if key.to_lowercase().starts_with(prefix) || prefix.is_empty() {
          let description = value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");

          let type_str = self.get_type_string(&value);

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

  /// Navigate to properties within a schema following a path.
  fn navigate_to_properties(
    &self,
    schema: &Value,
    path: &[String],
  ) -> Option<Map<String, Value>> {
    let mut current = schema.clone();

    for segment in path {
      if let Some(props) = current.get("properties") {
        if let Some(prop) = props.get(segment) {
          current = prop.clone();
          continue;
        }
      }
      if let Some(additional) = current.get("additionalProperties") {
        if additional.is_object() {
          current = additional.clone();
          continue;
        }
      }
      return None;
    }

    current
      .get("properties")
      .and_then(Value::as_object)
      .cloned()
  }

  /// Get properties from schema at a given pointer.
  fn get_schema_properties(&self, pointer: &str) -> Option<Map<String, Value>> {
    // Determine which schema to use based on pointer
    let schema = self.get_schema_for_pointer(pointer)?;

    // Navigate to the properties at the pointer
    let target = if pointer.is_empty() || pointer == "/" {
      schema.clone()
    } else {
      // Remove leading slash and navigate
      let path = pointer.trim_start_matches('/');
      let mut current = schema.clone();

      for segment in path.split('/') {
        // Try properties first
        if let Some(props) = current.get("properties") {
          if let Some(prop) = props.get(segment) {
            current = prop.clone();
            continue;
          }
        }
        // Try additionalProperties
        if let Some(additional) = current.get("additionalProperties") {
          if additional.is_object() {
            current = additional.clone();
            continue;
          }
        }
        return None;
      }

      current
    };

    target.get("properties").and_then(Value::as_object).cloned()
  }

  /// Get the appropriate schema for a pointer path.
  fn get_schema_for_pointer(&self, pointer: &str) -> Option<Value> {
    let path = pointer.trim_start_matches('/');

    if path.starts_with("tool/") || path == "tool" {
      // Extract tool name
      let parts: Vec<&str> = path.split('/').collect();
      if parts.len() >= 2 {
        let tool_name = parts[1];
        // Find schema for this tool
        for schema in SCHEMAS {
          if schema.tool == Some(tool_name) {
            return serde_json::from_str(schema.contents).ok();
          }
        }
      }
    }

    // Return root schema for non-tool paths
    Some(SchemaStore::root().clone())
  }

  /// Get a human-readable type string from schema.
  fn get_type_string(&self, schema: &Value) -> String {
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

  /// Generate completions for values.
  fn value_completions(
    &self,
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let path_str = path.join(".");
    let prefix_lower = prefix.to_lowercase();

    match path_str.as_str() {
      "build-system.build-backend" => {
        self.build_backend_completions(&prefix_lower)
      }
      "project.readme" => self.readme_completions(&prefix_lower),
      "project.license" => self.license_completions(&prefix_lower),
      "project.requires-python" => {
        self.requires_python_completions(&prefix_lower)
      }
      _ => self.schema_value_completions(path, &prefix_lower),
    }
  }

  /// Build backend completions.
  fn build_backend_completions(
    &self,
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
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
      .filter(|(name, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::VALUE),
        detail: Some(desc.to_string()),
        insert_text: Some(format!("\"{name}\"")),
        insert_text_format: Some(lsp::InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
      })
      .collect()
  }

  /// Readme value completions.
  fn readme_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
    let values = [
      ("README.md", "Markdown readme file"),
      ("README.rst", "reStructuredText readme file"),
      ("README.txt", "Plain text readme file"),
    ];

    values
      .iter()
      .filter(|(name, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::FILE),
        detail: Some(desc.to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// License value completions (common SPDX identifiers).
  fn license_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
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
      .filter(|(name, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::VALUE),
        detail: Some(desc.to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// Python version requirement completions.
  fn requires_python_completions(
    &self,
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
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
      .filter(|(name, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::VALUE),
        detail: Some(desc.to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// Schema-based value completions (for enums).
  fn schema_value_completions(
    &self,
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let pointer = if path.is_empty() {
      String::new()
    } else {
      format!("/{}", path.join("/"))
    };

    let Some(schema) = self.get_schema_for_pointer(&pointer) else {
      return Vec::new();
    };

    // Navigate to the schema node
    let path_segments = pointer.trim_start_matches('/');
    let mut current = schema;

    for segment in path_segments.split('/').filter(|s| !s.is_empty()) {
      if let Some(props) = current.get("properties") {
        if let Some(prop) = props.get(segment) {
          current = prop.clone();
          continue;
        }
      }
      return Vec::new();
    }

    // Check for enum values
    if let Some(enum_values) = current.get("enum").and_then(Value::as_array) {
      return enum_values
        .iter()
        .filter_map(|v| {
          let s = match v {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            _ => return None,
          };

          if s.to_lowercase().starts_with(prefix) || prefix.is_empty() {
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
        .collect();
    }

    Vec::new()
  }

  /// Generate completions for array items.
  fn array_item_completions(
    &self,
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let path_str = path.join(".");
    let prefix_lower = prefix.to_lowercase();

    match path_str.as_str() {
      "project.classifiers" => self.classifier_completions(&prefix_lower),
      "project.dynamic" => self.dynamic_field_completions(&prefix_lower),
      "build-system.requires" => self.build_requires_completions(&prefix_lower),
      "project.keywords" => Vec::new(), // No predefined completions
      "project.dependencies" | "project.optional-dependencies" => {
        self.dependency_completions(&prefix_lower)
      }
      _ => self.schema_array_item_completions(path, &prefix_lower),
    }
  }

  /// Classifier completions.
  fn classifier_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
    Self::classifiers()
      .iter()
      .filter(|c| c.to_lowercase().starts_with(prefix) || prefix.is_empty())
      .take(100) // Limit results for performance
      .map(|c| lsp::CompletionItem {
        label: c.to_string(),
        kind: Some(lsp::CompletionItemKind::ENUM_MEMBER),
        insert_text: Some(format!("\"{c}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// Dynamic field completions.
  fn dynamic_field_completions(
    &self,
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
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
      .filter(|f| f.to_lowercase().starts_with(prefix) || prefix.is_empty())
      .map(|f| lsp::CompletionItem {
        label: f.to_string(),
        kind: Some(lsp::CompletionItemKind::ENUM_MEMBER),
        detail: Some("Dynamic field".to_string()),
        insert_text: Some(format!("\"{f}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// Build requires completions.
  fn build_requires_completions(
    &self,
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
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
      .filter(|(name, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::MODULE),
        detail: Some(desc.to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// Common dependency completions.
  fn dependency_completions(&self, prefix: &str) -> Vec<lsp::CompletionItem> {
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
      .filter(|(name, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::MODULE),
        detail: Some(desc.to_string()),
        insert_text: Some(format!("\"{name}\"")),
        ..Default::default()
      })
      .collect()
  }

  /// Schema-based array item completions.
  fn schema_array_item_completions(
    &self,
    path: &[String],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    let pointer = if path.is_empty() {
      String::new()
    } else {
      format!("/{}", path.join("/"))
    };

    let Some(schema) = self.get_schema_for_pointer(&pointer) else {
      return Vec::new();
    };

    // Navigate to the schema node
    let path_segments = pointer.trim_start_matches('/');
    let mut current = schema;

    for segment in path_segments.split('/').filter(|s| !s.is_empty()) {
      if let Some(props) = current.get("properties") {
        if let Some(prop) = props.get(segment) {
          current = prop.clone();
          continue;
        }
      }
      return Vec::new();
    }

    // Check for items schema with enum
    if let Some(items) = current.get("items") {
      if let Some(enum_values) = items.get("enum").and_then(Value::as_array) {
        return enum_values
          .iter()
          .filter_map(|v| {
            let s = match v {
              Value::String(s) => s.clone(),
              _ => return None,
            };

            if s.to_lowercase().starts_with(prefix) || prefix.is_empty() {
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
          .collect();
      }
    }

    Vec::new()
  }

  /// Filter keys by prefix and create completion items.
  fn filter_keys(
    &self,
    keys: &[(&str, &str, &str)],
    prefix: &str,
  ) -> Vec<lsp::CompletionItem> {
    keys
      .iter()
      .filter(|(name, _, _)| {
        name.to_lowercase().starts_with(prefix) || prefix.is_empty()
      })
      .map(|(name, type_str, desc)| lsp::CompletionItem {
        label: name.to_string(),
        kind: Some(lsp::CompletionItemKind::PROPERTY),
        detail: Some(type_str.to_string()),
        documentation: Some(lsp::Documentation::MarkupContent(
          lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: desc.to_string(),
          },
        )),
        insert_text: Some(name.to_string()),
        ..Default::default()
      })
      .collect()
  }

  /// Get all known classifiers.
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
}

/// Represents the completion context at the cursor position.
#[derive(Debug)]
enum CompletionContext {
  /// Inside a table header: [prefix or [[prefix
  TableHeader { prefix: String },
  /// In a key position within a table
  Key { path: Vec<String>, prefix: String },
  /// In a value position after =
  Value { path: Vec<String>, prefix: String },
  /// In an array item context
  ArrayItem { path: Vec<String>, prefix: String },
  /// Unknown/unsupported context
  Unknown,
}

#[cfg(test)]
mod tests {
  use {super::*, indoc::indoc};

  fn completions_at(
    content: &str,
    line: u32,
    character: u32,
  ) -> Vec<lsp::CompletionItem> {
    let document = Document::from(content);
    let position = lsp::Position { line, character };
    let completions = Completions::new(&document, position);
    completions.completions()
  }

  fn completion_labels(items: &[lsp::CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
  }

  #[test]
  fn completes_table_headers() {
    let content = "[";
    let items = completions_at(content, 0, 1);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"project".to_string()));
    assert!(labels.contains(&"build-system".to_string()));
    assert!(labels.contains(&"tool".to_string()));
  }

  #[test]
  fn completes_table_headers_with_prefix() {
    let content = "[pro";
    let items = completions_at(content, 0, 4);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"project".to_string()));
    assert!(!labels.contains(&"tool".to_string()));
  }

  #[test]
  fn completes_tool_table_headers() {
    let content = "[tool.";
    let items = completions_at(content, 0, 6);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"tool.ruff".to_string()));
    assert!(labels.contains(&"tool.black".to_string()));
    assert!(labels.contains(&"tool.mypy".to_string()));
  }

  #[test]
  fn completes_project_keys() {
    let content = indoc! {r#"
      [project]

    "#};
    let items = completions_at(content, 1, 0);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"name".to_string()));
    assert!(labels.contains(&"version".to_string()));
    assert!(labels.contains(&"dependencies".to_string()));
  }

  #[test]
  fn completes_project_keys_with_prefix() {
    let content = indoc! {r#"
      [project]
      de
    "#};
    let items = completions_at(content, 1, 2);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"dependencies".to_string()));
    assert!(labels.contains(&"description".to_string()));
    assert!(!labels.contains(&"name".to_string()));
  }

  #[test]
  fn completes_build_backend_values() {
    let content = indoc! {r#"
      [build-system]
      build-backend =
    "#};
    let items = completions_at(content, 1, 16);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"hatchling.build".to_string()));
    assert!(labels.contains(&"setuptools.build_meta".to_string()));
    assert!(labels.contains(&"flit_core.buildapi".to_string()));
  }

  #[test]
  fn completes_license_values() {
    let content = indoc! {r#"
      [project]
      name = "test"
      license =
    "#};
    let items = completions_at(content, 2, 10);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"MIT".to_string()));
    assert!(labels.contains(&"Apache-2.0".to_string()));
    assert!(labels.contains(&"GPL-3.0-only".to_string()));
  }

  #[test]
  fn completes_classifiers_in_array() {
    let content = indoc! {r#"
      [project]
      name = "test"
      classifiers = ["Development
    "#};
    let items = completions_at(content, 2, 28);
    let labels = completion_labels(&items);

    assert!(
      labels
        .iter()
        .any(|l| l.starts_with("Development Status ::"))
    );
  }

  #[test]
  fn completes_dynamic_fields() {
    let content = indoc! {r#"
      [project]
      name = "test"
      dynamic = ["
    "#};
    let items = completions_at(content, 2, 12);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"version".to_string()));
    assert!(labels.contains(&"description".to_string()));
    assert!(labels.contains(&"readme".to_string()));
  }

  #[test]
  fn completes_build_system_keys() {
    let content = indoc! {r#"
      [build-system]

    "#};
    let items = completions_at(content, 1, 0);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"requires".to_string()));
    assert!(labels.contains(&"build-backend".to_string()));
    assert!(labels.contains(&"backend-path".to_string()));
  }

  #[test]
  fn completes_requires_python() {
    let content = indoc! {r#"
      [project]
      name = "test"
      requires-python =
    "#};
    let items = completions_at(content, 2, 18);
    let labels = completion_labels(&items);

    assert!(labels.contains(&">=3.12".to_string()));
    assert!(labels.contains(&">=3.11,<4".to_string()));
  }

  #[test]
  fn completes_tool_keys() {
    let content = indoc! {r#"
      [tool]

    "#};
    let items = completions_at(content, 1, 0);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"ruff".to_string()));
    assert!(labels.contains(&"black".to_string()));
    assert!(labels.contains(&"mypy".to_string()));
  }

  #[test]
  fn completes_tool_black_keys() {
    let content = indoc! {r#"
      [tool.black]

    "#};
    let items = completions_at(content, 1, 0);
    let labels = completion_labels(&items);

    assert!(labels.contains(&"line-length".to_string()));
    assert!(labels.contains(&"target-version".to_string()));
    assert!(labels.contains(&"skip-string-normalization".to_string()));
  }

  #[test]
  fn completes_empty_returns_all_options() {
    let content = indoc! {r#"
      [project]
      name = "test"
      license = ""
    "#};
    let items = completions_at(content, 2, 11);
    let labels = completion_labels(&items);

    // Should return all license options when prefix is empty
    assert!(!labels.is_empty());
    assert!(labels.contains(&"MIT".to_string()));
  }
}
