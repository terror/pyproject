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

    let (instance, pointers) = Self::instance_with_pointers(&dom);

    let validator = match Self::validator() {
      Ok(validator) => validator,
      Err(error) => {
        warn!("failed to build JSON schema validator: {error}");
        return Vec::new();
      }
    };

    validator
      .iter_errors(&instance)
      .map(|error| Self::diagnostic(document, &pointers, error))
      .collect()
  }
}

impl JsonSchemaRule {
  fn diagnostic(
    document: &Document,
    pointers: &HashMap<String, TextRange>,
    error: ValidationError,
  ) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: error.to_string().to_lowercase(),
      range: Self::range_for_error(document, pointers, &error),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn encode_pointer_segment(segment: &str) -> String {
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

  fn instance_with_pointers(
    root: &Node,
  ) -> (Value, HashMap<String, TextRange>) {
    let instance = serde_json::to_value(root).unwrap_or_else(|error| {
      panic!("failed to convert document to JSON: {error}")
    });

    let mut pointers = HashMap::new();

    Self::populate_pointers(root, "", &mut pointers, None);

    (instance, pointers)
  }

  fn join_pointer(parent: &str, segment: &str) -> String {
    if parent.is_empty() {
      format!("/{}", Self::encode_pointer_segment(segment))
    } else {
      format!("{}/{}", parent, Self::encode_pointer_segment(segment))
    }
  }

  fn lsp_range(document: &Document, range: TextRange) -> lsp::Range {
    lsp::Range {
      start: document.content.byte_to_lsp_position(range.start().into()),
      end: document.content.byte_to_lsp_position(range.end().into()),
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

  fn populate_pointers(
    node: &Node,
    pointer: &str,
    pointers: &mut HashMap<String, TextRange>,
    key: Option<&Key>,
  ) {
    let range = Self::node_range(node, key);

    pointers.insert(pointer.to_string(), range);

    match node {
      Node::Table(table) => {
        table
          .entries()
          .read()
          .iter()
          .for_each(|(entry_key, value)| {
            Self::populate_pointers(
              value,
              &Self::join_pointer(pointer, entry_key.value()),
              pointers,
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
            Self::populate_pointers(
              value,
              &Self::join_pointer(pointer, &idx.to_string()),
              pointers,
              None,
            );
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

  fn range_for_error(
    document: &Document,
    pointers: &HashMap<String, TextRange>,
    error: &ValidationError,
  ) -> lsp::Range {
    if let Some(range) =
      Self::range_for_specific_kind(document, pointers, error)
    {
      return range;
    }

    Self::lsp_range(
      document,
      Self::range_for_pointer(pointers, error.instance_path.as_str())
        .unwrap_or_else(|| TextRange::empty(TextSize::from(0))),
    )
  }

  fn range_for_pointer(
    pointers: &HashMap<String, TextRange>,
    pointer: &str,
  ) -> Option<TextRange> {
    if let Some(range) = pointers.get(pointer) {
      return Some(*range);
    }

    let mut current = pointer;

    while let Some(idx) = current.rfind('/') {
      if idx == 0 {
        return pointers.get("").copied();
      }

      current = &current[..idx];

      if let Some(range) = pointers.get(current) {
        return Some(*range);
      }
    }

    pointers.get("").copied()
  }

  fn range_for_specific_kind(
    document: &Document,
    pointers: &HashMap<String, TextRange>,
    error: &ValidationError,
  ) -> Option<lsp::Range> {
    match &error.kind {
      ValidationErrorKind::AdditionalProperties { unexpected } => {
        let property = unexpected.first()?;

        let pointer =
          Self::join_pointer(error.instance_path.as_str(), property);

        Some(Self::lsp_range(
          document,
          Self::range_for_pointer(pointers, &pointer)?,
        ))
      }
      ValidationErrorKind::Required { .. } => Some(Self::lsp_range(
        document,
        Self::range_for_pointer(pointers, error.instance_path.as_str())?,
      )),
      _ => None,
    }
  }

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
