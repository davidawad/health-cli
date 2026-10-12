//! `sheet`: generate the Health Dashboard workbook from the store.

use std::path::PathBuf;

use serde_json::json;

use crate::cli::SheetArgs;
use crate::context::Ctx;
use crate::error::{AppError, Result};
use crate::output::{to_record, Report};
use crate::paths::expand_tilde;
use crate::workbook::{self, labs, sheets, Data};

/// Where the workbook goes: `--out`, else the `sheet_out` setting.
fn target(ctx: &Ctx, a: &SheetArgs) -> PathBuf {
    a.out.clone().unwrap_or_else(|| PathBuf::from(expand_tilde(ctx.resolved.get("sheet_out"))))
}

/// A hand-made workbook (the original `Health Dashboard.xlsx`) must never be
/// overwritten by accident: only files this command names `(generated)`, or
/// `--force`, may be replaced.
fn check_overwrite(path: &std::path::Path, force: bool) -> Result<()> {
    let generated = path.file_name().is_some_and(|n| n.to_string_lossy().contains("(generated)"));
    if path.exists() && !generated && !force {
        return Err(AppError::invalid(format!(
            "{} exists and is not a generated workbook (its name lacks \"(generated)\"); choose another --out or pass --force",
            path.display()
        )));
    }
    Ok(())
}

pub fn run(ctx: &Ctx, a: SheetArgs) -> Result<()> {
    let registry = sheets::registry();
    if a.list {
        let rows = registry
            .iter()
            .map(|m| to_record(&json!({"id": m.id(), "title": m.title(), "hidden": m.hidden()})))
            .collect();
        return ctx.emit(&Report::list("sheets", rows));
    }
    let out = target(ctx, &a);
    check_overwrite(&out, a.force)?;
    let db = ctx.db()?;
    let mut data = Data::load(&db)?;
    if !a.no_labs {
        let person = ctx.resolved.get("biomarker_person");
        match labs::fetch_labs(ctx.resolved.get("biomarker_cmd"), person) {
            Ok(l) => data.labs = Some(l),
            Err(e) => ctx.warn(&format!("warning: no lab sheets ({})", e.message)),
        }
    }
    let genome = ctx.resolved.get("genome_cmd");
    if !a.no_genetics && !genome.is_empty() {
        match labs::fetch_kits(genome) {
            Ok(k) => data.kits = Some(k),
            Err(e) => ctx.warn(&format!("warning: no Genetics sheet ({})", e.message)),
        }
    }
    let mut wb = workbook::build(&data, &a.only)?;
    workbook::save(&mut wb, &out)?;
    let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    ctx.warn(&format!(
        "wrote plaintext health data to {} (mode 0600); keep it out of git and shared folders",
        out.display()
    ));
    let titles: Vec<&str> = registry
        .iter()
        .filter(|m| (a.only.is_empty() || a.only.iter().any(|o| o == m.id())) && m.available(&data))
        .map(|m| m.title())
        .collect();
    ctx.emit_mutation(&Report::object(
        "sheet",
        to_record(&json!({
            "path": out,
            "bytes": bytes,
            "sheets": titles,
            "labs": data.labs.is_some(),
            "genetics": data.kits.is_some(),
        })),
    ))
}
