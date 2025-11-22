use super::*;

pub(crate) use {
  project_classifiers::ProjectClassifiersRule,
  project_description::ProjectDescriptionRule,
  project_keywords::ProjectKeywordsRule, project_license::ProjectLicenseRule,
  project_name::ProjectNameRule, project_people::ProjectPeopleRule,
  project_readme::ProjectReadmeRule, project_version::ProjectVersionRule,
  semantic::SemanticRule, syntax::SyntaxRule,
};

mod project_classifiers;
mod project_description;
mod project_keywords;
mod project_license;
mod project_name;
mod project_people;
mod project_readme;
mod project_version;
mod semantic;
mod syntax;

pub(crate) trait Rule: Sync {
  /// Helper to annotate diagnostics with rule information.
  fn diagnostic(&self, diagnostic: lsp::Diagnostic) -> lsp::Diagnostic {
    lsp::Diagnostic {
      code: Some(lsp::NumberOrString::String(self.id().to_string())),
      source: Some(format!("pyproject ({})", self.display_name())),
      ..diagnostic
    }
  }

  /// Human-readable name for the rule.
  fn display_name(&self) -> &'static str;

  /// Unique identifier for the rule.
  fn id(&self) -> &'static str;

  /// Execute the rule and return diagnostics.
  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic>;
}
