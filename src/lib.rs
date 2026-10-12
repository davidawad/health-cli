//! health-cli: the source of truth for non-lab personal health data, in a
//! local encrypted FrankenSQLite (fsqlite) database, plus the generator of the
//! Health Dashboard workbook.

#![recursion_limit = "512"]

pub mod audit;
pub mod cli;
pub mod cli_admin;
pub mod commands;
pub mod config;
pub mod container;
pub mod context;
pub mod crypto;
pub mod db;
pub mod enc_config;
pub mod error;
pub mod import_xlsx;
pub mod keyfile;
pub mod keys;
pub mod keysetup;
pub mod migrations;
pub mod output;
pub mod paths;
pub mod perms;
pub mod prompt;
pub mod seal_output;
pub mod sheet;
pub mod sshkey;
pub mod store;
pub mod util;
pub mod vault;

use clap::{CommandFactory, FromArgMatches};

use crate::error::{AppError, ErrorKind};

/// Parse arguments, run the command and return the process exit code.
pub fn run<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let parsed =
        cli::Cli::command().try_get_matches_from(args).and_then(|m| cli::Cli::from_arg_matches(&m).map(|c| (c, m)));
    let (cli, matches) = match parsed {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return if e.use_stderr() { ErrorKind::Usage.exit_code() } else { 0 };
        }
    };
    let is_json = |f: Option<output::Format>| matches!(f, Some(output::Format::Json | output::Format::Jsonl));
    let mut json_errors = is_json(cli.global.format);
    let result = context::Ctx::new(&cli.global).and_then(|ctx| {
        json_errors = is_json(Some(ctx.out.format));
        let r = commands::dispatch(&ctx, cli.command);
        ctx.finish_audit(&command_path(&matches), r.as_ref().map_or_else(AppError::exit_code, |c| *c));
        r
    });
    match result {
        Ok(code) => code,
        Err(e) => {
            if json_errors {
                eprintln!(
                    "{}",
                    serde_json::json!({"schema": output::SCHEMA, "kind": "error", "error": {"kind": e.kind.as_str(), "code": e.exit_code(), "message": e.message}})
                );
            } else {
                eprintln!("health: error: {e}");
            }
            e.exit_code()
        }
    }
}

/// Subcommand path of an invocation, e.g. `db backup` (no arguments: they may hold personal data).
fn command_path(m: &clap::ArgMatches) -> String {
    let mut names = Vec::new();
    let mut cur = m;
    while let Some((name, sub)) = cur.subcommand() {
        names.push(name);
        cur = sub;
    }
    names.join(" ")
}
