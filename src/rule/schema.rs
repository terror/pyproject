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

      let validator = match Self::validator(&document.config) {
        Ok(validator) => validator,
        Err(error) => {
          let end = u32::try_from(document.content.len_bytes()).unwrap_or(u32::MAX);

          return vec![Diagnostic::error(
            format!("failed to load schema: {error}"),
            (0, end).span(&document.content),
          )];
        }
      };

      validator
        .iter_errors(&instance)
        .map(|error| pointers.diagnostic(error))
        .collect()
    }
  }
}

impl SchemaRule {
  pub(crate) fn validator(config: &Config) -> Result<SchemaValidator> {
    static VALIDATOR: OnceLock<Result<Validator>> = OnceLock::new();

    if !config.schemas.is_empty() {
      return SchemaStore::validator(config).map(SchemaValidator::Dynamic);
    }

    VALIDATOR
      .get_or_init(SchemaStore::builtin_validator)
      .as_ref()
      .map(SchemaValidator::Builtin)
      .map_err(|error| Error::msg(error.to_string()))
  }
}

pub(crate) enum SchemaValidator {
  Builtin(&'static Validator),
  Dynamic(Validator),
}

impl std::ops::Deref for SchemaValidator {
  type Target = Validator;

  fn deref(&self) -> &Self::Target {
    match self {
      Self::Builtin(validator) => validator,
      Self::Dynamic(validator) => validator,
    }
  }
}
