use {
  crate::{
    analyzer::Analyzer, arguments::Arguments, diagnostic::Diagnostic,
    document::Document, node_ext::NodeExt, pypi_client::PyPiClient,
    rope_ext::RopeExt, rule::*, rule_context::RuleContext, schema::Schema,
    schema_error::SchemaError, schema_pointer::PointerMap,
    schema_retriever::SchemaRetriever, schema_store::SchemaStore,
    schemas::SCHEMAS, server::Server, subcommand::Subcommand,
  },
  anyhow::{Error, anyhow, bail},
  ariadne::{Color, Label, Report, ReportKind, sources},
  clap::Parser,
  env_logger::Env,
  jsonschema::{
    Retrieve, Uri, ValidationError, Validator,
    error::{TypeKind, ValidationErrorKind},
  },
  log::debug,
  mailparse::{MailAddr, addrparse},
  owo_colors::OwoColorize,
  pep440_rs::{Operator, Version},
  pep508_rs::{PackageName, Requirement, VerbatimUrl, VersionOrUrl},
  rayon::prelude::*,
  regex::Regex,
  reqwest::{Error as ReqwestError, blocking::Client as ReqwestClient},
  ropey::Rope,
  rowan::TextRange,
  serde::Deserialize,
  serde_json::{Map, Value, json},
  similar::TextDiff,
  std::{
    backtrace::BacktraceStatus,
    collections::{BTreeMap, HashMap, HashSet},
    env,
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
    process,
    str::FromStr,
    sync::{
      Arc, Mutex, OnceLock,
      atomic::{AtomicBool, Ordering},
    },
    time::Duration,
  },
  taplo::{
    dom::{
      Node,
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

#[cfg(test)]
use {indoc::indoc, range::Range};

mod analyzer;
mod arguments;
mod diagnostic;
mod document;
mod node_ext;
mod pypi_client;
mod range;
mod rope_ext;
mod rule;
mod rule_context;
mod schema;
mod schema_error;
mod schema_pointer;
mod schema_retriever;
mod schema_store;
mod schemas;
mod server;
mod subcommand;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
  let env = Env::default().default_filter_or("info");

  env_logger::Builder::from_env(env).init();

  if let Err(error) = Arguments::parse().run().await {
    eprintln!("error: {error}");

    for (i, error) in error.chain().skip(1).enumerate() {
      if i == 0 {
        eprintln!();
        eprintln!("because:");
      }

      eprintln!("- {error}");
    }

    let backtrace = error.backtrace();

    if backtrace.status() == BacktraceStatus::Captured {
      eprintln!("backtrace:");
      eprintln!("{backtrace}");
    }

    process::exit(1);
  }
}
