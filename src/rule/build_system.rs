use super::*;

define_rule! {
  BuildSystemRule {
    id: "build-system",
    message: "invalid `[build-system]` configuration",
    run(context) {
      let Some(build_system) = context.get("build-system") else {
        return Vec::new();
      };

      Self::check_build_system(context, &build_system)
    }
  }
}

impl BuildSystemRule {
  const KEYS: [&str; 3] = ["requires", "build-backend", "backend-path"];

  fn check_backend_path(
    document: &Document,
    content: &Rope,
    node: &Node,
    value: &str,
  ) -> Option<Diagnostic> {
    let path = Path::new(value);

    if path.has_root() {
      return Some(Diagnostic::error(
        "`build-system.backend-path` items must be relative directories",
        node.span(content),
      ));
    }

    let (Some(root), Some(resolved_path)) =
      (document.root(), document.resolve_path(value))
    else {
      return None;
    };

    let root = match root.canonicalize() {
      Ok(root) => root,
      Err(error) => {
        return Some(Diagnostic::error(
          format!(
            "could not resolve project root for `build-system.backend-path`: {error}"
          ),
          node.span(content),
        ));
      }
    };

    let resolved_path = match resolved_path.canonicalize() {
      Ok(resolved_path) => resolved_path,
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
        return Some(Diagnostic::error(
          format!(
            "`build-system.backend-path` directory `{}` does not exist",
            path.display()
          ),
          node.span(content),
        ));
      }
      Err(error) => {
        return Some(Diagnostic::error(
          format!(
            "could not resolve `build-system.backend-path` directory `{}`: {error}",
            path.display()
          ),
          node.span(content),
        ));
      }
    };

    if !resolved_path.starts_with(root) {
      return Some(Diagnostic::error(
        format!(
          "`build-system.backend-path` directory `{}` must be inside the project root",
          path.display()
        ),
        node.span(content),
      ));
    }

    if !resolved_path.is_dir() {
      return Some(Diagnostic::error(
        format!(
          "`build-system.backend-path` item `{}` must be a directory",
          path.display()
        ),
        node.span(content),
      ));
    }

    None
  }

  fn check_backend_paths(
    document: &Document,
    content: &Rope,
    backend_paths: &Node,
  ) -> Vec<Diagnostic> {
    let Some(array) = backend_paths.as_array() else {
      return vec![Diagnostic::error(
        "`build-system.backend-path` must be an array of strings",
        backend_paths.span(content),
      )];
    };

    let mut diagnostics = Vec::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::error(
          "`build-system.backend-path` items must be strings",
          item.span(content),
        ));

        continue;
      };

      if let Some(diagnostic) =
        Self::check_backend_path(document, content, item, string.value())
      {
        diagnostics.push(diagnostic);
      }
    }

    diagnostics
  }

  fn check_build_backend(
    content: &Rope,
    build_backend: &Node,
  ) -> Option<Diagnostic> {
    let Some(string) = build_backend.as_str() else {
      return Some(Diagnostic::error(
        "`build-system.build-backend` must be a string",
        build_backend.span(content),
      ));
    };

    if Self::is_entry_point(string.value()) {
      return None;
    }

    Some(Diagnostic::error(
      "`build-system.build-backend` must be a Python module path optionally followed by `:object.path`",
      build_backend.span(content),
    ))
  }

  fn check_build_system(
    context: &RuleContext<'_>,
    build_system: &Node,
  ) -> Vec<Diagnostic> {
    let content = context.content();
    let Some(table) = build_system.as_table() else {
      return vec![Diagnostic::error(
        "`build-system` must be a table",
        build_system.span(content),
      )];
    };

    let mut diagnostics = table
      .entries()
      .read()
      .iter()
      .filter(|(key, _)| !Self::KEYS.contains(&key.value()))
      .map(|(key, _)| {
        Diagnostic::error(
          format!(
            "`build-system.{}` is not defined by PEP 518 or PEP 517",
            key.value()
          ),
          key.span(content),
        )
      })
      .collect::<Vec<_>>();

    match build_system.try_get("requires") {
      Ok(requires) => {
        diagnostics.extend(Self::check_requires(content, &requires));
      }
      Err(_) => diagnostics.push(Diagnostic::error(
        "missing required key `build-system.requires`",
        build_system.span(content),
      )),
    }

    if let Ok(build_backend) = build_system.try_get("build-backend")
      && let Some(diagnostic) =
        Self::check_build_backend(content, &build_backend)
    {
      diagnostics.push(diagnostic);
    }

    if let Ok(backend_paths) = build_system.try_get("backend-path") {
      diagnostics.extend(Self::check_backend_paths(
        context.document(),
        content,
        &backend_paths,
      ));
    }

    diagnostics
  }

  fn check_requires(content: &Rope, requires: &Node) -> Vec<Diagnostic> {
    let Some(array) = requires.as_array() else {
      return vec![Diagnostic::error(
        "`build-system.requires` must be an array of PEP 508 strings",
        requires.span(content),
      )];
    };

    array
      .items()
      .read()
      .iter()
      .filter_map(|item| {
        let Some(string) = item.as_str() else {
          return Some(Diagnostic::error(
            "`build-system.requires` items must be strings",
            item.span(content),
          ));
        };

        let value = string.value();

        Requirement::<VerbatimUrl>::from_str(value).err().map(|error| {
          Diagnostic::error(
            format!(
              "`build-system.requires` item `{value}` is not a valid PEP 508 dependency: {}",
              error.message.to_string().to_lowercase()
            ),
            item.span(content),
          )
        })
      })
      .collect()
  }

  fn is_entry_point(value: &str) -> bool {
    let (module, object) = value
      .split_once(':')
      .map_or((value, None), |(module, object)| (module, Some(object)));

    Self::is_module_path(module) && object.is_none_or(Self::is_module_path)
  }

  fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();

    let Some(first) = characters.next() else {
      return false;
    };

    (unicode_ident::is_xid_start(first) || first == '_')
      && characters.all(unicode_ident::is_xid_continue)
  }

  fn is_module_path(value: &str) -> bool {
    value.split('.').all(Self::is_identifier)
  }
}
