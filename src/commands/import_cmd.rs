//! `import xlsx PATH`.

use serde_json::json;

use crate::cli::{ImportCmd, ImportXlsxArgs};
use crate::context::Ctx;
use crate::error::{AppError, Result};
use crate::import_xlsx;
use crate::output::{to_record, Report};
use crate::util::parse_date;

pub fn run(ctx: &Ctx, cmd: ImportCmd) -> Result<()> {
    match cmd {
        ImportCmd::Xlsx(a) => xlsx(ctx, a),
    }
}

/// The workbook's modification date: the best "as of" date for its prices.
fn file_date(path: &std::path::Path) -> Result<String> {
    let modified = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map_err(|e| AppError::io(format!("{}: {e}", path.display())))?;
    Ok(chrono::DateTime::<chrono::Utc>::from(modified).format("%Y-%m-%d").to_string())
}

fn xlsx(ctx: &Ctx, a: ImportXlsxArgs) -> Result<()> {
    if !a.path.is_file() {
        return Err(AppError::not_found(format!("{} does not exist", a.path.display())));
    }
    let price_date = match &a.price_date {
        Some(d) => parse_date(d, &ctx.tz)?.to_string(),
        None => file_date(&a.path)?,
    };
    let db = ctx.db()?;
    let report = import_xlsx::import(&db, &a.path, &price_date, a.replace)?;
    let total: usize = report.sheets.iter().map(|s| s.items).sum();
    ctx.info(&format!("imported {total} rows from {} sheets of {}", report.sheets.len(), a.path.display()));
    let rows = report.sheets.iter().map(to_record).collect();
    ctx.emit(
        &Report::list("import_xlsx", rows)
            .table_columns(&["sheet", "rows_seen", "items", "headers", "derived", "other"])
            .meta("source", json!(report.source))
            .meta("sha256", json!(report.sha256))
            .meta("price_date", json!(price_date))
            .meta("skipped", json!(report.skipped)),
    )
}
