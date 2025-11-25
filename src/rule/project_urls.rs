use super::*;

pub(crate) struct ProjectUrlsRule;

struct UrlLocation {
  display: &'static str,
  path: &'static str,
}

impl Rule for ProjectUrlsRule {
  fn display(&self) -> &'static str {
    "invalid project url(s)"
  }

  fn id(&self) -> &'static str {
    "project-urls"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();

    let mut diagnostics = Vec::new();

    for location in Self::locations() {
      if let Some(urls) = context.get(location.path) {
        diagnostics.extend(Self::validate_table(
          document,
          &urls,
          location.display,
        ));
      }
    }

    diagnostics
  }
}

impl ProjectUrlsRule {
  const MAX_LABEL_LENGTH: usize = 32;

  fn is_browsable_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https")
  }

  fn locations() -> &'static [UrlLocation] {
    &[
      UrlLocation {
        display: "project.urls",
        path: "project.urls",
      },
      UrlLocation {
        display: "tool.flit.metadata.urls",
        path: "tool.flit.metadata.urls",
      },
    ]
  }

  fn validate_label(
    document: &Document,
    key: &Key,
    location: &str,
  ) -> Option<Diagnostic> {
    let label = key.value();

    if label.chars().count() > Self::MAX_LABEL_LENGTH {
      Some(Diagnostic::error(
        format!(
          "`{location}` label `{label}` must be {} characters or fewer",
          Self::MAX_LABEL_LENGTH,
        ),
        key.span(&document.content),
      ))
    } else {
      None
    }
  }

  fn validate_table(
    document: &Document,
    urls: &Node,
    location: &str,
  ) -> Vec<Diagnostic> {
    let Some(table) = urls.as_table() else {
      return vec![Diagnostic::error(
        format!("`{location}` must be a table of string URLs"),
        urls.span(&document.content),
      )];
    };

    let mut diagnostics = Vec::new();

    for (key, value) in table.entries().read().iter() {
      if let Some(diagnostic) = Self::validate_label(document, key, location) {
        diagnostics.push(diagnostic);
      }

      diagnostics.extend(Self::validate_value(
        document,
        key.value(),
        value,
        location,
      ));
    }

    diagnostics
  }

  fn validate_url(
    document: &Document,
    label: &str,
    node: &Node,
    value: &str,
    location: &str,
  ) -> Vec<Diagnostic> {
    match lsp::Url::parse(value) {
      Ok(url) if Self::is_browsable_scheme(url.scheme()) => Vec::new(),
      Ok(_) => vec![Diagnostic::error(
        format!(
          "`{location}` entry `{label}` must use an `http` or `https` URL"
        ),
        node.span(&document.content),
      )],
      Err(error) => vec![Diagnostic::error(
        format!("`{location}` entry `{label}` must be a valid URL: {error}"),
        node.span(&document.content),
      )],
    }
  }

  fn validate_value(
    document: &Document,
    label: &str,
    node: &Node,
    location: &str,
  ) -> Vec<Diagnostic> {
    match node {
      Node::Str(string) => {
        let value = string.value();

        if value.trim().is_empty() {
          vec![Diagnostic::error(
            format!("`{location}` entry `{label}` must not be empty"),
            node.span(&document.content),
          )]
        } else {
          Self::validate_url(document, label, node, value, location)
        }
      }
      _ => vec![Diagnostic::error(
        format!("`{location}` entry `{label}` must be a string URL"),
        node.span(&document.content),
      )],
    }
  }
}
