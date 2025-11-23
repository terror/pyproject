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

    map.populate(root, "", None);

    (instance, map)
  }

  fn diagnostic(&self, pointer: &str, message: String) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message,
      range: self
        .range_for_pointer(pointer)
        .unwrap_or_else(|| {
          self
            .ranges
            .get("")
            .copied()
            .unwrap_or_else(|| TextRange::empty(TextSize::from(0)))
        })
        .range(&self.document.content),
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

    let validator = match Self::validator() {
      Ok(validator) => validator,
      Err(error) => {
        warn!("failed to build JSON schema validator: {error}");
        return Vec::new();
      }
    };

    let validation_errors =
      validator.iter_errors(&instance).collect::<Vec<_>>();

    match validator.apply(&instance).basic() {
      BasicOutput::Valid(_) => Vec::new(),
      BasicOutput::Invalid(errors) => {
        let errors = errors.into_iter().collect::<Vec<_>>();

        let mut grouped = HashMap::new();
        let mut order = Vec::new();

        for error in errors {
          let pointer = error.instance_location().as_str();

          let message = validation_errors
            .iter()
            .find(|validation_error| {
              validation_error.instance_path.as_str() == pointer
            })
            .map(|validation_error| SchemaError(validation_error).to_string())
            .unwrap_or_else(|| error.error_description().to_string());

          let entry = grouped.entry(pointer.to_string()).or_insert_with(|| {
            order.push(pointer.to_string());
            Vec::new()
          });

          entry.push(message);
        }

        order
          .into_iter()
          .filter_map(|pointer| {
            grouped.remove(&pointer).map(|messages| {
              let message = if messages.len() == 1 {
                messages.into_iter().next().unwrap_or_default()
              } else {
                messages.join("; ")
              };

              pointers.diagnostic(&pointer, message)
            })
          })
          .collect()
      }
    }
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
