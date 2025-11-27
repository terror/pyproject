use super::*;

pub(crate) struct SchemaRule;

impl Rule for SchemaRule {
  fn id(&self) -> &'static str {
    "json-schema"
  }

  fn message(&self) -> &'static str {
    "schema mismatch"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();

    let Ok((instance, pointers)) = PointerMap::build(document) else {
      return Vec::new();
    };

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
  pub(crate) fn validator() -> Result<&'static Validator> {
    static VALIDATOR: OnceLock<Result<Validator>> = OnceLock::new();

    VALIDATOR
      .get_or_init(|| {
        jsonschema::options()
          .with_retriever(SchemaStore)
          .build(SchemaStore::root())
          .map_err(Error::new)
      })
      .as_ref()
      .map_err(|error| Error::msg(error.to_string()))
  }
}
