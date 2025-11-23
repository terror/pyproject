use std::{collections::HashMap, sync::OnceLock};

use serde_json::Value;

const SCHEMA_SOURCES: &[(&str, &str)] = &[
  (
    "https://json.schemastore.org/pyproject.json",
    include_str!("../schemas/pyproject.json"),
  ),
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

  pub(crate) fn pyproject() -> &'static Value {
    Self::documents()
      .get("https://json.schemastore.org/pyproject.json")
      .expect("pyproject schema not found")
  }
}
