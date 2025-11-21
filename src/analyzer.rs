use super::*;

static RULES: &[&dyn Rule] = &[&SyntaxRule, &SemanticRule, &ProjectNameRule];

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

#[cfg(test)]
mod tests {
  use {super::*, indoc::indoc, pretty_assertions::assert_eq};

  type Range = (u32, u32, u32, u32);

  fn to_lsp_range(
    (start_line, start_character, end_line, end_character): Range,
  ) -> lsp::Range {
    lsp::Range {
      start: lsp::Position {
        line: start_line,
        character: start_character,
      },
      end: lsp::Position {
        line: end_line,
        character: end_character,
      },
    }
  }

  #[derive(Debug)]
  struct Message<'a> {
    range: Range,
    text: &'a str,
  }

  #[derive(Debug)]
  struct Test {
    document: Document,
    messages: Vec<(Message<'static>, Option<lsp::DiagnosticSeverity>)>,
  }

  impl Test {
    fn error(self, message: Message<'static>) -> Self {
      Self {
        messages: self
          .messages
          .into_iter()
          .chain([(message, Some(lsp::DiagnosticSeverity::ERROR))])
          .collect(),
        ..self
      }
    }

    fn new(content: &str) -> Self {
      Self {
        document: Document::from(lsp::DidOpenTextDocumentParams {
          text_document: lsp::TextDocumentItem {
            uri: lsp::Url::parse("file:///test.just").unwrap(),
            language_id: "just".to_string(),
            version: 1,
            text: content.to_string(),
          },
        }),
        messages: Vec::new(),
      }
    }

    fn run(self) {
      let Test { document, messages } = self;

      let analyzer = Analyzer::new(&document);

      let diagnostics = analyzer.analyze();

      assert_eq!(
        diagnostics.len(),
        messages.len(),
        "Expected diagnostics {:?} but got {:?}",
        messages,
        diagnostics,
      );

      for (diagnostic, (expected_message, expected_severity)) in
        diagnostics.into_iter().zip(messages.into_iter())
      {
        assert_eq!(diagnostic.message, expected_message.text);
        assert_eq!(diagnostic.range, to_lsp_range(expected_message.range));
        assert_eq!(diagnostic.severity, expected_severity);
      }
    }
  }

  #[test]
  fn valid_document_has_no_diagnostics() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      "
    })
    .run();
  }

  #[test]
  fn unexpected_entry() {
    Test::new(indoc! {
      "
      FOO
      "
    })
    .error(Message {
      range: (0, 3, 1, 0),
      text: "expected \"=\"",
    })
    .run();
  }

  #[test]
  fn unexpected_value() {
    Test::new(indoc! {
      "
      FOO =
      "
    })
    .error(Message {
      range: (0, 5, 1, 0),
      text: "expected value",
    })
    .run();
  }

  #[test]
  fn conflicting_keys() {
    Test::new(indoc! {
      "
      [foo]
      foo = \"demo\"
      foo = \"example\"

      [bar]
      bar = \"demo\"
      bar = \"example\"
      "
    })
    .error(Message {
      range: (2, 0, 2, 3),
      text: "conflicting keys: `foo` conflicts with `foo`",
    })
    .error(Message {
      range: (6, 0, 6, 3),
      text: "conflicting keys: `bar` conflicts with `bar`",
    })
    .run();
  }

  #[test]
  fn reopening_table_as_array_requires_array_of_tables() {
    Test::new(indoc! {
      "
      [tool]
      name = \"demo\"

      [[tool]]
      name = \"example\"
      "
    })
    .error(Message {
      range: (0, 1, 0, 5),
      text: "expected array of tables `tool` required by `tool`",
    })
    .run();
  }

  #[test]
  fn reopening_scalar_as_table_requires_table() {
    Test::new(indoc! {
      "
      dependencies = \"demo\"

      [dependencies.packages]
      foo = \"bar\"
      "
    })
    .error(Message {
      range: (0, 0, 0, 12),
      text: "expected table `dependencies` required by `dependencies`",
    })
    .run();
  }

  #[test]
  fn redefining_table_header_conflicts() {
    Test::new(indoc! {
      "
      [tool]
      name = \"demo\"

      [tool]
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (3, 1, 3, 5),
      text: "conflicting keys: `tool` conflicts with `tool`",
    })
    .run();
  }

  #[test]
  fn invalid_escape_sequences() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\\q\"
      "
    })
    .error(Message {
      range: (1, 12, 1, 12),
      text: "invalid escape sequence",
    })
    .run();
  }
}
