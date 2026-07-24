use {
  anyhow::{Error, anyhow},
  globwalk::GlobWalkerBuilder,
  indoc::indoc,
  jsonschema::{
    Retrieve, Uri, ValidationError, Validator,
    error::{TypeKind, ValidationErrorKind},
  },
  log::{debug, warn},
  mailparse::{MailAddr, addrparse},
  pep440_rs::{Operator, Version, VersionSpecifiers},
  pep508_rs::{ExtraName, PackageName, Requirement, VerbatimUrl, VersionOrUrl},
  pypi_client::PyPiClient,
  rayon::prelude::*,
  re::PROJECT_NAME,
  regex::Regex,
  reqwest::blocking::Client as ReqwestClient,
  ropey::Rope,
  rowan::TextRange,
  rule::*,
  schema::Schema,
  schema_error::SchemaError,
  schema_pointer::SchemaPointer,
  schema_store::SchemaStore,
  schemas::SCHEMAS,
  serde::Deserialize,
  serde_json::{Map, Value, json},
  std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    fmt::{self, Display, Formatter},
    fs, iter,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
      Arc, LazyLock, Mutex, OnceLock,
      atomic::{AtomicBool, Ordering},
    },
    time::Duration,
  },
  taplo::{
    dom::{
      KeyOrIndex, Node,
      error::Error as SemanticError,
      node::{Key, TableKind},
    },
    parser::{Parse, parse},
    syntax::SyntaxElement,
  },
  text_size::TextSize,
  tokio::sync::RwLock,
  tower_lsp::{Client, LanguageServer, LspService, jsonrpc, lsp_types as lsp},
};

pub use {
  analyzer::Analyzer,
  builtin::Builtin,
  builtins::BUILTINS,
  config::{Config, RuleConfig, RuleLevel},
  dependency::Dependency,
  diagnostic::Diagnostic,
  document::Document,
  quickfix::Quickfix,
  quickfixer::Quickfixer,
  resolver::Resolver,
  rope_ext::{Edit, RopeExt},
  rule::Rule,
  rule_context::RuleContext,
  server::Server,
  span::Span,
};

#[cfg(test)]
use {anyhow::bail, into_range::IntoRange};

mod analyzer;
mod builtin;
mod builtins;
mod config;
mod dependency;
mod diagnostic;
mod document;
mod into_range;
mod pypi_client;
mod quickfix;
mod quickfixer;
mod re;
mod resolver;
mod rope_ext;
mod rule;
mod rule_context;
mod schema;
mod schema_error;
mod schema_pointer;
mod schema_store;
mod schemas;
mod server;
mod span;

type Result<T = (), E = Error> = std::result::Result<T, E>;
