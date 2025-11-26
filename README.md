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

### Neovim

This project is still in its early stages, until there is a published
configuration to [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) and
a package to [Mason](https://github.com/mason-org/mason.nvim), you can configure
it like so:

```lua
local pyproject_binary = '/path/to/pyproject'

if vim.loop.fs_stat(pyproject_binary) then
  vim.lsp.config('pyproject_lsp', {
    on_attach = on_attach, -- Define what to run when the client is attached.
    capabilities = capabilities, -- Define capabilities for the client.
    cmd = { pyproject_binary, 'server' },
    filetypes = { 'pyproject' }, -- The custom filetype set below.
    root_dir = function(bufnr, on_dir)
      local root = vim.fs.root(bufnr, { 'pyproject.toml', '.git' })
      if root then
        on_dir(root)
      end
    end,
    settings = {},
  })
end

vim.api.nvim_create_autocmd({ 'BufRead', 'BufNewFile' }, {
  pattern = 'pyproject.toml',
  callback = function(args)
    vim.bo[args.buf].filetype = 'pyproject'
    vim.lsp.enable('pyproject_lsp', { bufnr = args.buf })
  end,
})
```

This will configure the language server if the binary exists, and enable the language server for a `pyproject` filetype, i.e. a file with the name `pyproject.toml`.

## Prior Art

This project was inspired by a language server I saw for
[`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html) files,
namely [crates-lsp](https://github.com/MathiasPius/crates-lsp). I couldn't find
similar a tool for
[`pyproject.toml`](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
files, so I thought I'd write one.
