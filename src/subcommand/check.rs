use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Check {
  #[arg(
    value_name = "PATH",
    help = "Path to the pyproject.toml file to check",
    value_hint = clap::ValueHint::FilePath
  )]
  path: Option<PathBuf>,
}

impl Check {
  pub(crate) fn run(self) -> Result<()> {
    let path = match self.path {
      Some(path) => path,
      None => Subcommand::find_pyproject_toml()?,
    };

    let content = fs::read_to_string(&path)?;

    let absolute_path = if path.is_absolute() {
      path.clone()
    } else {
      env::current_dir()?.join(&path)
    };

    let uri = lsp::Url::from_file_path(&absolute_path).map_err(|()| {
      anyhow!("failed to convert `{}` to file url", path.display())
    })?;

    let document = Document::from(lsp::DidOpenTextDocumentParams {
      text_document: lsp::TextDocumentItem {
        language_id: "toml".to_string(),
        text: content.clone(),
        uri,
        version: 1,
      },
    });

    let analyzer = Analyzer::new(&document);

    let mut diagnostics = analyzer.analyze();

    if diagnostics.is_empty() {
      return Ok(());
    }

    diagnostics.sort_by_key(|diagnostic| {
      (
        diagnostic.range.start.line,
        diagnostic.range.start.character,
        diagnostic.range.end.line,
        diagnostic.range.end.character,
      )
    });

    let any_error = diagnostics.iter().any(|diagnostic| {
      matches!(diagnostic.severity, lsp::DiagnosticSeverity::ERROR)
    });

    let source_id = path.to_string_lossy().to_string();

    let mut cache = sources(vec![(source_id.clone(), content.as_str())]);

    let source_len = document.content.len_chars();

    for diagnostic in diagnostics {
      let (kind, color) = Self::severity_to_style(diagnostic.severity)?;

      let start = document
        .content
        .lsp_position_to_char(diagnostic.range.start)
        .min(source_len);

      let end = document
        .content
        .lsp_position_to_char(diagnostic.range.end)
        .min(source_len);

      let (start, end) = (start.min(end), start.max(end));

      let span = (source_id.clone(), start..end);

      let report = Report::build(kind, span.clone())
        .with_message(&diagnostic.header)
        .with_label(
          Label::new(span.clone())
            .with_message(diagnostic.message.trim().to_string())
            .with_color(color),
        );

      let report = report.with_code(diagnostic.id).finish();

      report
        .print(&mut cache)
        .map_err(|error| anyhow!("failed to render diagnostic: {error}"))?;
    }

    if any_error {
      process::exit(1);
    }

    Ok(())
  }

  fn severity_to_style(
    severity: lsp::DiagnosticSeverity,
  ) -> Result<(ReportKind<'static>, Color)> {
    match severity {
      lsp::DiagnosticSeverity::ERROR => {
        Ok((ReportKind::Custom("error", Color::Red), Color::Red))
      }
      lsp::DiagnosticSeverity::WARNING => {
        Ok((ReportKind::Custom("warning", Color::Yellow), Color::Yellow))
      }
      lsp::DiagnosticSeverity::INFORMATION => {
        Ok((ReportKind::Custom("info", Color::Blue), Color::Blue))
      }
      lsp::DiagnosticSeverity::HINT => {
        Ok((ReportKind::Custom("hint", Color::Cyan), Color::Cyan))
      }
      _ => bail!("failed to map unknown severity {severity:?}"),
    }
  }
}
