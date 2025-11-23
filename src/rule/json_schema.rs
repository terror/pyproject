use super::*;

struct SchemaRetriever;

impl Retrieve for SchemaRetriever {
  fn retrieve(
    &self,
    uri: &Uri<String>,
  ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    SchemaStore::documents()
      .get(uri.as_str())
      .cloned()
      .ok_or_else(|| format!("schema not found for `{uri}`").into())
  }
}

struct PointerMap<'a> {
  document: &'a Document,
  ranges: HashMap<String, TextRange>,
}

impl<'a> PointerMap<'a> {
  fn build(document: &'a Document, root: &Node) -> (Value, Self) {
    let instance = serde_json::to_value(root).unwrap_or_else(|error| {
      panic!("failed to convert document to JSON: {error}")
    });

    let mut map = Self {
      document,
      ranges: HashMap::new(),
    };

    map.populate(root, "", None);

    (instance, map)
  }

  fn diagnostic(&self, error: ValidationError) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: error.to_string().to_lowercase(),
      range: self.range_for_error(&error),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn encode_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());

    for ch in segment.chars() {
      match ch {
        '~' => encoded.push_str("~0"),
        '/' => encoded.push_str("~1"),
        _ => encoded.push(ch),
      }
    }

    encoded
  }

  fn join(parent: &str, segment: &str) -> String {
    if parent.is_empty() {
      format!("/{}", Self::encode_segment(segment))
    } else {
      format!("{}/{}", parent, Self::encode_segment(segment))
    }
  }

  fn lsp_range(&self, range: TextRange) -> lsp::Range {
    lsp::Range {
      start: self
        .document
        .content
        .byte_to_lsp_position(range.start().into()),
      end: self
        .document
        .content
        .byte_to_lsp_position(range.end().into()),
    }
  }

  fn node_range(node: &Node, key: Option<&Key>) -> TextRange {
    let mut range = node
      .text_ranges(false)
      .next()
      .unwrap_or_else(|| TextRange::empty(TextSize::from(0)));

    if let Some(key) = key
      && let Some(key_range) = key.text_ranges().next()
    {
      range = range.cover(key_range);
    }

    range
  }

  fn pointer_for_error(error: &ValidationError) -> Option<String> {
    match &error.kind {
      ValidationErrorKind::AdditionalProperties { unexpected } => Some(
        Self::join(error.instance_path.as_str(), unexpected.first()?),
      ),
      ValidationErrorKind::Required { .. } => {
        Some(error.instance_path.as_str().to_string())
      }
      _ => Some(error.instance_path.as_str().to_string()),
    }
  }

  fn populate(&mut self, node: &Node, pointer: &str, key: Option<&Key>) {
    let range = Self::node_range(node, key);

    self.ranges.insert(pointer.to_string(), range);

    match node {
      Node::Table(table) => {
        table
          .entries()
          .read()
          .iter()
          .for_each(|(entry_key, value)| {
            self.populate(
              value,
              &Self::join(pointer, entry_key.value()),
              Some(entry_key),
            );
          });
      }
      Node::Array(array) => {
        array
          .items()
          .read()
          .iter()
          .enumerate()
          .for_each(|(idx, value)| {
            self.populate(value, &Self::join(pointer, &idx.to_string()), None);
          });
      }
      Node::Bool(_)
      | Node::Str(_)
      | Node::Integer(_)
      | Node::Float(_)
      | Node::Date(_)
      | Node::Invalid(_) => {}
    }
  }

  fn range_for_error(&self, error: &ValidationError) -> lsp::Range {
    let range = Self::pointer_for_error(error)
      .and_then(|pointer| self.range_for_pointer(&pointer))
      .unwrap_or_else(|| {
        self
          .ranges
          .get("")
          .copied()
          .unwrap_or_else(|| TextRange::empty(TextSize::from(0)))
      });

    self.lsp_range(range)
  }

  fn range_for_pointer(&self, pointer: &str) -> Option<TextRange> {
    if let Some(range) = self.ranges.get(pointer) {
      return Some(*range);
    }

    let mut current = pointer;

    while let Some(idx) = current.rfind('/') {
      if idx == 0 {
        return self.ranges.get("").copied();
      }

      current = &current[..idx];

      if let Some(range) = self.ranges.get(current) {
        return Some(*range);
      }
    }

    self.ranges.get("").copied()
  }
}

pub(crate) struct JsonSchemaRule;

impl Rule for JsonSchemaRule {
  fn display_name(&self) -> &'static str {
    "JSON Schema Validation"
  }

  fn id(&self) -> &'static str {
    "json-schema"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    let dom = context.tree().clone().into_dom();

    if dom.validate().is_err() {
      return Vec::new();
    }

    let (instance, pointers) = PointerMap::build(document, &dom);

    let validator = match Self::validator() {
      Ok(validator) => validator,
      Err(error) => {
        warn!("failed to build JSON schema validator: {error}");
        return Vec::new();
      }
    };

    validator
      .iter_errors(&instance)
      .map(|error| pointers.diagnostic(error))
      .collect()
  }
}

impl JsonSchemaRule {
  fn validator() -> Result<&'static Validator> {
    static VALIDATOR: OnceLock<Result<Validator>> = OnceLock::new();

    VALIDATOR
      .get_or_init(|| {
        jsonschema::options()
          .with_retriever(SchemaRetriever)
          .build(SchemaStore::root())
          .map_err(Error::new)
      })
      .as_ref()
      .map_err(|error| Error::msg(error.to_string()))
  }
}
