use super::*;

pub(crate) const SCHEMAS: &[Schema] = &[
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
    contents: include_str!("../schemas/partial-setuptools.json"),
    tool: Some("setuptools"),
    url: "https://json.schemastore.org/partial-setuptools.json",
  },
  Schema {
    contents: include_str!("../schemas/partial-setuptools-scm.json"),
    tool: Some("setuptools_scm"),
    url: "https://json.schemastore.org/partial-setuptools-scm.json",
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
    contents: include_str!("../schemas/tombi.json"),
    tool: Some("tombi"),
    url: "https://www.schemastore.org/tombi.json",
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
];
