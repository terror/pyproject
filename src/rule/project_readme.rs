use super::*;

pub(crate) struct ProjectReadmeRule;

impl Rule for ProjectReadmeRule {
  fn message(&self) -> &'static str {
    "invalid `project.readme` configuration"
  }

  fn id(&self) -> &'static str {
    "project-readme"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(readme) = context.get("project.readme") else {
      return Vec::new();
    };

    let document = context.document();

    match &readme {
      Node::Str(string) => {
        Self::check_readme_string(document, string.value(), &readme)
      }
      Node::Table(_) => Self::check_table(document, &readme),
      _ => vec![Diagnostic::error(
        "`project.readme` must be a string or table",
        readme.span(&document.content),
      )],
    }
  }
}

impl ProjectReadmeRule {
  const KNOWN_README_EXTENSIONS: [&'static str; 2] = ["md", "rst"];
  const SUPPORTED_CONTENT_TYPES: [&'static str; 3] =
    ["text/markdown", "text/x-rst", "text/plain"];
  const SUPPORTED_KEYS: [&'static str; 3] = ["file", "text", "content-type"];

  fn check_readme_string(
    document: &Document,
    path: &str,
    node: &Node,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = document
      .validate_relative_path(path, "project.readme", node)
      .err()
      .into_iter()
      .flatten()
      .collect::<Vec<_>>();

    if !Self::has_known_extension(path) {
      diagnostics.push(Diagnostic::error(
        "`project.readme` must point to a `.md` or `.rst` file when specified as a string",
        node.span(&document.content),
      ));
    }

    diagnostics
  }

  fn check_table(document: &Document, readme: &Node) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(table) = readme.as_table() {
      for (key, _) in table.entries().read().iter() {
        if !Self::SUPPORTED_KEYS.contains(&key.value()) {
          diagnostics.push(Diagnostic::error(
            "`project.readme` only supports `file`, `text`, and `content-type` keys",
            key.span(&document.content),
          ));
        }
      }
    }

    let file = readme.try_get("file").ok();
    let text = readme.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(Diagnostic::error(
        "`project.readme` must specify only one of `file` or `text`",
        readme.span(&document.content),
      )),
      (None, None) => diagnostics.push(Diagnostic::error(
        "missing required key `project.readme.file` or `project.readme.text`",
        readme.span(&document.content),
      )),
      _ => {}
    }

    match readme.try_get("content-type") {
      Ok(content_type) => match content_type.as_str() {
        Some(string) => {
          let value = string.value();

          if !Self::is_supported_content_type(value) {
            diagnostics.push(Diagnostic::error(
              "`project.readme.content-type` must be one of `text/markdown`, `text/x-rst`, or `text/plain`",
              content_type.span(&document.content),
            ));
          }
        }
        None => diagnostics.push(Diagnostic::error(
          "`project.readme.content-type` must be a string",
          content_type.span(&document.content),
        )),
      },
      Err(_) => diagnostics.push(Diagnostic::error(
        "missing required key `project.readme.content-type`",
        readme.span(&document.content),
      )),
    }

    if let Some(ref file) = file {
      match file {
        Node::Str(string) => {
          diagnostics.extend(
            document
              .validate_relative_path(string.value(), "project.readme", file)
              .err()
              .into_iter()
              .flatten(),
          );
        }
        _ => diagnostics.push(Diagnostic::error(
          "`project.readme.file` must be a string",
          file.span(&document.content),
        )),
      }
    }

    match text {
      Some(text) if !text.is_str() => {
        diagnostics.push(Diagnostic::error(
          "`project.readme.text` must be a string",
          text.span(&document.content),
        ));
      }
      _ => {}
    }

    diagnostics
  }

  fn has_known_extension(path: &str) -> bool {
    let Some(extension) =
      Path::new(path).extension().and_then(|ext| ext.to_str())
    else {
      return false;
    };

    Self::KNOWN_README_EXTENSIONS
      .iter()
      .any(|known| extension.eq_ignore_ascii_case(known))
  }

  fn is_supported_content_type(content_type: &str) -> bool {
    Self::SUPPORTED_CONTENT_TYPES
      .iter()
      .any(|supported| supported.eq_ignore_ascii_case(content_type))
  }
}
