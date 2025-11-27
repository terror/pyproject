use super::*;
use crate::config::RuleLevel;

pub(crate) use {
  dependency_groups::DependencyGroupsRule,
  project_classifiers::ProjectClassifiersRule,
  project_dependencies::ProjectDependenciesRule,
  project_dependencies_version_bounds::ProjectDependenciesVersionBoundsRule,
  project_dependency_deprecations::ProjectDependencyDeprecationsRule,
  project_dependency_updates::ProjectDependencyUpdatesRule,
  project_description::ProjectDescriptionRule,
  project_dynamic::ProjectDynamicRule,
  project_entry_points::ProjectEntryPointsRule,
  project_entry_points_extras::ProjectEntryPointsExtrasRule,
  project_import_names::ProjectImportNamesRule,
  project_keywords::ProjectKeywordsRule,
  project_license_classifiers::ProjectLicenseClassifiersRule,
  project_license_classifiers_deprecated::ProjectLicenseClassifiersDeprecatedRule,
  project_license_files::ProjectLicenseFilesRule,
  project_license_value::ProjectLicenseValueRule,
  project_license_value_deprecations::ProjectLicenseValueDeprecationsRule,
  project_name::ProjectNameRule,
  project_optional_dependencies::ProjectOptionalDependenciesRule,
  project_people::ProjectPeopleRule, project_readme::ProjectReadmeRule,
  project_readme_content_type::ProjectReadmeContentTypeRule,
  project_requires_python::ProjectRequiresPythonRule,
  project_requires_python_upper_bound::ProjectRequiresPythonUpperBoundRule,
  project_unknown_keys::ProjectUnknownKeysRule, project_urls::ProjectUrlsRule,
  project_version::ProjectVersionRule, schema::SchemaRule,
  semantic::SemanticRule, syntax::SyntaxRule,
};

mod dependency_groups;
mod project_classifiers;
mod project_dependencies;
mod project_dependencies_version_bounds;
mod project_dependency_deprecations;
mod project_dependency_updates;
mod project_description;
mod project_dynamic;
mod project_entry_points;
mod project_entry_points_extras;
mod project_import_names;
mod project_keywords;
mod project_license_classifiers;
mod project_license_classifiers_deprecated;
mod project_license_files;
mod project_license_value;
mod project_license_value_deprecations;
mod project_name;
mod project_optional_dependencies;
mod project_people;
mod project_readme;
mod project_readme_content_type;
mod project_requires_python;
mod project_requires_python_upper_bound;
mod project_unknown_keys;
mod project_urls;
mod project_version;
mod schema;
mod semantic;
mod syntax;

pub(crate) trait Rule: Sync {
  /// What to show the user in the header of the diagnostics.
  fn display(&self) -> &'static str;

  /// The default severity level for the rule when not configured.
  fn default_level(&self) -> Option<RuleLevel> {
    None
  }

  /// Unique identifier for the rule.
  fn id(&self) -> &'static str;

  /// Execute the rule and return diagnostics.
  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic>;
}
