use super::*;

define_rule! {
  ProjectLicenseFilesRule {
    id: "project-license-files",
    message: "invalid `project.license-files` configuration",
    run(context) {
      let Some(license_files) = context.get("project.license-files") else {
        return Vec::new();
      };

      Self::check_license_files(context.document(), &license_files)
    }
  }
}

impl ProjectLicenseFilesRule {
  fn check_license_files(
    document: &Document,
    license_files: &Node,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(array) = license_files.as_array() else {
      diagnostics.push(Diagnostic::error(
        "`project.license-files` must be an array of strings",
        license_files.span(&document.content),
      ));

      return diagnostics;
    };

    let items = array.items().read();

    if items.is_empty() {
      return diagnostics;
    }

    let Some(root) = document.root() else {
      return diagnostics;
    };

    for item in items.iter() {
      let Some(pattern) = item.as_str() else {
        diagnostics.push(Diagnostic::error(
          "`project.license-files` items must be strings",
          item.span(&document.content),
        ));

        continue;
      };

      let pattern_value = pattern.value();

      if pattern_value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
          "`project.license-files` patterns must not be empty",
          item.span(&document.content),
        ));

        continue;
      }

      if let Err(message) = Self::validate_license_files_pattern(pattern_value)
      {
        diagnostics.push(Diagnostic::error(
          format!(
            "invalid `project.license-files` pattern `{pattern_value}`: {message}"
          ),
          item.span(&document.content),
        ));

        continue;
      }

      match Self::matched_files(&root, pattern_value) {
        Ok(matches) if matches.is_empty() => diagnostics.push(
          Diagnostic::error(
            format!(
              "`project.license-files` pattern `{pattern_value}` did not match any files"
            ),
            item.span(&document.content),
          ),
        ),
        Ok(matches) => diagnostics.extend(
          matches
            .into_iter()
            .filter_map(|path| Self::ensure_utf8_file(&path).err().map(|message| {
              Diagnostic::error(
                message,
                item.span(&document.content),
              )
            })),
        ),
        Err(error) => diagnostics.push(Diagnostic::error(
          format!(
            "failed to evaluate `project.license-files` pattern `{pattern_value}`: {error}"
          ),
          item.span(&document.content),
        )),
      }
    }

    diagnostics
  }

  fn ensure_utf8_file(path: &Path) -> Result<(), String> {
    fs::read_to_string(path).map(|_| ()).map_err(|error| {
      format!(
        "license file `{}` must be valid UTF-8 text ({error})",
        path.display()
      )
    })
  }

  fn glob_max_depth(pattern: &str) -> Option<usize> {
    if pattern.contains("**") {
      return None;
    }

    Some(
      pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .count()
        .max(1),
    )
  }

  fn matched_files(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let mut builder =
      GlobWalkerBuilder::from_patterns(root, &[pattern]).follow_links(false);

    if let Some(max_depth) = Self::glob_max_depth(pattern) {
      builder = builder.max_depth(max_depth);
    }

    let walker = builder.build().map_err(|error| error.to_string())?;

    let mut paths = Vec::new();

    for entry in walker {
      paths.push(entry.map_err(|error| error.to_string())?.into_path());
    }

    Ok(paths)
  }

  fn validate_license_files_pattern(pattern: &str) -> Result<(), String> {
    if pattern.starts_with('/') {
      return Err(
        "patterns must be relative; leading `/` is not allowed".into(),
      );
    }

    if pattern.contains('\\') {
      return Err("path delimiter must be `/`, not `\\`".into());
    }

    if pattern.contains("..") {
      return Err("parent directory segments (`..`) are not allowed".into());
    }

    let mut in_brackets = false;

    for (index, character) in pattern.chars().enumerate() {
      match character {
        '[' if !in_brackets => in_brackets = true,
        ']' if in_brackets => in_brackets = false,
        ']' => {
          return Err(format!(
            "unmatched closing `]` at position {}",
            index + 1
          ));
        }
        _ if in_brackets => {
          if !matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.')
          {
            return Err("`[]` character classes may only contain alphanumerics, `_`, `-`, or `.`"
              .into());
          }
        }
        '*' | '?' | '/' | '.' | '-' | '_' => {}
        c if c.is_ascii_alphanumeric() => {}
        c => {
          return Err(format!("character `{c}` is not allowed"));
        }
      }
    }

    if in_brackets {
      return Err("unclosed `[` in pattern".into());
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn validate_license_files_pattern_valid_simple() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_extension() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE.txt"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_subdirectory() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "licenses/MIT.txt"
      ),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_wildcard() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE*"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_question_mark() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE?"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_globstar() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("**/LICENSE"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_character_class() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "LICENSE[0-9].txt"
      ),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_with_underscore_dash() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "LICENSE_MIT-2.0.txt"
      ),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_complex() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "licenses/**/LICENSE*.txt"
      ),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_character_class_alphanumeric() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "file[abc123].txt"
      ),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_character_class_with_underscore() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("file[a_b].txt"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_character_class_with_dash() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("file[a-z].txt"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_valid_character_class_with_dot() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("file[a.b].txt"),
      Ok(())
    );
  }

  #[test]
  fn validate_license_files_pattern_leading_slash() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("/LICENSE"),
      Err("patterns must be relative; leading `/` is not allowed".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_backslash() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "licenses\\LICENSE"
      ),
      Err("path delimiter must be `/`, not `\\`".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_parent_directory() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("../LICENSE"),
      Err("parent directory segments (`..`) are not allowed".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_parent_directory_in_middle() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("foo/../LICENSE"),
      Err("parent directory segments (`..`) are not allowed".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_unmatched_closing_bracket() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE].txt"),
      Err("unmatched closing `]` at position 8".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_unclosed_bracket() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE[abc"),
      Err("unclosed `[` in pattern".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_invalid_character_in_class() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE[a/b].txt"),
      Err("`[]` character classes may only contain alphanumerics, `_`, `-`, or `.`".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_invalid_character_space() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern(
        "LICENSE FILE.txt"
      ),
      Err("character ` ` is not allowed".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_invalid_character_special() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE@.txt"),
      Err("character `@` is not allowed".to_string())
    );
  }

  #[test]
  fn validate_license_files_pattern_invalid_character_hash() {
    assert_eq!(
      ProjectLicenseFilesRule::validate_license_files_pattern("LICENSE#1.txt"),
      Err("character `#` is not allowed".to_string())
    );
  }

  #[test]
  fn glob_max_depth_simple_file() {
    assert_eq!(ProjectLicenseFilesRule::glob_max_depth("LICENSE"), Some(1));
  }

  #[test]
  fn glob_max_depth_with_extension() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("LICENSE.txt"),
      Some(1)
    );
  }

  #[test]
  fn glob_max_depth_one_subdirectory() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("licenses/LICENSE"),
      Some(2)
    );
  }

  #[test]
  fn glob_max_depth_two_subdirectories() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("foo/bar/LICENSE"),
      Some(3)
    );
  }

  #[test]
  fn glob_max_depth_with_trailing_slash() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("licenses/"),
      Some(1)
    );
  }

  #[test]
  fn glob_max_depth_with_leading_slash() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("/licenses/LICENSE"),
      Some(2)
    );
  }

  #[test]
  fn glob_max_depth_with_wildcard() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("licenses/*.txt"),
      Some(2)
    );
  }

  #[test]
  fn glob_max_depth_with_globstar() {
    assert_eq!(ProjectLicenseFilesRule::glob_max_depth("**/LICENSE"), None);
  }

  #[test]
  fn glob_max_depth_globstar_in_middle() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("licenses/**/LICENSE"),
      None
    );
  }

  #[test]
  fn glob_max_depth_globstar_at_end() {
    assert_eq!(ProjectLicenseFilesRule::glob_max_depth("licenses/**"), None);
  }

  #[test]
  fn glob_max_depth_multiple_consecutive_slashes() {
    assert_eq!(
      ProjectLicenseFilesRule::glob_max_depth("foo//bar///LICENSE"),
      Some(3)
    );
  }

  #[test]
  fn glob_max_depth_empty_string() {
    assert_eq!(ProjectLicenseFilesRule::glob_max_depth(""), Some(1));
  }

  #[test]
  fn glob_max_depth_single_slash() {
    assert_eq!(ProjectLicenseFilesRule::glob_max_depth("/"), Some(1));
  }
}
