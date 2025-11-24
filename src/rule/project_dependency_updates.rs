use super::*;

pub(crate) struct ProjectDependencyUpdatesRule;

impl Rule for ProjectDependencyUpdatesRule {
  fn header(&self) -> &'static str {
    "project.dependencies update reminders"
  }

  fn id(&self) -> &'static str {
    "project-dependency-updates"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(dependencies) = context.get("project.dependencies") else {
      return Vec::new();
    };

    let Some(array) = dependencies.as_array() else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        continue;
      };

      let Ok(requirement) =
        Requirement::<VerbatimUrl>::from_str(string.value())
      else {
        continue;
      };

      let Some(VersionOrUrl::VersionSpecifier(specifiers)) =
        requirement.version_or_url.as_ref()
      else {
        continue;
      };

      if specifiers.is_empty() {
        continue;
      }

      let Some(latest_version) =
        PyPiClient::shared().latest_version(&requirement.name)
      else {
        continue;
      };

      if specifiers.contains(&latest_version) {
        continue;
      }

      diagnostics.push(Diagnostic::new(
        format!(
          "`project.dependencies` entry `{}` excludes the latest release `{}` (current constraint: `{}`)",
          requirement.name, latest_version, specifiers
        ),
        item.span(&document.content),
        lsp::DiagnosticSeverity::WARNING,
      ));
    }

    diagnostics
  }
}
