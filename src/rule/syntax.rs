use super::*;

define_rule! {
  /// Reports TOML syntax errors from the parser.
  SyntaxRule {
    id: "syntax-errors",
    message: "syntax error",
    run(context) {
      context
        .tree()
        .errors
        .clone()
        .into_iter()
        .map(|error| {
          Diagnostic::error(
            error.message.clone(),
            error.span(context.content()),
          )
        })
        .collect()
    }
  }
}
