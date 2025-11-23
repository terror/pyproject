use super::*;

use crate::schema::SchemaStore;
use jsonschema::{
  Retrieve, Uri, ValidationError, Validator, error::ValidationErrorKind,
};
use log::warn;
use serde_json::{Number, Value};
use std::{collections::HashMap, sync::OnceLock};
use taplo::dom::node::IntegerValue;
use text_size::TextSize;

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
    let mut pointers = HashMap::new();
    (Self::node_to_value(root, "", &mut pointers, None), pointers)
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

    if let Some(key) = key {
      if let Some(key_range) = key.text_ranges().next() {
        range = range.cover(key_range);
      }
    }

    range
  }

  fn node_to_value(
    node: &Node,
    pointer: &str,
    pointers: &mut HashMap<String, TextRange>,
    key: Option<&Key>,
  ) -> Value {
    let range = Self::node_range(node, key);

    pointers.insert(pointer.to_string(), range);

    match node {
      Node::Table(table) => {
        let mut map = serde_json::Map::new();

        let entries = table.entries().read();

        for (entry_key, value) in entries.iter() {
          let entry_pointer = Self::join_pointer(pointer, entry_key.value());

          map.insert(
            entry_key.value().to_string(),
            Self::node_to_value(
              value,
              &entry_pointer,
              pointers,
              Some(entry_key),
            ),
          );
        }

        Value::Object(map)
      }
      Node::Array(array) => {
        let items = array.items().read();

        let mut values = Vec::with_capacity(items.len());

        for (idx, value) in items.iter().enumerate() {
          let entry_pointer = Self::join_pointer(pointer, &idx.to_string());

          values.push(Self::node_to_value(
            value,
            &entry_pointer,
            pointers,
            None,
          ));
        }

        Value::Array(values)
      }
      Node::Bool(bool_node) => Value::Bool(bool_node.value()),
      Node::Str(string) => Value::String(string.value().to_string()),
      Node::Integer(integer) => Value::Number(match integer.value() {
        IntegerValue::Negative(value) => Number::from(value),
        IntegerValue::Positive(value) => Number::from(value),
      }),
      Node::Float(float) => Value::Number(
        Number::from_f64(float.value()).unwrap_or_else(|| Number::from(0)),
      ),
      Node::Date(date) => Value::String(date.value().to_string()),
      Node::Invalid(_) => Value::Null,
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

    let text_range =
      Self::range_for_pointer(pointers, error.instance_path.as_str())
        .unwrap_or_else(|| TextRange::empty(TextSize::from(0)));

    Self::lsp_range(document, text_range)
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

        let text_range = Self::range_for_pointer(pointers, &pointer)?;

        Some(Self::lsp_range(document, text_range))
      }
      ValidationErrorKind::Required { .. } => {
        let text_range =
          Self::range_for_pointer(pointers, error.instance_path.as_str())?;

        Some(Self::lsp_range(document, text_range))
      }
      _ => None,
    }
  }

  fn validator() -> Result<&'static Validator> {
    static VALIDATOR: OnceLock<Result<Validator>> = OnceLock::new();

    VALIDATOR
      .get_or_init(|| {
        jsonschema::options()
          .with_retriever(SchemaRetriever)
          .build(SchemaStore::pyproject())
          .map_err(Error::new)
      })
      .as_ref()
      .map_err(|error| Error::msg(error.to_string()))
  }
}
