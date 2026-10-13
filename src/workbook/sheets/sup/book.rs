//! `Sup_Book` (SUP-01): the position book. One row per `tblSupp` row with dose,
//! frequency, time of day, price, $/yr, protocol, research stance, start, status
//! and weight (% of active annual spend). Every other per-supplement view reads
//! its numbers from here through [`at`] / [`range`].

use rust_xlsxwriter::{DataValidation, Worksheet};

use super::{fx, head, kpi, mirror, n, supp_cell, table, x, HEAD};
use crate::error::Result;
use crate::infra::data::EVIDENCE;
use crate::infra::refs::REF_STANCE;
use crate::infra::theme;
use crate::workbook::sheets::lists;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Book";

pub const HEADERS: &[&str] = &[
    "supp_id",
    "name",
    "taking",
    "kind",
    "frequency",
    "time_of_day",
    "amount",
    "unit",
    "dose",
    "doses_per_wk",
    "daily_dose",
    "price",
    "units_per_year",
    "usd_yr",
    "usd_mo",
    "usd_day",
    "active_usd_yr",
    "weight",
    "protocols",
    "n_protocols",
    "n_refs",
    "stance_net",
    "stance",
    "start_date",
    "stop_date",
    "status",
    "weeks_on",
    "rank_key",
];

/// Zero-based column of a book header (panics on a typo: a programming error).
pub fn c(h: &str) -> u16 {
    HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("Sup_Book has no column {h}")) as u16
}

/// Absolute range of a book column over all supplements.
pub fn range(cx: &Cx, h: &str) -> String {
    super::rng(TITLE, c(h), cx.d)
}

/// The book cell of column `h` on Excel row `xr` (same row in every aligned view).
pub fn at(h: &str, xr: u32) -> String {
    format!("{TITLE}!${}{xr}", col(c(h)))
}

/// Stance words as `tblEvidence` and `Ref_Stance` spell them.
pub const STANCES: &[&str] = &["supporting", "neutral", "warning"];
/// Labels of the derived per-supplement stance.
pub const STANCE_LABELS: &[&str] = &["supporting", "mixed", "warning", "none"];

pub struct Book;

impl SheetModule for Book {
    fn id(&self) -> &'static str {
        "sup-book"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        super::register_params(cx)?;
        head(ws, cx, "Position book: every supplement, its cost and its standing", "SUP-01", "KAU Ch24 (quant Q44)")?;
        let dates = match &d.regimens {
            Some(r) => format!("taking, start_date and stop_date follow `biomarker regimen ls -p {}` where it has a regimen (Data_Supp.taking_source).", r.person),
            None => "start_date / stop_date come from biomarker regimens, not available (GATE-DATES).".to_string(),
        };
        ws.write_string_with_format(
            3,
            0,
            format!("Inputs come from Data_Supp; every column right of the inputs is a formula. {dates}"),
            &f.note,
        )?;
        let rows = n(d);
        table(ws, "tblSupBook", HEAD, 0, HEADERS, rows)?;
        let nev = EVIDENCE.nrows(d);
        let (ev_supp, ev_stance) = (EVIDENCE.range_n(nev, "supp_id"), EVIDENCE.range_n(nev, "stance"));
        let ns = REF_STANCE.nrows(d);
        let (st_name, st_w) = (REF_STANCE.range_n(ns, "stance"), REF_STANCE.range_n(ns, "weight"));
        let active = range(cx, "active_usd_yr");
        let l = |h: &str| col(c(h));
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            let cell = |h: &str| format!("{}{xr}", l(h));
            for (h, src) in [
                ("supp_id", "supp_id"),
                ("name", "name"),
                ("taking", "taking"),
                ("kind", "kind"),
                ("frequency", "frequency"),
                ("time_of_day", "time_of_day"),
                ("amount", "amount"),
                ("unit", "unit"),
                ("price", "price"),
                ("units_per_year", "units_per_year"),
                ("protocols", "sources"),
                ("start_date", "start_date"),
                ("stop_date", "stop_date"),
            ] {
                let fm = match h {
                    "price" => Some(&f.money),
                    "start_date" | "stop_date" => Some(&f.date),
                    _ => None,
                };
                fx(ws, r, c(h), &mirror(&supp_cell(src, i)), fm)?;
            }
            let (amount, unit, text) = (cell("amount"), cell("unit"), supp_cell("amount_text", i));
            fx(
                ws,
                r,
                c("dose"),
                &format!("=IF(ISNUMBER({amount}),TRIM({amount}&\" \"&{unit}),IF({text}=\"\",\"\",{text}))"),
                None,
            )?;
            fx(
                ws,
                r,
                c("doses_per_wk"),
                &format!(
                    "=IFERROR(SUM(INDEX({},MATCH({},{},0),0)),\"\")",
                    lists::freq_matrix(),
                    cell("frequency"),
                    lists::freq_names()
                ),
                None,
            )?;
            let dpw = cell("doses_per_wk");
            fx(
                ws,
                r,
                c("daily_dose"),
                &format!("=IF(AND(ISNUMBER({amount}),ISNUMBER({dpw})),{amount}*{dpw}/7,\"\")"),
                Some(&f.dec2),
            )?;
            let (price, units) = (cell("price"), cell("units_per_year"));
            fx(
                ws,
                r,
                c("usd_yr"),
                &format!("=IF(AND(ISNUMBER({price}),ISNUMBER({units})),{price}*{units},0)"),
                Some(&f.money),
            )?;
            let yr = cell("usd_yr");
            fx(ws, r, c("usd_mo"), &format!("={yr}/12"), Some(&f.money))?;
            fx(ws, r, c("usd_day"), &format!("={yr}/365.25"), Some(&f.money))?;
            fx(ws, r, c("active_usd_yr"), &format!("=IF({}=\"YES\",{yr},0)", cell("taking")), Some(&f.money))?;
            fx(
                ws,
                r,
                c("weight"),
                &format!("=IF(SUM({active})=0,0,{}/SUM({active}))", cell("active_usd_yr")),
                Some(&f.pct),
            )?;
            let p = cell("protocols");
            fx(
                ws,
                r,
                c("n_protocols"),
                &format!("=IF(TRIM({p})=\"\",0,LEN(TRIM({p}))-LEN(SUBSTITUTE(TRIM({p}),\",\",\"\"))+1)"),
                None,
            )?;
            let id = cell("supp_id");
            fx(ws, r, c("n_refs"), &format!("=COUNTIFS({ev_supp},{id})"), None)?;
            let weighted: Vec<String> = STANCES
                .iter()
                .map(|s| {
                    format!(
                        "COUNTIFS({ev_supp},{id},{ev_stance},\"{s}\")*IFERROR(INDEX({st_w},MATCH(\"{s}\",{st_name},0)),0)"
                    )
                })
                .collect();
            fx(ws, r, c("stance_net"), &format!("={}", weighted.join("+")), None)?;
            let (nr, net) = (cell("n_refs"), cell("stance_net"));
            fx(
                ws,
                r,
                c("stance"),
                &format!("=IF({nr}=0,\"none\",IF({net}>0,\"supporting\",IF({net}<0,\"warning\",\"mixed\")))"),
                None,
            )?;
            let (start, stop) = (cell("start_date"), cell("stop_date"));
            fx(
                ws,
                r,
                c("status"),
                &format!(
                    "=IF(ISNUMBER({stop}),\"stopped\",IF({}=\"YES\",\"active\",IF({}=\"NO\",\"not taking\",\"unknown\")))",
                    cell("taking"),
                    cell("taking")
                ),
                None,
            )?;
            fx(
                ws,
                r,
                c("weeks_on"),
                &format!("=IF(ISNUMBER({start}),(IF(ISNUMBER({stop}),{stop},Today)-{start})/7,\"\")"),
                Some(&f.dec1),
            )?;
            // Unique sort key for the Pareto / frontier lists (ties broken by row).
            fx(ws, r, c("rank_key"), &format!("={}+ROW()/1000000000", cell("active_usd_yr")), None)?;
        }
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        theme::cf_data_bar(ws, r0, c("weight"), r1, c("weight"), "A9D18E")?;
        theme::cf_data_bar(ws, r0, c("usd_yr"), r1, c("usd_yr"), "9BC2E6")?;
        for (s, fill) in [("supporting", theme::OPTIMAL), ("warning", theme::CRITICAL), ("none", theme::PLACEHOLDER)] {
            let tl = format!("${}{}", l("stance"), r0 + 1);
            theme::cf_formula(ws, r0, c("stance"), r1, c("stance"), &format!("={tl}=\"{s}\""), fill)?;
        }
        ws.add_data_validation(
            r0,
            c("taking"),
            r1 + 100,
            c("taking"),
            &DataValidation::new().allow_list_strings(lists::YES_NO)?,
        )?;
        ws.add_data_validation(
            r0,
            c("time_of_day"),
            r1 + 100,
            c("time_of_day"),
            &DataValidation::new().allow_list_strings(lists::TIMES)?,
        )?;
        let mut w =
            vec![9.0, 30.0, 8.0, 6.0, 10.0, 12.0, 8.0, 7.0, 12.0, 8.0, 9.0, 9.0, 8.0, 10.0, 9.0, 9.0, 10.0, 8.0];
        w.extend([30.0, 7.0, 7.0, 8.0, 11.0, 11.0, 11.0, 10.0, 8.0, 6.0]);
        set_widths(ws, &w)?;
        ws.set_column_hidden(c("rank_key"))?;
        ws.set_freeze_panes(HEAD + 1, 2)?;
        let taking = range(cx, "taking");
        kpi(cx, "n_active", "Supplements currently taken", &format!("COUNTIF({taking},\"YES\")"), None)?;
        kpi(cx, "usd_yr", "Active supplement spend per year", &format!("SUM({active})"), Some(theme::nf::MONEY0))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_resolve_and_cells_are_row_aligned() {
        assert_eq!(c("supp_id"), 0);
        assert_eq!(at("usd_yr", 7), "Sup_Book!$N7");
        assert_eq!(HEADERS.len(), c("rank_key") as usize + 1);
    }
}
