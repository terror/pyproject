use super::*;

define_rule! {
  SyntaxRule {
    id: "syntax-errors",
    message: "syntax error",
    run(context) {
      let document = context.document();

      context
        .tree()
        .errors
        .clone()
        .into_iter()
        .map(|error| {
          Diagnostic::error(error.message.clone(), error.span(&document.content))
        })
        .collect()
    }
  }
}
