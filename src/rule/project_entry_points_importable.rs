use super::*;
use std::io::{self, Write};

pub(crate) struct ProjectEntryPointsImportableRule;

#[derive(Debug)]
struct Entry {
  location: String,
  module: String,
  qualname: Option<String>,
  range: lsp::Range,
}

struct Reference {
  module: String,
  qualname: Option<String>,
}

#[derive(serde::Serialize)]
struct EntryProbe<'a> {
  index: usize,
  module: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  qualname: Option<&'a str>,
}

#[derive(Debug, serde::Deserialize)]
struct EntryResult {
  error: Option<String>,
  index: usize,
  isolated_error: Option<String>,
  status: ImportStatus,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ImportStatus {
  Error,
  NeedsCwd,
  Ok,
}

impl Rule for ProjectEntryPointsImportableRule {
  fn display(&self) -> &'static str {
    "unimportable project entry points"
  }

  fn id(&self) -> &'static str {
    "project-entry-points-importable"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();
    let Some(root) = document.root() else {
      return Vec::new();
    };

    let mut entries = Vec::new();

    if let Some(scripts) = context.get("project.scripts") {
      Self::collect_entries(document, "project.scripts", scripts, &mut entries);
    }

    if let Some(gui_scripts) = context.get("project.gui-scripts") {
      Self::collect_entries(
        document,
        "project.gui-scripts",
        gui_scripts,
        &mut entries,
      );
    }

    if entries.is_empty() {
      return Vec::new();
    }

    let Some(results) = Self::check_importable(&entries, &root) else {
      return Vec::new();
    };

    let mut diagnostics = Vec::new();

    for result in results {
      if result.index >= entries.len() {
        continue;
      }

      let entry = &entries[result.index];
      let reference = Self::display_reference(entry);

      match result.status {
        ImportStatus::Ok => {}
        ImportStatus::NeedsCwd => {
          let reason =
            result.isolated_error.as_deref().unwrap_or("import failed");

          diagnostics.push(Diagnostic::warning(
            format!(
              "`{}` target `{reference}` is not importable in isolated mode (without the current working directory on `sys.path`): {reason}",
              entry.location
            ),
            entry.range,
          ));
        }
        ImportStatus::Error => {
          let reason = result
            .error
            .as_deref()
            .or(result.isolated_error.as_deref())
            .unwrap_or("import failed");

          diagnostics.push(Diagnostic::error(
            format!(
              "`{}` target `{reference}` is not importable: {reason}",
              entry.location
            ),
            entry.range,
          ));
        }
      }
    }

    diagnostics
  }
}

impl ProjectEntryPointsImportableRule {
  const IMPORT_CHECK_SCRIPT: &'static str = r#"
import importlib
import inspect
import json
import os
import sys

data = json.load(sys.stdin)
base_path = list(sys.path)
cwd = os.getcwd()
isolated_path = [
    path for path in base_path
    if path and os.path.abspath(path) != cwd
]


def try_import(path, module, qualname):
    sys.path[:] = path

    try:
        module_obj = importlib.import_module(module)
    except Exception as exc:  # pragma: no cover - surfaced to Rust caller
        return False, f"{type(exc).__name__}: {exc}"

    target = module_obj

    if qualname:
        for part in qualname.split('.'):
            try:
                target = inspect.getattr_static(target, part)
            except AttributeError:
                return False, f"missing attribute {part}"
            except Exception as exc:  # pragma: no cover - surfaced to Rust caller
                return False, f"{type(exc).__name__}: {exc}"

    return True, None


results = []

for entry in data:
    ok, isolated_error = try_import(
        isolated_path,
        entry['module'],
        entry.get('qualname'),
    )

    if ok:
        results.append({'index': entry['index'], 'status': 'ok'})
        continue

    ok, default_error = try_import(
        base_path,
        entry['module'],
        entry.get('qualname'),
    )

    if ok:
        results.append({
            'index': entry['index'],
            'status': 'needs-cwd',
            'isolated_error': isolated_error,
        })
        continue

    results.append({
        'index': entry['index'],
        'status': 'error',
        'isolated_error': isolated_error,
        'error': default_error,
    })

json.dump(results, sys.stdout)
"#;

  fn check_importable(
    entries: &[Entry],
    root: &Path,
  ) -> Option<Vec<EntryResult>> {
    let payload = serde_json::to_vec(
      &entries
        .iter()
        .enumerate()
        .map(|(index, entry)| EntryProbe {
          index,
          module: entry.module.as_str(),
          qualname: entry.qualname.as_deref(),
        })
        .collect::<Vec<_>>(),
    )
    .ok()?;

    for candidate in ["python3", "python"] {
      let mut command = process::Command::new(candidate);

      command
        .arg("-c")
        .arg(Self::IMPORT_CHECK_SCRIPT)
        .current_dir(root)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped());

      match command.spawn() {
        Ok(mut child) => {
          let Some(stdin) = child.stdin.as_mut() else {
            continue;
          };

          if stdin.write_all(&payload).is_err() {
            continue;
          }

          match child.wait_with_output() {
            Ok(output) if output.status.success() => {
              if let Ok(results) = serde_json::from_slice(&output.stdout) {
                return Some(results);
              }
            }
            Ok(_) | Err(_) => {}
          }
        }
        Err(error) => {
          if error.kind() == io::ErrorKind::NotFound {
            continue;
          }

          warn!(
            "failed to run `{candidate}` for entry point importability checks: {error}"
          );

          return None;
        }
      }
    }

    warn!(
      "skipping entry point importability checks; no usable Python interpreter found"
    );

    None
  }

  fn collect_entries(
    document: &Document,
    field: &str,
    node: Node,
    entries: &mut Vec<Entry>,
  ) {
    let Some(table) = node.as_table() else {
      return;
    };

    for (key, value) in table.entries().read().iter() {
      let Some(string) = value.as_str() else {
        continue;
      };

      let Some(reference) = Self::parse_reference(string.value()) else {
        continue;
      };

      entries.push(Entry {
        location: format!("{field}.{}", key.value()),
        module: reference.module,
        qualname: reference.qualname,
        range: value.span(&document.content),
      });
    }
  }

  fn display_reference(entry: &Entry) -> String {
    match &entry.qualname {
      Some(qualname) => format!("{}:{qualname}", entry.module),
      None => entry.module.clone(),
    }
  }

  fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();

    let Some(first) = chars.next() else {
      return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
      && chars
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
  }

  fn is_reference(value: &str) -> bool {
    value
      .split('.')
      .all(|segment| !segment.is_empty() && Self::is_identifier(segment))
  }

  fn parse_reference(raw: &str) -> Option<Reference> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
      return None;
    }

    let reference = match trimmed.split_once('[') {
      Some((reference, _extras)) => reference.trim_end(),
      None => trimmed,
    };

    let mut parts = reference.splitn(2, ':').map(str::trim);

    let module = parts.next().unwrap_or_default();
    let qualname = parts.next().filter(|value| !value.is_empty());

    if !Self::is_reference(module)
      || qualname.is_some_and(|value| !Self::is_reference(value))
    {
      return None;
    }

    Some(Reference {
      module: module.to_string(),
      qualname: qualname.map(str::to_string),
    })
  }
}
