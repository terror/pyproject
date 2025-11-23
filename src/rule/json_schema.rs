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
  fn array_length(value: &Value) -> Option<usize> {
    value.as_array().map(Vec::len)
  }

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

  fn decode_segment(segment: &str) -> String {
    let mut decoded = String::with_capacity(segment.len());
    let mut chars = segment.chars().peekable();

    while let Some(ch) = chars.next() {
      if ch == '~' {
        match chars.next() {
          Some('0') | None => decoded.push('~'),
          Some('1') => decoded.push('/'),
          Some(other) => {
            decoded.push('~');
            decoded.push(other);
          }
        }
      } else {
        decoded.push(ch);
      }
    }

    decoded
  }

  fn describe_value(value: &Value) -> String {
    match value {
      Value::Null => "null".to_string(),
      Value::Bool(boolean) => format!("boolean {boolean}"),
      Value::Number(number) => {
        if number.is_i64() || number.is_u64() {
          format!("integer {number}")
        } else {
          format!("number {number}")
        }
      }
      Value::String(string) => format!("string \"{string}\""),
      Value::Array(values) => format!("array of length {}", values.len()),
      Value::Object(_) => "object".to_string(),
    }
  }

  fn diagnostic(&self, error: ValidationError) -> lsp::Diagnostic {
    let message = Self::format_validation_error(&error);

    lsp::Diagnostic {
      message,
      range: self.range_for_error(&error),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn dotted_path(pointer: &str) -> String {
    pointer
      .trim_start_matches('/')
      .split('/')
      .filter(|segment| !segment.is_empty())
      .map(Self::decode_segment)
      .collect::<Vec<_>>()
      .join(".")
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

  fn expected_types(kind: &jsonschema::error::TypeKind) -> String {
    match kind {
      jsonschema::error::TypeKind::Single(type_) => type_.to_string(),
      jsonschema::error::TypeKind::Multiple(types) => {
        let mut names = types
          .iter()
          .map(|type_| type_.to_string())
          .collect::<Vec<_>>();

        names.sort();

        if names.len() <= 1 {
          names.join("")
        } else if names.len() == 2 {
          format!("{} or {}", names[0], names[1])
        } else {
          let last = names.pop().unwrap_or_default();
          format!("{}, or {last}", names.join(", "))
        }
      }
    }
  }

  fn format_enum_options(options: &Value) -> String {
    match options {
      Value::Array(values) => values
        .iter()
        .map(Self::format_literal)
        .collect::<Vec<_>>()
        .join(", "),
      _ => options.to_string(),
    }
  }

  fn format_literal(value: &Value) -> String {
    match value {
      Value::Null => "null".to_string(),
      Value::Bool(boolean) => boolean.to_string(),
      Value::Number(number) => number.to_string(),
      Value::String(string) => format!("\"{string}\""),
      Value::Array(_) | Value::Object(_) => value.to_string(),
    }
  }

  fn format_setting(path: &str) -> String {
    if path.is_empty() {
      "value".to_string()
    } else {
      format!("`{path}`")
    }
  }

  fn format_validation_error(error: &ValidationError) -> String {
    let path = Self::dotted_path(error.instance_path.as_str());

    let target = Self::format_setting(&path);

    let message = match &error.kind {
      ValidationErrorKind::AdditionalItems { limit } => {
        let count = Self::array_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} allows at most {limit} items, found {len}")
          }
          None => format!("{target} allows at most {limit} items"),
        }
      }
      ValidationErrorKind::AdditionalProperties { unexpected } => {
        let setting_path = unexpected
          .first()
          .map(|property| Self::join_path_segments(&path, property))
          .filter(|setting| !setting.is_empty())
          .unwrap_or_else(|| path.clone());

        let setting = Self::format_setting(&setting_path);

        format!("unknown setting {setting}")
      }
      ValidationErrorKind::AnyOf => {
        format!("{target} does not match any allowed schema in anyOf")
      }
      ValidationErrorKind::BacktrackLimitExceeded { error } => {
        format!("regex backtracking limit exceeded: {error}")
      }
      ValidationErrorKind::Constant { expected_value } => {
        let expected = Self::format_literal(expected_value);

        format!("{target} must equal {expected}")
      }
      ValidationErrorKind::Contains => {
        format!("{target} must contain at least one item matching the schema")
      }
      ValidationErrorKind::ContentEncoding { content_encoding } => {
        format!("{target} is not valid {content_encoding} content encoding")
      }
      ValidationErrorKind::ContentMediaType { content_media_type } => {
        format!(
          "{target} is not compliant with media type {content_media_type}"
        )
      }
      ValidationErrorKind::Custom { message } => {
        format!("{target}: {message}")
      }
      ValidationErrorKind::Required { property } => {
        let property = Self::value_to_property(property);

        let setting =
          Self::format_setting(&Self::join_path_segments(&path, &property));

        format!("missing required setting {setting}")
      }
      ValidationErrorKind::Type { kind } => {
        let expected = Self::expected_types(kind);

        let actual = Self::describe_value(error.instance.as_ref());

        format!("expected {expected} for {target}, got {actual}")
      }
      ValidationErrorKind::Enum { options } => {
        let options = Self::format_enum_options(options);

        format!("{target} must be one of: {options}")
      }
      ValidationErrorKind::ExclusiveMaximum { limit } => {
        let actual = Self::describe_value(error.instance.as_ref());

        format!("expected a value less than {limit} for {target}, got {actual}")
      }
      ValidationErrorKind::ExclusiveMinimum { limit } => {
        let actual = Self::describe_value(error.instance.as_ref());

        format!(
          "expected a value greater than {limit} for {target}, got {actual}"
        )
      }
      ValidationErrorKind::FalseSchema => {
        format!("no values are allowed for {target}")
      }
      ValidationErrorKind::Format { format } => {
        format!("{target} is not a valid {format}")
      }
      ValidationErrorKind::FromUtf8 { error } => {
        format!("invalid utf-8 data for {target}: {error}")
      }
      ValidationErrorKind::MaxItems { limit } => {
        let count = Self::array_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} allows at most {limit} items, found {len}")
          }
          None => format!("{target} allows at most {limit} items"),
        }
      }
      ValidationErrorKind::Maximum { limit } => {
        let actual = Self::describe_value(error.instance.as_ref());

        format!(
          "expected a value no greater than {limit} for {target}, got {actual}"
        )
      }
      ValidationErrorKind::MaxLength { limit } => {
        let length = Self::string_length(error.instance.as_ref());

        match length {
          Some(len) => {
            format!(
              "{target} must be at most {limit} characters long, found {len}"
            )
          }
          None => {
            format!("{target} must be at most {limit} characters long")
          }
        }
      }
      ValidationErrorKind::MaxProperties { limit } => {
        let count = Self::object_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} allows at most {limit} properties, found {len}")
          }
          None => format!("{target} allows at most {limit} properties"),
        }
      }
      ValidationErrorKind::MinItems { limit } => {
        let count = Self::array_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} must contain at least {limit} items, found {len}")
          }
          None => format!("{target} must contain at least {limit} items"),
        }
      }
      ValidationErrorKind::Minimum { limit } => {
        let actual = Self::describe_value(error.instance.as_ref());

        format!(
          "expected a value no less than {limit} for {target}, got {actual}"
        )
      }
      ValidationErrorKind::MinLength { limit } => {
        let length = Self::string_length(error.instance.as_ref());

        match length {
          Some(len) => {
            format!(
              "{target} must be at least {limit} characters long, found {len}"
            )
          }
          None => format!("{target} must be at least {limit} characters long"),
        }
      }
      ValidationErrorKind::MinProperties { limit } => {
        let count = Self::object_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!(
              "{target} must contain at least {limit} properties, found {len}"
            )
          }
          None => format!("{target} must contain at least {limit} properties"),
        }
      }
      ValidationErrorKind::MultipleOf { multiple_of } => {
        let actual = Self::describe_value(error.instance.as_ref());

        format!(
          "expected a multiple of {multiple_of} for {target}, got {actual}"
        )
      }
      ValidationErrorKind::Not { .. } => {
        format!("{target} must not match the disallowed schema")
      }
      ValidationErrorKind::OneOfMultipleValid => {
        format!("{target} matches multiple schemas in oneOf")
      }
      ValidationErrorKind::OneOfNotValid => {
        format!("{target} does not match any schema in oneOf")
      }
      ValidationErrorKind::Pattern { pattern } => {
        format!("{target} does not match pattern `{pattern}`")
      }
      ValidationErrorKind::PropertyNames { error } => {
        let nested = Self::format_validation_error(error);
        format!("invalid property name in {target}: {nested}")
      }
      ValidationErrorKind::UnevaluatedItems { unexpected }
      | ValidationErrorKind::UnevaluatedProperties { unexpected } => {
        if unexpected.is_empty() {
          format!("unevaluated properties are not allowed in {target}")
        } else {
          let properties = unexpected.join(", ");
          format!(
            "unevaluated properties are not allowed in {target}: {properties}"
          )
        }
      }
      ValidationErrorKind::UniqueItems => {
        format!("items in {target} must be unique")
      }
      ValidationErrorKind::Referencing(error) => {
        format!("schema reference error: {error}")
      }
    };

    Self::lowercase_message(message)
  }

  fn join(parent: &str, segment: &str) -> String {
    if parent.is_empty() {
      format!("/{}", Self::encode_segment(segment))
    } else {
      format!("{}/{}", parent, Self::encode_segment(segment))
    }
  }

  fn join_path_segments(base: &str, segment: &str) -> String {
    if base.is_empty() {
      segment.to_string()
    } else {
      format!("{base}.{segment}")
    }
  }

  fn lowercase_message(message: String) -> String {
    let mut chars = message.chars();

    if let Some(first) = chars.next() {
      let mut lowered = String::with_capacity(message.len());
      lowered.extend(first.to_lowercase());
      lowered.push_str(chars.as_str());
      lowered
    } else {
      message
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

  fn object_length(value: &Value) -> Option<usize> {
    value.as_object().map(Map::len)
  }

  fn pointer_for_error(error: &ValidationError) -> Option<String> {
    match &error.kind {
      ValidationErrorKind::AdditionalProperties { unexpected }
      | ValidationErrorKind::UnevaluatedItems { unexpected }
      | ValidationErrorKind::UnevaluatedProperties { unexpected } => Some(
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

  fn string_length(value: &Value) -> Option<usize> {
    value.as_str().map(|string| string.chars().count())
  }

  fn value_to_property(property: &Value) -> String {
    property
      .as_str()
      .map_or_else(|| property.to_string(), str::to_string)
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
