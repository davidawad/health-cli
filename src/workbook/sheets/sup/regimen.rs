//! `Sup_Regimen` (SUP-23): taking vs not per supplement and the contraindication
//! grid, from biomarker-cli (>= 0.7.1), which owns per-person regimens. The table
//! is row-aligned with `tblSupp`: `taking` mirrors the book (biomarker's status
//! when it has a regimen, else the store's), next to the store's own value and
//! biomarker's status, start/stop days and dose. Below it come biomarker regimens
//! with no store row, then `interaction grid --catalog` (rows: what is taken;
//! columns: those plus every catalog intervention with an edge to them) and the
//! edges to conditions, lab tests, genotypes and classes no column belongs to.
//! Without biomarker the table shows the store and the grid is a placeholder.

use rust_xlsxwriter::{Format, Worksheet};

use super::{book, fx, head, mirror, supp_cell, table, x, HEAD};
use crate::error::Result;
use crate::infra::theme;
use crate::regimens::Catalog;
use crate::store::Rec;
use crate::workbook::regimens::Grid;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Regimen";

pub const HEADERS: &[&str] = &[
    "name",
    "slug",
    "taking",
    "store_taking",
    "biomarker_status",
    "agree",
    "started_on",
    "stopped_on",
    "days",
    "dose",
    "unit",
    "frequency",
    "time_slot",
    "source",
];

const EXTRA_HEADERS: &[&str] = &["intervention", "name", "status", "started_on", "stopped_on", "dose", "unit"];
const OTHER_HEADERS: &[&str] = &["intervention", "target_kind", "target", "kind", "severity"];

/// Zero-based column of `h` in [`HEADERS`].
fn c(h: &str) -> u16 {
    HEADERS.iter().position(|x| *x == h).unwrap_or(0) as u16
}

/// Cell format of a grid relation: red do-not-combine, amber with care, grey lab, green synergy.
fn kind_format<'a>(cx: &'a Cx, kind: &str) -> &'a Format {
    match kind {
        "contraindicated" | "avoid" => &cx.f.bad,
        "caution" | "monitor" => &cx.f.warn,
        "synergy" => &cx.f.good,
        _ => &cx.f.neutral,
    }
}

pub struct Regimen;

impl SheetModule for Regimen {
    fn id(&self) -> &'static str {
        "sup-regimen"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Regimens: taking vs not, and the contraindication grid",
            "SUP-23",
            "biomarker regimen ls / interaction grid --catalog (biomarker-cli >= 0.7.1); store fallback",
        )?;
        match &d.regimens {
            Some(r) => ws.write_string_with_format(
                3,
                0,
                format!(
                    "Taking, start and stop from `biomarker regimen ls -p {}`; supplements biomarker has no regimen for fall back to the store (source = store).",
                    r.person
                ),
                &f.note,
            )
            .map(|_| ())?,
            None => theme::awaiting(ws, 3, "biomarker regimens (biomarker-cli >= 0.7.1); taking shown from the store", f)?,
        }
        let rows = super::n(d);
        let research = d.regimens.as_ref().map_or_else(Catalog::embedded, |r| r.research.clone());
        table(ws, "tblSupRegimen", HEAD, 0, HEADERS, rows)?;
        for (i, s) in d.supplements.iter().enumerate() {
            let (r, xr) = (HEAD + 1 + i as u32, x(i as u32));
            let name = s.s("name").unwrap_or_default();
            fx(ws, r, c("name"), &format!("={}", book::at("name", xr)), None)?;
            if let Some(slug) = research.slug_for(&name) {
                ws.write_string(r, c("slug"), slug)?;
            }
            fx(ws, r, c("taking"), &mirror(&book::at("taking", xr)), None)?;
            if let Some(t) = s.b("taking") {
                ws.write_string(r, c("store_taking"), if t { "YES" } else { "NO" })?;
            }
            let (tk, st) = (col(c("taking")), col(c("store_taking")));
            fx(
                ws,
                r,
                c("agree"),
                &format!("=IF(OR({tk}{xr}=\"\",{st}{xr}=\"\"),\"\",IF({tk}{xr}={st}{xr},\"yes\",\"DIFFERS\"))"),
                None,
            )?;
            fx(ws, r, c("started_on"), &mirror(&book::at("start_date", xr)), Some(&f.date))?;
            fx(ws, r, c("stopped_on"), &mirror(&book::at("stop_date", xr)), Some(&f.date))?;
            fx(ws, r, c("source"), &mirror(&supp_cell("taking_source", i as u32)), None)?;
            let Some(g) = d.regimen(&name) else { continue };
            ws.write_string(r, c("biomarker_status"), &g.status)?;
            if let Some(n) = g.days {
                ws.write_number(r, c("days"), n as f64)?;
            }
            if let Some(n) = g.dose {
                ws.write_number(r, c("dose"), n)?;
            }
            for (h, v) in [("unit", &g.unit), ("frequency", &g.frequency), ("time_slot", &g.time_slot)] {
                if let Some(v) = v {
                    ws.write_string(r, c(h), v)?;
                }
            }
        }
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        theme::cf_formula(
            ws,
            r0,
            c("agree"),
            r1,
            c("agree"),
            &format!("=${}{}=\"DIFFERS\"", col(c("agree")), r0 + 1),
            theme::OUT_OF_REF,
        )?;
        let mut row = HEAD + rows + 2;
        row = extra(ws, cx, row)?;
        grid(ws, cx, row)?;
        set_widths(ws, &[24.0, 22.0, 8.0, 8.0, 12.0, 9.0, 11.0, 11.0, 7.0, 8.0, 8.0, 12.0, 12.0, 10.0])?;
        Ok(())
    }
}

/// Biomarker regimens with no store supplement; returns the next free row.
fn extra(ws: &mut Worksheet, cx: &Cx, row: u32) -> Result<u32> {
    let d = cx.d;
    let Some(regs) = &d.regimens else { return Ok(row) };
    let known: Vec<&crate::workbook::regimens::Regimen> =
        d.supplements.iter().filter_map(|s| s.s("name")).flat_map(|n| regs.of(&n)).collect();
    let extra: Vec<_> = regs.rows.iter().filter(|r| !known.contains(r)).collect();
    if extra.is_empty() {
        return Ok(row);
    }
    ws.write_string_with_format(row, 0, "Biomarker regimens with no store supplement", &cx.f.section)?;
    table(ws, "tblSupRegimenExtra", row + 1, 0, EXTRA_HEADERS, extra.len() as u32)?;
    for (i, g) in extra.iter().enumerate() {
        let r = row + 2 + i as u32;
        for (k, v) in [
            &Some(g.intervention.clone()),
            &Some(g.name.clone()),
            &Some(g.status.clone()),
            &g.started_on,
            &g.stopped_on,
        ]
        .iter()
        .enumerate()
        {
            if let Some(v) = v {
                ws.write_string(r, k as u16, v)?;
            }
        }
        if let Some(n) = g.dose {
            ws.write_number(r, 5, n)?;
        }
        if let Some(u) = &g.unit {
            ws.write_string(r, 6, u)?;
        }
    }
    Ok(row + 3 + extra.len() as u32)
}

/// The contraindication grid and its `other` edges at `row`.
fn grid(ws: &mut Worksheet, cx: &Cx, row: u32) -> Result<()> {
    let f = cx.f;
    let Some(regs) = &cx.d.regimens else {
        return theme::awaiting(ws, row, "biomarker interaction grid (biomarker-cli >= 0.7.1)", f);
    };
    let Some(g) = &regs.grid else {
        return theme::awaiting(ws, row, "biomarker interaction grid (`interaction grid` failed)", f);
    };
    ws.write_string_with_format(
        row,
        0,
        format!("Contraindication grid: `biomarker interaction grid -p {} --catalog` as of {}", regs.person, g.as_of),
        &f.section,
    )?;
    if g.rows.is_empty() {
        ws.write_string_with_format(row + 1, 0, format!("{} is taking nothing on {}.", regs.person, g.as_of), &f.note)?;
        return Ok(());
    }
    write_matrix(ws, cx, row + 1, g)?;
    let o = row + 3 + g.rows.len() as u32;
    ws.write_string_with_format(
        o,
        0,
        "Edges to conditions, lab tests, genotypes and classes outside the grid",
        &f.section,
    )?;
    table(ws, "tblSupRegimenOther", o + 1, 0, OTHER_HEADERS, g.other.len() as u32)?;
    for (i, e) in g.other.iter().enumerate() {
        let r = o + 2 + i as u32;
        for (k, v) in [&e.intervention, &e.target_kind, &e.target, &e.kind, &e.severity].iter().enumerate() {
            ws.write_string(r, k as u16, *v)?;
        }
    }
    Ok(())
}

/// Header row of column slugs (columns already taken are bold), one row per taken
/// intervention, the cell holding the highest-ranked kind (with its class, if any).
fn write_matrix(ws: &mut Worksheet, cx: &Cx, row: u32, g: &Grid) -> Result<()> {
    let f = cx.f;
    ws.write_string_with_format(row, 0, "taking \\ with", &f.header)?;
    for (j, col) in g.columns.iter().enumerate() {
        ws.write_string_with_format(row, 1 + j as u16, col, &f.header)?;
    }
    for (i, r) in g.rows.iter().enumerate() {
        let xr = row + 1 + i as u32;
        ws.write_string_with_format(xr, 0, r, &f.bold)?;
        for (j, col) in g.columns.iter().enumerate() {
            let c = 1 + j as u16;
            if col == r {
                ws.write_string_with_format(xr, c, "-", &f.placeholder)?;
            } else if let Some(cell) = g.cell(r, col) {
                let text =
                    if cell.via.is_empty() { cell.kind.clone() } else { format!("{} ({})", cell.kind, cell.via) };
                ws.write_string_with_format(xr, c, text, kind_format(cx, &cell.kind))?;
            }
        }
    }
    Ok(())
}
