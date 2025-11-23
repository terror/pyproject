use super::*;

#[derive(Debug)]
struct Schema {
  contents: &'static str,
  tool: Option<&'static str>,
  url: &'static str,
}

const SCHEMAS: &[Schema] = &[
  Schema {
    contents: include_str!("../schemas/hatch.json"),
    tool: Some("hatch"),
    url: "https://json.schemastore.org/hatch.json",
  },
  Schema {
    contents: include_str!("../schemas/maturin.json"),
    tool: Some("maturin"),
    url: "https://json.schemastore.org/maturin.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-black.json"),
    tool: Some("black"),
    url: "https://json.schemastore.org/partial-black.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-cibuildwheel.json"),
    tool: Some("cibuildwheel"),
    url: "https://json.schemastore.org/partial-cibuildwheel.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-mypy.json"),
    tool: Some("mypy"),
    url: "https://json.schemastore.org/partial-mypy.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-pdm.json"),
    tool: Some("pdm"),
    url: "https://json.schemastore.org/partial-pdm.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-pdm-dockerize.json"),
    tool: None,
    url: "https://json.schemastore.org/partial-pdm-dockerize.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-poe.json"),
    tool: Some("poe"),
    url: "https://json.schemastore.org/partial-poe.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-poetry.json"),
    tool: Some("poetry"),
    url: "https://json.schemastore.org/partial-poetry.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-pyright.json"),
    tool: Some("pyright"),
    url: "https://json.schemastore.org/partial-pyright.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-pytest.json"),
    tool: Some("pytest"),
    url: "https://json.schemastore.org/partial-pytest.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-repo-review.json"),
    tool: Some("repo-review"),
    url: "https://json.schemastore.org/partial-repo-review.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-scikit-build.json"),
    tool: Some("scikit-build"),
    url: "https://json.schemastore.org/partial-scikit-build.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-setuptools-scm.json"),
    tool: Some("setuptools_scm"),
    url: "https://json.schemastore.org/partial-setuptools-scm.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-setuptools.json"),
    tool: Some("setuptools"),
    url: "https://json.schemastore.org/partial-setuptools.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-taskipy.json"),
    tool: Some("taskipy"),
    url: "https://json.schemastore.org/partial-taskipy.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-tox.json"),
    tool: Some("tox"),
    url: "https://json.schemastore.org/partial-tox.json",
  },
  Schema {
    contents: include_str!("../schemas/ruff.json"),
    tool: Some("ruff"),
    url: "https://json.schemastore.org/ruff.json",
  },
  Schema {
    contents: include_str!("../schemas/ty.json"),
    tool: Some("ty"),
    url: "https://json.schemastore.org/ty.json",
  },
  Schema {
    contents: include_str!("../schemas/uv.json"),
    tool: Some("uv"),
    url: "https://json.schemastore.org/uv.json",
  },
  Schema {
    contents: include_str!("../schemas/tombi.json"),
    tool: Some("tombi"),
    url: "https://www.schemastore.org/tombi.json",
  },
];

pub(crate) struct SchemaStore;

impl SchemaStore {
  pub(crate) fn documents() -> &'static HashMap<&'static str, Value> {
    static DOCUMENTS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();

    DOCUMENTS.get_or_init(|| {
      SCHEMAS
        .iter()
        .map(|schema| (schema.url, Self::parse_schema(schema)))
        .collect()
    })
  }

  fn parse_schema(schema: &Schema) -> Value {
    serde_json::from_str(schema.contents).unwrap_or_else(|error| {
      panic!("failed to parse bundled schema {}: {error}", schema.url)
    })
  }

  pub(crate) fn root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();

    ROOT.get_or_init(|| {
      let tool_properties: Map<String, Value> = SCHEMAS
        .iter()
        .filter_map(|schema| schema.tool.map(|tool| (tool, schema.url)))
        .map(|(tool, url)| (tool.to_string(), json!({ "$ref": url })))
        .collect();

      json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "additionalProperties": true,
        "properties": {
          "tool": {
            "type": "object",
            "additionalProperties": true,
            "properties": tool_properties,
          }
        }
      })
    })
  }
}
