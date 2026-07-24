use super::*;

macro_rules! define_rule {
  (
    $(#[$doc:meta])*
    $name:ident {
      id: $id:literal,
      message: $message:literal,
      $(default_level: $level:expr,)?
      run($context:ident) $body:block
    }
  ) => {
    pub(crate) struct $name;

    impl Rule for $name {
      fn default_level(&self) -> Option<RuleLevel> {
        define_rule!(@default $( $level )?)
      }

      fn id(&self) -> &'static str {
        $id
      }

      fn message(&self) -> &'static str {
        $message
      }

      fn run(&self, $context: &RuleContext<'_>) -> Vec<Diagnostic> {
        $body
      }
    }

    inventory::submit!(&$name as &dyn Rule);
  };
  (@default $level:expr) => {
    Some($level)
  };
  (@default) => {
    None
  };
}

pub(crate) use schema::SchemaRule;

mod build_system;
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
mod project_name_normalization;
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

inventory::collect!(&'static dyn Rule);

pub trait Rule: Sync {
  /// The default severity level for the rule when not configured.
  fn default_level(&self) -> Option<RuleLevel> {
    None
  }

  /// Unique identifier for the rule.
  fn id(&self) -> &'static str;

  /// What to show the user in the header of the diagnostics.
  fn message(&self) -> &'static str;

  /// Execute the rule and return diagnostics.
  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic>;
}
