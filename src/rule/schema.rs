use super::*;

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

    let tree = context.tree().clone().into_dom();

    if tree.validate().is_err() {
      return Vec::new();
    }

    let (instance, pointers) = PointerMap::build(document, &tree);

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
          .with_retriever(SchemaRetriever)
          .build(SchemaStore::root())
          .map_err(Error::new)
      })
      .as_ref()
      .map_err(|error| Error::msg(error.to_string()))
  }
}
