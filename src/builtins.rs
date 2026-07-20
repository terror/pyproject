use super::*;

pub(crate) const BUILTINS: &[Builtin<'static>] = &[
  Builtin::Key {
    name: "authors",
    type_name: "array",
    description: indoc! {
      "
      The people or organizations considered the project's authors.

      Each item is an inline table with optional `name` and `email` string
      keys. At least one key must be present.
      "
    },
  },
  Builtin::Key {
    name: "classifiers",
    type_name: "array",
    description: indoc! {
      "
      The Trove classifiers that apply to the project.

      Each item must be a valid classifier string. License classifiers are
      deprecated when the project declares a `license` expression.
      "
    },
  },
  Builtin::Key {
    name: "dependencies",
    type_name: "array",
    description: indoc! {
      "
      The project's runtime dependencies.

      Each item must be a dependency specifier. These dependencies are always
      considered during installation, subject to any environment markers.
      "
    },
  },
  Builtin::Key {
    name: "description",
    type_name: "string",
    description: indoc! {
      "
      A one-line summary of the project.

      This value is published as the distribution's core metadata summary.
      Build tools may reject descriptions containing multiple lines.
      "
    },
  },
  Builtin::Key {
    name: "dynamic",
    type_name: "array",
    description: indoc! {
      "
      Project metadata fields supplied by the build backend instead of this
      file.

      A field cannot be declared both statically and in `dynamic`. The `name`
      field must not be listed here.
      "
    },
  },
  Builtin::Key {
    name: "entry-points",
    type_name: "table",
    description: indoc! {
      "
      Additional entry point groups.

      Each subtable names an entry point group and maps entry point names to
      object references. Do not use `console_scripts` or `gui_scripts` here;
      use `scripts` and `gui-scripts` instead.
      "
    },
  },
  Builtin::Key {
    name: "gui-scripts",
    type_name: "table",
    description: indoc! {
      "
      GUI application entry points.

      Each key is an entry point name and each value is an object reference.
      This table corresponds to the `gui_scripts` entry point group.
      "
    },
  },
  Builtin::Key {
    name: "import-names",
    type_name: "array",
    description: indoc! {
      "
      Import names that the project exclusively provides.

      Each name must be a Python identifier and may be followed by `; private`.
      An empty array represents a distribution with no import names.
      "
    },
  },
  Builtin::Key {
    name: "import-namespaces",
    type_name: "array",
    description: indoc! {
      "
      Import names that the project provides non-exclusively.

      Use this field for namespace packages. An import name must not appear in
      both `import-names` and `import-namespaces`.
      "
    },
  },
  Builtin::Key {
    name: "keywords",
    type_name: "array",
    description: indoc! {
      "
      Search keywords for the project.

      Each item is a string published in the distribution's core metadata.
      "
    },
  },
  Builtin::Key {
    name: "license",
    type_name: "string",
    description: indoc! {
      "
      The SPDX license expression for distribution files built from this
      project.

      The expression should apply to every produced distribution file. The
      legacy table form is deprecated.
      "
    },
  },
  Builtin::Key {
    name: "license-files",
    type_name: "array",
    description: indoc! {
      "
      Glob patterns for license and legal-notice files.

      Patterns are relative to the project root and matching files must be
      included in distribution archives.
      "
    },
  },
  Builtin::Key {
    name: "maintainers",
    type_name: "array",
    description: indoc! {
      "
      The people or organizations maintaining the project.

      Each item is an inline table with optional `name` and `email` string
      keys. At least one key must be present.
      "
    },
  },
  Builtin::Key {
    name: "name",
    type_name: "string",
    description: indoc! {
      "
      The distribution name of the project.

      This field is required when `[project]` is present and cannot be
      dynamically supplied by a build backend.
      "
    },
  },
  Builtin::Key {
    name: "optional-dependencies",
    type_name: "table",
    description: indoc! {
      "
      Optional dependency groups, also known as extras.

      Each key names an extra and maps to an array of dependency specifiers.
      "
    },
  },
  Builtin::Key {
    name: "readme",
    type_name: "string | table",
    description: indoc! {
      "
      The project's full description.

      A string is a UTF-8 path relative to `pyproject.toml`. A table supplies
      either `file` or `text`, plus the required `content-type` field.
      "
    },
  },
  Builtin::Key {
    name: "requires",
    type_name: "array",
    description: indoc! {
      "
      Dependencies required to run the project's build system.

      This is the required key in `[build-system]`. Each item is a dependency
      specifier string.
      "
    },
  },
  Builtin::Key {
    name: "requires-python",
    type_name: "string",
    description: indoc! {
      "
      The Python versions supported by the project.

      The value is a version specifier string published as `Requires-Python`
      core metadata.
      "
    },
  },
  Builtin::Key {
    name: "scripts",
    type_name: "table",
    description: indoc! {
      "
      Console application entry points.

      Each key is an entry point name and each value is an object reference.
      This table corresponds to the `console_scripts` entry point group.
      "
    },
  },
  Builtin::Key {
    name: "urls",
    type_name: "table",
    description: indoc! {
      "
      URLs associated with the project.

      Each key is a user-facing label and each value is a URL string published
      as a `Project-URL` core metadata field.
      "
    },
  },
  Builtin::Key {
    name: "version",
    type_name: "string",
    description: indoc! {
      "
      The version of the project.

      This field is required either here or in `dynamic`. Values should use a
      normalized Python version.
      "
    },
  },
  Builtin::Table {
    name: "build-system",
    description: indoc! {
      "
      Build-system dependencies.

      This table's required `requires` key lists the Python-level dependencies
      needed to execute the build system.

      ```toml
      [build-system]
      requires = [\"setuptools\"]
      ```
      "
    },
  },
  Builtin::Table {
    name: "project",
    description: indoc! {
      "
      Standardized project metadata.

      The `name` field is required when this table is present. Other metadata
      fields may be static, dynamically supplied, or omitted as permitted by
      the specification.
      "
    },
  },
  Builtin::Table {
    name: "project.entry-points",
    description: indoc! {
      "
      Additional entry point groups.

      Each immediate subtable is an entry point group. Nested subtables are not
      permitted.
      "
    },
  },
  Builtin::Table {
    name: "project.gui-scripts",
    description: indoc! {
      "
      GUI application entry points.

      Each entry maps a command name to an importable object reference.
      "
    },
  },
  Builtin::Table {
    name: "project.optional-dependencies",
    description: indoc! {
      "
      Optional dependency groups.

      Each key names an extra and maps to an array of dependency specifiers.
      "
    },
  },
  Builtin::Table {
    name: "project.scripts",
    description: indoc! {
      "
      Console application entry points.

      Each entry maps a command name to an importable object reference.
      "
    },
  },
  Builtin::Table {
    name: "project.urls",
    description: indoc! {
      "
      URLs associated with the project.

      Each key is a user-facing label and each value is a URL string.
      "
    },
  },
  Builtin::Table {
    name: "tool",
    description: indoc! {
      "
      Tool-specific configuration.

      A tool may use `[tool.NAME]` when it owns `NAME` on PyPI. The pyproject
      specification does not define any tool-specific subtables.
      "
    },
  },
  Builtin::Value {
    name: "authors",
    description: indoc! {
      "
      Permit the build backend to provide `project.authors`.

      Do not also define `authors` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "classifiers",
    description: indoc! {
      "
      Permit the build backend to provide `project.classifiers`.

      Do not also define `classifiers` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "dependencies",
    description: indoc! {
      "
      Permit the build backend to provide `project.dependencies`.

      Do not also define `dependencies` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "description",
    description: indoc! {
      "
      Permit the build backend to provide `project.description`.

      Do not also define `description` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "entry-points",
    description: indoc! {
      "
      Permit the build backend to provide `project.entry-points`.

      Do not also define `entry-points` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "gui-scripts",
    description: indoc! {
      "
      Permit the build backend to provide `project.gui-scripts`.

      Do not also define `gui-scripts` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "import-names",
    description: indoc! {
      "
      Permit the build backend to provide `project.import-names`.

      Do not also define `import-names` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "import-namespaces",
    description: indoc! {
      "
      Permit the build backend to provide `project.import-namespaces`.

      Do not also define `import-namespaces` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "keywords",
    description: indoc! {
      "
      Permit the build backend to provide `project.keywords`.

      Do not also define `keywords` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "license",
    description: indoc! {
      "
      Permit the build backend to provide `project.license`.

      Do not also define `license` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "license-files",
    description: indoc! {
      "
      Permit the build backend to provide `project.license-files`.

      Do not also define `license-files` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "maintainers",
    description: indoc! {
      "
      Permit the build backend to provide `project.maintainers`.

      Do not also define `maintainers` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "optional-dependencies",
    description: indoc! {
      "
      Permit the build backend to provide `project.optional-dependencies`.

      Do not also define `optional-dependencies` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "readme",
    description: indoc! {
      "
      Permit the build backend to provide `project.readme`.

      Do not also define `readme` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "requires-python",
    description: indoc! {
      "
      Permit the build backend to provide `project.requires-python`.

      Do not also define `requires-python` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "scripts",
    description: indoc! {
      "
      Permit the build backend to provide `project.scripts`.

      Do not also define `scripts` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "urls",
    description: indoc! {
      "
      Permit the build backend to provide `project.urls`.

      Do not also define `urls` statically in `[project]`.
      "
    },
  },
  Builtin::Value {
    name: "version",
    description: indoc! {
      "
      Permit the build backend to provide `project.version`.

      Do not also define `version` statically in `[project]`.
      "
    },
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn alphabetical_by_kind() {
    fn names<'a>(kind: &str, names: impl Iterator<Item = &'a str>) {
      let names = names.collect::<Vec<_>>();

      for window in names.windows(2) {
        assert!(
          window[0] < window[1],
          "{kind} names out of order in BUILTINS: {:?} before {:?}",
          window[0],
          window[1],
        );
      }
    }

    names(
      "key",
      BUILTINS.iter().filter_map(|builtin| match builtin {
        Builtin::Key { name, .. } => Some(*name),
        _ => None,
      }),
    );
    names(
      "table",
      BUILTINS.iter().filter_map(|builtin| match builtin {
        Builtin::Table { name, .. } => Some(*name),
        _ => None,
      }),
    );
    names(
      "value",
      BUILTINS.iter().filter_map(|builtin| match builtin {
        Builtin::Value { name, .. } => Some(*name),
        _ => None,
      }),
    );
  }
}
