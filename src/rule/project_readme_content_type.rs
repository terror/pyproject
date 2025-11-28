use super::*;

define_rule! {
  ProjectReadmeContentTypeRule {
    id: "project-readme-content-type",
    message: "suboptimal `project.readme` content type",
    run(context) {
      let Some(readme) = context.get("project.readme") else {
        return Vec::new();
      };

      if readme.as_table().is_none() {
        return Vec::new();
      }

      let Ok(content_type) = readme.try_get("content-type") else {
        return Vec::new();
      };

      let Some(string) = content_type.as_str() else {
        return Vec::new();
      };

      let value = string.value();

      if value.eq_ignore_ascii_case("text/plain") {
        return vec![Diagnostic::warning(
          "`project.readme.content-type` is `text/plain`; consider `text/markdown` or `text/x-rst` for better rendering on package indexes",
          content_type.span(context.content()),
        )];
      }

      Vec::new()
    }
  }
}
