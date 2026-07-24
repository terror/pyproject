use super::*;

pub struct Quickfixer<'a> {
  diagnostics: &'a [Diagnostic],
  parameters: &'a lsp::CodeActionParams,
}

impl<'a> Quickfixer<'a> {
  fn action(
    &self,
    source: &Diagnostic,
    quickfix: &Quickfix,
  ) -> lsp::CodeActionOrCommand {
    let diagnostics = self
      .parameters
      .context
      .diagnostics
      .iter()
      .filter(|diagnostic| {
        diagnostic.range == source.range
          && matches!(
            &diagnostic.code,
            Some(lsp::NumberOrString::String(value)) if value == &source.id
          )
      })
      .cloned()
      .collect::<Vec<_>>();

    lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
      title: quickfix.title.clone(),
      kind: Some(lsp::CodeActionKind::QUICKFIX),
      diagnostics: (!diagnostics.is_empty()).then_some(diagnostics),
      edit: Some(lsp::WorkspaceEdit {
        changes: Some(HashMap::from([(
          self.parameters.text_document.uri.clone(),
          quickfix.edits.clone(),
        )])),
        ..Default::default()
      }),
      ..Default::default()
    })
  }

  #[must_use]
  pub fn collect(&self) -> Vec<lsp::CodeActionOrCommand> {
    self
      .diagnostics
      .iter()
      .filter(|diagnostic| {
        diagnostic.range.start <= self.parameters.range.end
          && self.parameters.range.start <= diagnostic.range.end
      })
      .filter_map(|diagnostic| {
        diagnostic
          .quickfix
          .as_ref()
          .map(|quickfix| self.action(diagnostic, quickfix))
      })
      .collect()
  }

  #[must_use]
  pub fn new(
    parameters: &'a lsp::CodeActionParams,
    diagnostics: &'a [Diagnostic],
  ) -> Self {
    Self {
      diagnostics,
      parameters,
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, crate::into_range::IntoRange, pretty_assertions::assert_eq};

  fn actions(
    parameters: &lsp::CodeActionParams,
    document: &Document,
  ) -> Vec<lsp::CodeActionOrCommand> {
    Quickfixer::new(parameters, &Analyzer::new(document).analyze()).collect()
  }

  #[test]
  fn returns_project_name_normalization_replacement() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "My_Package"
      version = "1.0.0"

      [tool.pyproject.rules]
      project-name-normalization = "warning"
      "#
    });

    let parameters = lsp::CodeActionParams {
      text_document: lsp::TextDocumentIdentifier {
        uri: document.uri.clone(),
      },
      range: (1, 8, 1, 18).range(),
      context: lsp::CodeActionContext::default(),
      work_done_progress_params: lsp::WorkDoneProgressParams::default(),
      partial_result_params: lsp::PartialResultParams::default(),
    };

    assert_eq!(
      actions(&parameters, &document),
      vec![lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        title: "Replace `My_Package` with `my-package`".to_string(),
        kind: Some(lsp::CodeActionKind::QUICKFIX),
        edit: Some(lsp::WorkspaceEdit {
          changes: Some(HashMap::from([(
            document.uri,
            vec![lsp::TextEdit {
              range: (1, 8, 1, 18).range(),
              new_text: "my-package".to_string(),
            }],
          )])),
          ..Default::default()
        }),
        ..Default::default()
      })]
    );
  }
}
