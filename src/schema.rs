use super::*;

const SCHEMA_SOURCES: &[(&str, &str)] = &[
  (
    "https://json.schemastore.org/hatch.json",
    include_str!("../schemas/hatch.json"),
  ),
  (
    "https://json.schemastore.org/maturin.json",
    include_str!("../schemas/maturin.json"),
  ),
  (
    "https://json.schemastore.org/partial-black.json",
    include_str!("../schemas/partial-black.json"),
  ),
  (
    "https://json.schemastore.org/partial-cibuildwheel.json",
    include_str!("../schemas/partial-cibuildwheel.json"),
  ),
  (
    "https://json.schemastore.org/partial-mypy.json",
    include_str!("../schemas/partial-mypy.json"),
  ),
  (
    "https://json.schemastore.org/partial-pdm.json",
    include_str!("../schemas/partial-pdm.json"),
  ),
  (
    "https://json.schemastore.org/partial-pdm-dockerize.json",
    include_str!("../schemas/partial-pdm-dockerize.json"),
  ),
  (
    "https://json.schemastore.org/partial-poe.json",
    include_str!("../schemas/partial-poe.json"),
  ),
  (
    "https://json.schemastore.org/partial-poetry.json",
    include_str!("../schemas/partial-poetry.json"),
  ),
  (
    "https://json.schemastore.org/partial-pyright.json",
    include_str!("../schemas/partial-pyright.json"),
  ),
  (
    "https://json.schemastore.org/partial-pytest.json",
    include_str!("../schemas/partial-pytest.json"),
  ),
  (
    "https://json.schemastore.org/partial-repo-review.json",
    include_str!("../schemas/partial-repo-review.json"),
  ),
  (
    "https://json.schemastore.org/partial-scikit-build.json",
    include_str!("../schemas/partial-scikit-build.json"),
  ),
  (
    "https://json.schemastore.org/partial-setuptools-scm.json",
    include_str!("../schemas/partial-setuptools-scm.json"),
  ),
  (
    "https://json.schemastore.org/partial-setuptools.json",
    include_str!("../schemas/partial-setuptools.json"),
  ),
  (
    "https://json.schemastore.org/partial-taskipy.json",
    include_str!("../schemas/partial-taskipy.json"),
  ),
  (
    "https://json.schemastore.org/partial-tox.json",
    include_str!("../schemas/partial-tox.json"),
  ),
  (
    "https://json.schemastore.org/ruff.json",
    include_str!("../schemas/ruff.json"),
  ),
  (
    "https://json.schemastore.org/ty.json",
    include_str!("../schemas/ty.json"),
  ),
  (
    "https://json.schemastore.org/uv.json",
    include_str!("../schemas/uv.json"),
  ),
  (
    "https://www.schemastore.org/tombi.json",
    include_str!("../schemas/tombi.json"),
  ),
];

const TOOL_SCHEMAS: &[(&str, &str)] = &[
  ("black", "https://json.schemastore.org/partial-black.json"),
  (
    "cibuildwheel",
    "https://json.schemastore.org/partial-cibuildwheel.json",
  ),
  ("hatch", "https://json.schemastore.org/hatch.json"),
  ("maturin", "https://json.schemastore.org/maturin.json"),
  ("mypy", "https://json.schemastore.org/partial-mypy.json"),
  ("pdm", "https://json.schemastore.org/partial-pdm.json"),
  ("poe", "https://json.schemastore.org/partial-poe.json"),
  ("poetry", "https://json.schemastore.org/partial-poetry.json"),
  (
    "pyright",
    "https://json.schemastore.org/partial-pyright.json",
  ),
  ("pytest", "https://json.schemastore.org/partial-pytest.json"),
  (
    "repo-review",
    "https://json.schemastore.org/partial-repo-review.json",
  ),
  ("ruff", "https://json.schemastore.org/ruff.json"),
  (
    "scikit-build",
    "https://json.schemastore.org/partial-scikit-build.json",
  ),
  (
    "setuptools",
    "https://json.schemastore.org/partial-setuptools.json",
  ),
  (
    "setuptools_scm",
    "https://json.schemastore.org/partial-setuptools-scm.json",
  ),
  (
    "taskipy",
    "https://json.schemastore.org/partial-taskipy.json",
  ),
  ("tombi", "https://www.schemastore.org/tombi.json"),
  ("tox", "https://json.schemastore.org/partial-tox.json"),
  ("ty", "https://json.schemastore.org/ty.json"),
  ("uv", "https://json.schemastore.org/uv.json"),
];

fn parse_schema(contents: &'static str, name: &str) -> Value {
  serde_json::from_str(contents).unwrap_or_else(|error| {
    panic!("failed to parse bundled schema {name}: {error}")
  })
}

pub(crate) struct SchemaStore;

impl SchemaStore {
  pub(crate) fn documents() -> &'static HashMap<&'static str, Value> {
    static DOCUMENTS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();

    DOCUMENTS.get_or_init(|| {
      SCHEMA_SOURCES
        .iter()
        .map(|(url, contents)| (*url, parse_schema(contents, url)))
        .collect()
    })
  }

  pub(crate) fn root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();

    ROOT.get_or_init(|| {
      let tool_properties = TOOL_SCHEMAS
        .iter()
        .map(|(tool, schema)| ((*tool).to_string(), json!({"$ref": *schema })))
        .collect::<Map<String, Value>>();

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
