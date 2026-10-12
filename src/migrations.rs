//! Versioned schema migrations.
//!
//! Each migration runs once, inside a transaction, and is recorded in
//! `schema_migrations`. `PRAGMA user_version` mirrors the latest version so
//! external tools can inspect it cheaply.

use crate::db::{int, text, Db, RowExt};
use crate::error::Result;

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub apply: fn(&Db) -> Result<()>,
}

/// v1: everything the original Health Dashboard workbook held outside labs
/// and genetics. Prices carry a currency and the date they were last seen;
/// derived values ($/yr, weekly dose, totals, ages) are never stored.
const SCHEMA_V1: &str = r"
CREATE TABLE import_batch (
    id          INTEGER PRIMARY KEY,
    source_name TEXT NOT NULL,
    sha256      TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    counts      TEXT NOT NULL
);
CREATE TABLE note (
    id       INTEGER PRIMARY KEY,
    scope    TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    text     TEXT NOT NULL
);
CREATE TABLE profile_fact (
    key      TEXT PRIMARY KEY,
    label    TEXT NOT NULL,
    value    TEXT,
    kind     TEXT NOT NULL DEFAULT 'text',
    unit     TEXT,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE predisposition (
    id          INTEGER PRIMARY KEY,
    position    INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    name        TEXT NOT NULL,
    description TEXT,
    prevalence  TEXT
);
CREATE TABLE source (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE,
    url   TEXT,
    notes TEXT
);
CREATE TABLE supplement (
    id             INTEGER PRIMARY KEY,
    position       INTEGER NOT NULL,
    name           TEXT NOT NULL UNIQUE,
    kind           TEXT,
    notes          TEXT,
    experience     TEXT,
    units_per_year REAL,
    price          REAL,
    price_currency TEXT NOT NULL DEFAULT 'USD',
    price_date     TEXT,
    price_note     TEXT,
    buy_url        TEXT
);
CREATE TABLE dose_schedule (
    id            INTEGER PRIMARY KEY,
    supplement_id INTEGER NOT NULL REFERENCES supplement(id) ON DELETE CASCADE,
    taking        INTEGER,
    frequency     TEXT,
    time_of_day   TEXT,
    amount        REAL,
    amount_text   TEXT,
    unit          TEXT
);
CREATE TABLE supplement_source (
    supplement_id INTEGER NOT NULL REFERENCES supplement(id) ON DELETE CASCADE,
    source_id     INTEGER NOT NULL REFERENCES source(id) ON DELETE CASCADE,
    PRIMARY KEY (supplement_id, source_id)
);
CREATE TABLE evidence_ref (
    id            INTEGER PRIMARY KEY,
    supplement_id INTEGER REFERENCES supplement(id) ON DELETE CASCADE,
    url           TEXT,
    kd_work_id    TEXT,
    stance        TEXT NOT NULL DEFAULT 'neutral',
    title         TEXT,
    note          TEXT,
    added_at      TEXT NOT NULL
);
CREATE TABLE product (
    id             INTEGER PRIMARY KEY,
    position       INTEGER NOT NULL,
    category       TEXT,
    name           TEXT NOT NULL,
    price          REAL,
    price_currency TEXT NOT NULL DEFAULT 'USD',
    price_date     TEXT,
    dosage         TEXT,
    in_use         INTEGER,
    units_per_year REAL,
    notes          TEXT,
    url            TEXT,
    alt_url        TEXT
);
CREATE TABLE nutrition_target (
    id          INTEGER PRIMARY KEY,
    position    INTEGER NOT NULL,
    label       TEXT NOT NULL,
    calories    REAL,
    protein     REAL,
    net_carbs   REAL,
    fiber       REAL,
    sugar       REAL,
    sodium      REAL,
    cholesterol REAL,
    total_fat   REAL,
    unsat_fat   REAL,
    trans_fat   REAL,
    sat_fat     REAL,
    yuka_score  REAL,
    extra       TEXT
);
CREATE TABLE nutrition_item (
    id             INTEGER PRIMARY KEY,
    position       INTEGER NOT NULL,
    category       TEXT,
    name           TEXT NOT NULL,
    calories       REAL,
    protein        REAL,
    net_carbs      REAL,
    fiber          REAL,
    sugar          REAL,
    sodium         REAL,
    cholesterol    REAL,
    total_fat      REAL,
    unsat_fat      REAL,
    trans_fat      REAL,
    sat_fat        REAL,
    yuka_score     REAL,
    ingredients_ok TEXT,
    no_prep        TEXT,
    vegan          TEXT,
    qualifying     TEXT,
    portions       REAL,
    notes          TEXT,
    price          REAL,
    price_currency TEXT NOT NULL DEFAULT 'USD',
    price_date     TEXT,
    url            TEXT,
    alt_url        TEXT
);
CREATE TABLE routine_step (
    id             INTEGER PRIMARY KEY,
    routine        TEXT NOT NULL,
    position       INTEGER NOT NULL,
    name           TEXT NOT NULL,
    effect_time    TEXT,
    exec_seconds   REAL,
    kind           TEXT,
    notes          TEXT
);
CREATE TABLE exercise (
    id       INTEGER PRIMARY KEY,
    position INTEGER NOT NULL,
    name     TEXT NOT NULL,
    muscles  TEXT
);
CREATE TABLE exercise_alt (
    id          INTEGER PRIMARY KEY,
    exercise_id INTEGER NOT NULL REFERENCES exercise(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    text        TEXT NOT NULL
);
CREATE TABLE vaccine (
    id           INTEGER PRIMARY KEY,
    position     INTEGER NOT NULL,
    kind         TEXT NOT NULL DEFAULT 'vaccine',
    name         TEXT NOT NULL,
    disease_date TEXT,
    disease_note TEXT,
    notes        TEXT
);
CREATE TABLE vaccine_dose (
    id         INTEGER PRIMARY KEY,
    vaccine_id INTEGER NOT NULL REFERENCES vaccine(id) ON DELETE CASCADE,
    dose_no    INTEGER NOT NULL,
    given_on   TEXT,
    raw        TEXT
);
CREATE TABLE physical_check (
    id       INTEGER PRIMARY KEY,
    position INTEGER NOT NULL,
    section  TEXT NOT NULL,
    name     TEXT NOT NULL,
    detail   TEXT,
    done     INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE provider (
    id       INTEGER PRIMARY KEY,
    position INTEGER NOT NULL,
    name     TEXT NOT NULL,
    note     TEXT,
    url      TEXT
);
CREATE TABLE size (
    id       INTEGER PRIMARY KEY,
    position INTEGER NOT NULL,
    category TEXT NOT NULL,
    name     TEXT NOT NULL,
    value    REAL,
    unit     TEXT,
    note     TEXT
);
CREATE TABLE resource_link (
    id       INTEGER PRIMARY KEY,
    position INTEGER NOT NULL,
    section  TEXT NOT NULL DEFAULT 'resources',
    label    TEXT,
    url      TEXT NOT NULL
);
CREATE INDEX idx_dose_supplement ON dose_schedule(supplement_id);
CREATE INDEX idx_evidence_supplement ON evidence_ref(supplement_id);
CREATE INDEX idx_vaccine_dose_vaccine ON vaccine_dose(vaccine_id);
";

pub const MIGRATIONS: &[Migration] =
    &[Migration { version: 1, name: "initial schema", apply: |db| db.execute_batch(SCHEMA_V1) }];

pub fn latest_version() -> i64 {
    MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0)
}

fn ensure_table(db: &Db) -> Result<()> {
    // Check first: a no-op CREATE would still count as a write and re-seal
    // an encrypted database on every read-only command.
    if db.query_opt("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'", &[])?.is_some() {
        return Ok(());
    }
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )
}

/// Applied migrations as (version, name, applied_at).
pub fn applied(db: &Db) -> Result<Vec<(i64, String, String)>> {
    ensure_table(db)?;
    Ok(db
        .query("SELECT version, name, applied_at FROM schema_migrations ORDER BY version", &[])?
        .iter()
        .map(|r| (r.i(0).unwrap_or_default(), r.s(1).unwrap_or_default(), r.s(2).unwrap_or_default()))
        .collect())
}

pub fn current_version(db: &Db) -> Result<i64> {
    Ok(applied(db)?.iter().map(|a| a.0).max().unwrap_or(0))
}

/// Apply all pending migrations; returns the versions that were applied.
pub fn migrate(db: &Db) -> Result<Vec<i64>> {
    let current = current_version(db)?;
    if current > latest_version() {
        return Err(crate::error::AppError::db(format!(
            "database schema version {current} is newer than this binary supports ({})",
            latest_version()
        )));
    }
    MIGRATIONS
        .iter()
        .filter(|m| m.version > current)
        .map(|m| {
            db.transaction(|db| {
                (m.apply)(db)?;
                db.execute(
                    "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                    &[int(m.version), text(m.name), text(crate::util::now_iso())],
                )?;
                Ok(m.version)
            })
        })
        .collect::<Result<Vec<_>>>()
        .and_then(|v| {
            if !v.is_empty() {
                db.execute(&format!("PRAGMA user_version = {}", latest_version()), &[])?;
            }
            Ok(v)
        })
}
