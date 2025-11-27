## pyproject

[![release](https://img.shields.io/github/release/terror/pyproject.svg?label=release&style=flat&labelColor=1d1d1d&color=424242&logo=github&logoColor=white)](https://github.com/terror/pyproject/releases/latest)
[![build](https://img.shields.io/github/actions/workflow/status/terror/pyproject/ci.yaml?branch=master&style=flat&labelColor=1d1d1d&color=424242&logo=GitHub%20Actions&logoColor=white&label=build)](https://github.com/terror/pyproject/actions/workflows/ci.yaml)
[![codecov](https://img.shields.io/codecov/c/gh/terror/pyproject?style=flat&labelColor=1d1d1d&color=424242&logo=Codecov&logoColor=white)](https://codecov.io/gh/terror/pyproject)
[![downloads](https://img.shields.io/github/downloads/terror/pyproject/total.svg?style=flat&labelColor=1d1d1d&color=424242&logo=github&logoColor=white)](https://github.com/terror/pyproject/releases)

**pyproject** is a linter and language server for
[`pyproject.toml`](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
files.

<img width="1337" alt="demo" src="screenshot.png" />

The
[`pyproject.toml`](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
specification has become
[increasingly more complex](https://peps.python.org/pep-0725/) over time.
Although tools apply their own validation rules, there is no standard way to
surface useful configuration errors/warnings directly in an editor before those
tools run. This language server (and linter) provides real-time feedback on
configuration issues as you edit your project file, helping you catch errors
early and maintain clearer, more reliable builds.

We currently provide over
[30+ rules](https://github.com/terror/pyproject/tree/master/src/rule) that cover
syntax validation, schema compliance, project metadata (i.e. name, version,
description, etc), dependencies (i.e. PEP 508 format, version bounds, deprecations,
updates), and lots more. The rule system is designed to be easily extended with
custom rules to fit any projects specific needs.

## Installation

`pyproject` should run on any system, including Linux, MacOS, and the BSDs.

The easiest way to install it is by using
[cargo](https://doc.rust-lang.org/cargo/index.html), the Rust package manager:

```bash
cargo install pyproject
```

Otherwise, see below for the complete package list:

#### Cross-platform

<table>
  <thead>
    <tr>
      <th>Package Manager</th>
      <th>Package</th>
      <th>Command</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><a href=https://www.rust-lang.org>Cargo</a></td>
      <td><a href=https://crates.io/crates/pyproject>pyproject</a></td>
      <td><code>cargo install pyproject</code></td>
    </tr>
    <tr>
      <td><a href=https://brew.sh>Homebrew</a></td>
      <td><a href=https://github.com/terror/homebrew-tap>terror/tap/pyproject</a></td>
      <td><code>brew install terror/tap/pyproject</code></td>
    </tr>
    <tr>
      <td><a href=https://github.com/pypa/pip>Pip</a></td>
      <td><a href=https://pypi.org/project/pyproject/>pyproject</a></td>
      <td><code>pip install pyproject</code></td>
    </tr>
  </tbody>
</table>

### Pre-built binaries

Pre-built binaries for Linux, MacOS, and Windows can be found on
[the releases page](https://github.com/terror/pyproject/releases).

## Usage

`pyproject` can be used from the command-line or as a language server.

### CLI

Below is the output of `pyproject --help`:

```present cargo run -- --help
pyproject 0.1.0

Usage: pyproject <COMMAND>

Commands:
  check   Check a pyproject.toml file for errors and warnings [aliases: lint]
  format  Format a pyproject.toml file [aliases: fmt]
  server  Start the language server [aliases: lsp]

Options:
  -h, --help     Print help
  -V, --version  Print version
```

**n.b.** Running `pyproject check` or `pyproject format` on their own will
attempt to perform actions on the nearest `pyproject.toml` file, walking
backwards from the current location.

## Configuration

You can configure rules in your `pyproject.toml` under the `[tool.pyproject]`
section.

Each rule can be set to a severity level (`error`, `warning`, `hint`,
`information` (or `info`), or `off`) using either a simple string or a table
with a `level` field:

```toml
[tool.pyproject.rules]
project-unknown-keys = "warning"
project-dependency-updates = { level = "hint" }
project-requires-python-upper-bound = "off"
```

Rule identifiers are shown in diagnostic output (e.g.,
`error[project-unknown-keys]`). Rules that aren't explicitly configured use
their default severity level.

## Prior Art

This project was inspired by a language server I saw for
[`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html) files,
namely [crates-lsp](https://github.com/MathiasPius/crates-lsp). I couldn't find
similar a tool for
[`pyproject.toml`](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
files, so I thought I'd write one.
