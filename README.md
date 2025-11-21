## pyproject

[![CI](https://github.com/terror/pyproject-lsp/actions/workflows/ci.yaml/badge.svg)](https://github.com/terror/pyproject-lsp/actions/workflows/ci.yaml)

**pyproject** is a linter and language server for
[`pyproject.toml`](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
files.

The
[`pyproject.toml`](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
specification has become
[increasingly more complex](https://peps.python.org/pep-0725/) over time.
Although tools apply their own validation rules, there is no standard way to
surface useful configuration errors/warnings directly in an editor before those
tools run. This language server (and linter) provides real-time feedback on
configuration issues as you edit your project file, helping you catch errors
early and maintain clearer, more reliable builds.
