use super::*;

define_rule! {
  /// Validates the document against JSON schemas for `pyproject.toml`.
  ///
  /// Uses JSON Schema validation to check tool-specific configuration
  /// sections against their published schemas.
  SchemaRule {
    id: "json-schema",
    message: "schema mismatch",
    run(context) {
      let document = context.document();

      let Ok((instance, pointers)) = SchemaPointer::build(document) else {
        return Vec::new();
      };

      let Ok(validator) = SchemaStore::validator() else {
        return Vec::new();
      };

      validator
        .iter_errors(&instance)
        .map(|error| pointers.diagnostic(error))
        .collect()
    }
  }
}
