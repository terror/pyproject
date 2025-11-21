use super::*;

static RULES: &[&dyn Rule] = &[&SyntaxRule, &SemanticRule];

pub(crate) struct Analyzer<'a> {
  document: &'a Document,
}

impl<'a> Analyzer<'a> {
  pub(crate) fn analyze(&self) -> Vec<lsp::Diagnostic> {
    let context = RuleContext::new(self.document);
    RULES.iter().flat_map(|rule| rule.run(&context)).collect()
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }
}
