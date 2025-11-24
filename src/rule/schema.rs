use super::*;

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

    map.populate(root, String::new(), None);

    (instance, map)
  }

  fn diagnostic(&self, error: ValidationError) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: SchemaError(&error).to_string(),
      range: self.range_for_error(&error),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn empty_range() -> TextRange {
    TextRange::empty(TextSize::from(0))
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

  fn node_range(node: &Node, key: Option<&Key>) -> TextRange {
    let base = node
      .text_ranges(false)
      .next()
      .unwrap_or_else(Self::empty_range);

    match key.and_then(|key| key.text_ranges().next()) {
      Some(key_range) => base.cover(key_range),
      None => base,
    }
  }

  fn pointer_for_error(error: &ValidationError) -> Option<String> {
    match error.kind() {
      ValidationErrorKind::AdditionalProperties { unexpected }
      | ValidationErrorKind::UnevaluatedItems { unexpected }
      | ValidationErrorKind::UnevaluatedProperties { unexpected } => Some(
        PointerMap::join(error.instance_path().as_str(), unexpected.first()?),
      ),
      _ => Some(error.instance_path().as_str().to_string()),
    }
  }

  fn populate(&mut self, node: &Node, pointer: String, key: Option<&Key>) {
    let range = Self::node_range(node, key);

    self.ranges.insert(pointer.clone(), range);

    match node {
      Node::Table(table) => {
        for (entry_key, value) in table.entries().read().iter() {
          self.populate(
            value,
            Self::join(&pointer, entry_key.value()),
            Some(entry_key),
          );
        }
      }
      Node::Array(array) => {
        for (idx, value) in array.items().read().iter().enumerate() {
          self.populate(value, Self::join(&pointer, &idx.to_string()), None);
        }
      }
      _ => {}
    }
  }

  fn range_for_error(&self, error: &ValidationError) -> lsp::Range {
    Self::pointer_for_error(error)
      .map_or_else(
        || {
          self
            .ranges
            .get("")
            .copied()
            .unwrap_or_else(|| TextRange::empty(TextSize::from(0)))
        },
        |pointer| self.range_for_pointer(&pointer),
      )
      .range(&self.document.content)
  }

  fn range_for_pointer(&self, pointer: &str) -> TextRange {
    if let Some(range) = self.ranges.get(pointer) {
      return *range;
    }

    let mut current = pointer;

    while let Some(idx) = current.rfind('/') {
      current = &current[..idx];

      if let Some(range) = self.ranges.get(current) {
        return *range;
      }
    }

    self.root_range()
  }

  fn root_range(&self) -> TextRange {
    self
      .ranges
      .get("")
      .copied()
      .unwrap_or_else(Self::empty_range)
  }
}

pub(crate) struct SchemaRule;

impl Rule for SchemaRule {
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

    let Ok(validator) = Self::validator() else {
      return Vec::new();
    };

    validator
      .iter_errors(&instance)
      .map(|error| pointers.diagnostic(error))
      .collect()
  }
}

impl SchemaRule {
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
