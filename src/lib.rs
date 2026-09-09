use {
  dependency::{Dependency, DependencyError},
  globwalk::GlobWalkerBuilder,
  indoc::indoc,
  jsonschema::{
    Retrieve, Uri, ValidationError, Validator,
    error::{TypeKind, ValidationErrorKind},
  },
  mailparse::{MailAddr, addrparse},
  pep440_rs::{Operator, Version, VersionSpecifiers},
  pep508_rs::{
    ExtraName, PackageName, Pep508Error, Requirement, VerbatimUrl, VersionOrUrl,
  },
  pypi_client::PyPiClient,
  rayon::prelude::*,
  re::PROJECT_NAME,
  regex::Regex,
  reqwest::blocking::Client as ReqwestClient,
  ropey::Rope,
  schema_error::SchemaError,
  schema_pointer::SchemaPointer,
  schema_store::SchemaStore,
  serde::Deserialize,
  serde_json::{Map, Value, json},
  std::{
    collections::{HashMap, HashSet},
    env,
    fmt::{self, Display, Formatter},
    fs, iter,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, Mutex, OnceLock},
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
  text_size::{TextRange, TextSize},
  tower_lsp::lsp_types as lsp,
  tracing::{debug, warn},
  version_specifiers_ext::VersionSpecifiersExt,
};

pub use {
  analyzer::Analyzer,
  builtin::Builtin,
  builtins::BUILTINS,
  config::{Config, RuleConfig, RuleLevel},
  diagnostic::Diagnostic,
  document::Document,
  error::Error,
  quickfix::Quickfix,
  quickfixer::Quickfixer,
  resolver::Resolver,
  rope_ext::{Edit, RopeExt},
  rule::Rule,
  rule_context::RuleContext,
  schema::Schema,
  schemas::SCHEMAS,
  span::Span,
};

#[cfg(test)]
use into_range::IntoRange;

mod analyzer;
mod builtin;
mod builtins;
mod config;
mod dependency;
mod diagnostic;
mod document;
mod error;
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
mod span;
mod version_specifiers_ext;

type Result<T = (), E = Error> = std::result::Result<T, E>;
