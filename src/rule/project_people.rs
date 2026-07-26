use super::*;

define_rule! {
  /// Validates `project.authors` and `project.maintainers`.
  ///
  /// Both fields must be arrays of inline tables containing at least one valid
  /// `name` or `email`. Names cannot contain commas, and emails must be a single
  /// RFC 5322 address without a display name.
  ProjectPeopleRule {
    id: "project-people",
    message: "invalid project people configuration",
    run(context) {
      let content = context.content();

      let mut diagnostics = Vec::new();

      for (field, people) in [
        ("project.authors", context.get("project.authors")),
        ("project.maintainers", context.get("project.maintainers")),
      ] {
        let Some(people) = people else {
          continue;
        };

        let Some(array) = people.as_array() else {
          diagnostics.push(Diagnostic::error(
            format!("`{field}` must be an array of inline tables"),
            people.span(content),
          ));

          continue;
        };

        for item in array.items().read().iter() {
          let Some(table) = item.as_table() else {
            diagnostics.push(Diagnostic::error(
              format!("`{field}` items must be inline tables"),
              item.span(content),
            ));

            continue;
          };

          if table.kind() != TableKind::Inline {
            diagnostics.push(Diagnostic::error(
              format!("`{field}` items must use inline tables"),
              item.span(content),
            ));
          }

          let entries = table.entries().read();

          if table.kind() == TableKind::Inline && entries.is_empty() {
            diagnostics.push(Diagnostic::error(
              format!(
                "`{field}` items must contain at least one of `name` or `email`"
              ),
              item.span(content),
            ));
          }

          for (key, value) in entries.iter() {
            match key.value() {
              "email" => match value {
                Node::Str(string)
                  if matches!(
                    addrparse(string.value().trim()).as_deref(),
                    Ok(addresses)
                      if matches!(
                        addresses.as_slice(),
                        [MailAddr::Single(single)]
                          if single.display_name.is_none()
                            && !single.addr.trim().is_empty()
                      )
                  ) => {}
                Node::Str(_) => diagnostics.push(Diagnostic::error(
                  format!("`{field}.email` must be a valid email address"),
                  value.span(content),
                )),
                _ => diagnostics.push(Diagnostic::error(
                  format!("`{field}.email` must be a string"),
                  value.span(content),
                )),
              },
              "name" => match value {
                Node::Str(string)
                  if !string.value().trim().is_empty()
                    && !string.value().contains(',')
                    && matches!(
                      addrparse(&format!(
                        "{} <example@example.com>",
                        string.value()
                      ))
                      .as_deref(),
                      Ok(addresses)
                        if matches!(
                          addresses.as_slice(),
                          [MailAddr::Single(single)] if single.display_name.is_some()
                        )
                    ) => {}
                Node::Str(_) => diagnostics.push(Diagnostic::error(
                  format!("`{field}.name` must be a valid email name without commas"),
                  value.span(content),
                )),
                _ => diagnostics.push(Diagnostic::error(
                  format!("`{field}.name` must be a string"),
                  value.span(content),
                )),
              },
              _ => diagnostics.push(Diagnostic::error(
                format!("`{field}` items may only contain `name` or `email`"),
                key.span(content),
              )),
            }
          }
        }
      }

      diagnostics
    }
  }
}
