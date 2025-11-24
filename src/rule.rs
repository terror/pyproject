use super::*;

pub(crate) use {
  dependency_groups::DependencyGroupsRule,
  project_classifiers::ProjectClassifiersRule,
  project_dependencies::ProjectDependenciesRule,
  project_dependency_updates::ProjectDependencyUpdatesRule,
  project_description::ProjectDescriptionRule,
  project_dynamic::ProjectDynamicRule,
  project_import_names::ProjectImportNamesRule,
  project_keywords::ProjectKeywordsRule,
  project_license_classifiers::ProjectLicenseClassifiersRule,
  project_license_files::ProjectLicenseFilesRule,
  project_license_value::ProjectLicenseValueRule,
  project_name::ProjectNameRule, project_people::ProjectPeopleRule,
  project_readme::ProjectReadmeRule, project_urls::ProjectUrlsRule,
  project_version::ProjectVersionRule, schema::SchemaRule,
  semantic::SemanticRule, syntax::SyntaxRule,
};

mod dependency_groups;
mod project_classifiers;
mod project_dependencies;
mod project_dependency_updates;
mod project_description;
mod project_dynamic;
mod project_import_names;
mod project_keywords;
mod project_license_classifiers;
mod project_license_files;
mod project_license_value;
mod project_name;
mod project_people;
mod project_readme;
mod project_urls;
mod project_version;
mod schema;
mod semantic;
mod syntax;

pub(crate) trait Rule: Sync {
  /// What to show the user in the header of the diagnostics.
  ///
  /// Example: "invalid project version".
  fn header(&self) -> &'static str;

  /// Unique identifier for the rule.
  fn id(&self) -> &'static str;

  /// Execute the rule and return diagnostics.
  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic>;
}
