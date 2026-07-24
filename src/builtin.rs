use super::*;

#[derive(Clone, Copy, Debug)]
pub enum Builtin<'a> {
  Key {
    name: &'a str,
    type_name: &'a str,
    description: &'a str,
  },
  Table {
    name: &'a str,
    description: &'a str,
  },
  Value {
    name: &'a str,
    description: &'a str,
  },
}

impl Builtin<'_> {
  #[must_use]
  pub fn completion_item(self) -> lsp::CompletionItem {
    let (label, kind, detail, description, quoted) = match self {
      Self::Key {
        name,
        type_name,
        description,
      } => (
        name,
        lsp::CompletionItemKind::PROPERTY,
        type_name,
        description,
        false,
      ),
      Self::Table { name, description } => (
        name,
        lsp::CompletionItemKind::MODULE,
        "table",
        description,
        false,
      ),
      Self::Value { name, description } => (
        name,
        lsp::CompletionItemKind::ENUM_MEMBER,
        description,
        description,
        true,
      ),
    };

    let insert_text = if quoted {
      format!("\"{label}\"")
    } else {
      label.to_string()
    };

    lsp::CompletionItem {
      label: label.to_string(),
      kind: Some(kind),
      detail: Some(detail.to_string()),
      documentation: Some(lsp::Documentation::MarkupContent(
        lsp::MarkupContent {
          kind: lsp::MarkupKind::Markdown,
          value: description.to_string(),
        },
      )),
      insert_text: Some(insert_text),
      ..Default::default()
    }
  }
}
