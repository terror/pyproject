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
      let started = Instant::now();

      let document = context.document();

      let Ok((instance, pointers)) = SchemaPointer::build(document) else {
        debug!(uri = %document.uri, "failed to build schema instance");
        return Vec::new();
      };

      let Ok(validator) = SchemaStore::validator() else {
        debug!(uri = %document.uri, "schema validator is unavailable");
        return Vec::new();
      };

      let diagnostics = validator
        .iter_errors(&instance)
        .map(|error| pointers.diagnostic(error))
        .collect::<Vec<_>>();

      debug!(
        uri = %document.uri,
        document_bytes = document.content.len_bytes(),
        diagnostic_count = diagnostics.len(),
        elapsed = ?started.elapsed(),
        "validated schema"
      );

      diagnostics
    }
  }
}
