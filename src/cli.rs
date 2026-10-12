//! Command-line interface definition (clap derive).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::output::Format;

pub use crate::cli_admin::{AuditCmd, ConfigCmd, DbCmd, KeyCmd};

#[derive(Debug, Parser)]
#[command(
    name = "health",
    version,
    about = "Source of truth for non-lab personal health data",
    long_about = "Source of truth for non-lab personal health data: supplements, products, nutrition, \
        routines, workouts, vaccines, physicals, sizes and resources.\n\n\
        Data lives in an encrypted local FrankenSQLite (fsqlite) database. Every command can \
        emit table, json, jsonl, csv or tsv output; JSON output uses the versioned \
        `health/v1` envelope documented in docs/json-schema.md. `health sheet` generates the \
        Health Dashboard workbook from the store (labs from `biomarker`, genetics from `genome`).",
    after_help = "Exit codes: 0 ok, 1 error, 2 usage, 3 not found, 4 invalid data, 5 database, 6 io, 7 config, 8 key (database locked: missing or wrong encryption key)."
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Args, Default)]
pub struct GlobalOpts {
    /// Config file (default: health-cli/config.toml under $XDG_CONFIG_HOME, ~/.config,
    /// or %APPDATA% on Windows; env HEALTH_CONFIG)
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
    /// Database file (env HEALTH_DB)
    #[arg(long, global = true, value_name = "FILE")]
    pub db: Option<PathBuf>,
    /// Output format
    #[arg(short = 'f', long, global = true, value_enum, ignore_case = true)]
    pub format: Option<Format>,
    /// Write output to FILE instead of stdout
    #[arg(short = 'o', long, global = true, value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Decimal places for text output (table/csv/tsv/markdown/html/org)
    #[arg(long, global = true, value_name = "N")]
    pub precision: Option<String>,
    /// strftime date format for text output (table/csv/tsv/markdown/html/org)
    #[arg(long, global = true, value_name = "FMT")]
    pub date_format: Option<String>,
    /// Time zone: local, UTC, IANA name or +HH:MM
    #[arg(long = "tz", global = true, value_name = "TZ")]
    pub timezone: Option<String>,
    /// Colour: auto, always, never
    #[arg(long, global = true, value_name = "WHEN")]
    pub color: Option<String>,
    /// CSV delimiter character (or 'tab')
    #[arg(long, global = true, value_name = "CHAR")]
    pub delimiter: Option<String>,
    /// CSV quote character
    #[arg(long, global = true, value_name = "CHAR")]
    pub quote: Option<String>,
    /// Omit the header row in csv/tsv output
    #[arg(long, global = true)]
    pub no_header: bool,
    /// Text for missing values in text output (table/csv/tsv/markdown/html/org)
    #[arg(long, global = true, value_name = "TEXT")]
    pub null: Option<String>,
    /// Comma-separated list of columns to output (text formats)
    #[arg(long, global = true, value_name = "COLS", value_delimiter = ',')]
    pub columns: Option<Vec<String>>,
    /// Suppress informational messages
    #[arg(short, long, global = true)]
    pub quiet: bool,
    /// Print extra diagnostics to stderr
    #[arg(short, long, global = true)]
    pub verbose: bool,
    /// Allow an UNENCRYPTED database (create one, or open a legacy plaintext one)
    #[arg(long, global = true)]
    pub insecure_plaintext: bool,
    /// Encrypt --output (or stdout) as an ASCII-armored age file
    #[arg(long, global = true)]
    pub encrypt_output: bool,
    /// age X25519 recipient (age1...) for --encrypt-output (repeatable; default: passphrase)
    #[arg(long = "recipient", global = true, value_name = "AGE_PUBKEY")]
    pub recipients: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Import data from another source
    #[command(subcommand)]
    Import(ImportCmd),
    /// List stored rows of an entity (supplements, products, nutrition, ...)
    List(ListArgs),
    /// Entity row counts of the store
    Stats,
    /// Generate the Health Dashboard workbook (xlsx) from the store
    Sheet(SheetArgs),
    /// Database maintenance: init, backup, rekey, check...
    #[command(subcommand)]
    Db(DbCmd),
    /// Show and change configuration
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Encrypted audit log of commands that touched the database
    #[command(subcommand)]
    Audit(AuditCmd),
    /// Manage the keys that can open the encrypted database
    #[command(subcommand)]
    Key(KeyCmd),
    /// Check encryption, keys, permissions and the audit chain
    Doctor,
    /// Print shell completions
    Completions(CompletionsArgs),
    /// Print man pages
    Man(ManArgs),
}

#[derive(Debug, Subcommand)]
pub enum ImportCmd {
    /// Read a Health Dashboard workbook (read-only) and load every non-lab sheet
    Xlsx(ImportXlsxArgs),
}

#[derive(Debug, Args)]
pub struct ImportXlsxArgs {
    /// The workbook (it is only read; copy it first if you want to be extra careful)
    pub path: PathBuf,
    /// Wipe existing health data and import again
    #[arg(long)]
    pub replace: bool,
    /// Date stamped on every imported price (default: the workbook's modification date)
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub price_date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Entity {
    Facts,
    Predispositions,
    Supplements,
    Doses,
    Sources,
    Evidence,
    Products,
    Nutrition,
    Targets,
    Routine,
    Workout,
    Vaccines,
    VaccineDoses,
    Physicals,
    Providers,
    Sizes,
    Resources,
    Notes,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    pub entity: Entity,
    /// Only rows of this category / section / routine
    #[arg(long)]
    pub category: Option<String>,
}

#[derive(Debug, Args)]
pub struct SheetArgs {
    /// Output workbook (default: the sheet_out setting, outside the repo). Never the original workbook.
    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,
    /// Overwrite an existing file whose name does not contain "(generated)"
    #[arg(long)]
    pub force: bool,
    /// Skip the lab sheets (do not run `biomarker`)
    #[arg(long)]
    pub no_labs: bool,
    /// Skip the Genetics sheet (do not run `genome`)
    #[arg(long)]
    pub no_genetics: bool,
    /// Only these sheets (comma-separated registry names; see --list)
    #[arg(long, value_delimiter = ',', value_name = "NAMES")]
    pub only: Vec<String>,
    /// List the registered sheets and exit
    #[arg(long)]
    pub list: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Nushell,
    Elvish,
    Powershell,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Debug, Args)]
pub struct ManArgs {
    /// Write one page per (sub)command into DIR instead of printing the main page
    #[arg(long, value_name = "DIR")]
    pub dir: Option<PathBuf>,
}
