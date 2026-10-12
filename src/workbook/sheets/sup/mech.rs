//! `Sup_Mech` (SUP-06) mechanism-exposure ("factor loadings") matrix and
//! `Sup_DropOne` (SUP-07) drop-one analysis. Both read the user-owned
//! `tblSuppTags.m_*` loadings (0-3), empty until someone tags the stack: placeholders.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{book, data_cell, fx, head, table, x, HEAD};
use crate::error::Result;
use crate::infra::data::{SUPP_TAGS, SUPP_TAGS_DEF as TAGS};
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const MECH_TITLE: &str = "Sup_Mech";
pub const DROP_TITLE: &str = "Sup_DropOne";
/// First loading column on `Sup_Mech`.
const M0: u16 = 3;

/// The `m_*` tag columns, in order.
pub fn mechanisms() -> Vec<&'static str> {
    SUPP_TAGS.iter().copied().filter(|c| c.starts_with("m_")).collect()
}

fn m1() -> u16 {
    M0 + mechanisms().len() as u16 - 1
}

/// Excel row of the exposure (column-sum) row.
pub fn exposure_row(d: &Data) -> u32 {
    super::last(d) + 2
}

/// `Sup_Mech!$D$e:$M$e`: total exposure per mechanism.
pub fn exposure_range(d: &Data) -> String {
    let e = exposure_row(d);
    format!("{MECH_TITLE}!${}${e}:${}${e}", col(M0), col(m1()))
}

/// `Sup_Mech!$D7:$M7`: the loadings of the supplement on Excel row `xr`.
pub fn row_range(xr: u32) -> String {
    format!("{MECH_TITLE}!${}{xr}:${}{xr}", col(M0), col(m1()))
}

/// Total loading of the supplement on Excel row `xr`.
pub fn total_at(xr: u32) -> String {
    format!("{MECH_TITLE}!${}{xr}", col(m1() + 1))
}

pub struct Mech;

impl SheetModule for Mech {
    fn id(&self) -> &'static str {
        "sup-mech"
    }
    fn title(&self) -> &'static str {
        MECH_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Mechanism exposure (factor loadings), coverage and cost per mechanism",
            "SUP-06",
            "PAL Ch4.1-4.2 (quant Q47); loadings MED-VERIFY",
        )?;
        theme::awaiting(ws, 3, "mechanism loadings 0-3 in tblSuppTags m_* (user/LLM tagging, flag unverified)", f)?;
        let ms = mechanisms();
        let mut hdr = vec!["name", "active", "usd_yr"];
        hdr.extend(ms.iter().copied());
        hdr.push("total_loading");
        let rows = super::n(d);
        table(ws, "tblSupMech", HEAD, 0, &hdr, rows)?;
        let tot_c = col(m1() + 1);
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &format!("=IF({}=\"YES\",1,0)", book::at("taking", xr)), None)?;
            fx(ws, r, 2, &format!("={}", book::at("active_usd_yr", xr)), Some(&f.money))?;
            for (k, m) in ms.iter().enumerate() {
                fx(ws, r, M0 + k as u16, &format!("=$B{xr}*N({})", data_cell(&TAGS, m, i)), None)?;
            }
            fx(ws, r, m1() + 1, &format!("=SUM({}{xr}:{}{xr})", col(M0), col(m1())), None)?;
        }
        theme::cf_two_color(ws, HEAD + 1, M0, HEAD + rows, m1(), "FFFFFF", "63BE7B")?;
        // Column summaries under the matrix.
        let (x0, xl) = (x(0), super::last(d));
        let e = exposure_row(d); // Excel row
        type Line<'a> = (&'a str, &'a dyn Fn(&str) -> String);
        let lines: [Line; 4] = [
            ("exposure (sum of loadings)", &|c| format!("=SUM({c}{x0}:{c}{xl})")),
            ("items loading", &|c| format!("=COUNTIF({c}{x0}:{c}{xl},\">0\")")),
            ("covered (>= Sup_Cover_Min)", &|c| format!("=IF({c}{e}>=Sup_Cover_Min,1,0)")),
            ("cost per mechanism ($/yr, by loading share)", &|c| {
                format!("=SUMPRODUCT($C${x0}:$C${xl}/(${tot_c}${x0}:${tot_c}${xl}+(${tot_c}${x0}:${tot_c}${xl}=0))*{c}{x0}:{c}{xl})")
            }),
        ];
        for (k, (label, fm)) in lines.iter().enumerate() {
            let r = e - 1 + k as u32;
            ws.write_string_with_format(r, 0, *label, &f.bold)?;
            for m in M0..=m1() {
                fx(ws, r, m, &fm(&col(m)), if k == 3 { Some(&f.money0) } else { None })?;
            }
        }
        super::figure(
            ws,
            cx,
            e + 4,
            0,
            "Mechanisms covered",
            &format!("=SUM({}{}:{}{})", col(M0), e + 2, col(m1()), e + 2),
            None,
        )?;
        let mut radar = Chart::new(ChartType::RadarFilled);
        radar.add_series().set_name("exposure").set_categories((MECH_TITLE, HEAD, M0, HEAD, m1())).set_values((
            MECH_TITLE,
            e - 1,
            M0,
            e - 1,
            m1(),
        ));
        radar.title().set_name("Mechanism coverage (sum of loadings)");
        radar.set_width(480).set_height(360);
        ws.insert_chart(e + 6, 0, &radar)?;
        let mut w = vec![30.0, 7.0, 10.0];
        w.extend(std::iter::repeat_n(9.0, ms.len()));
        w.push(10.0);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

const DROP_HEADERS: &[&str] =
    &["name", "active", "saving_usd_yr", "total_loading", "mechanisms_lost", "coverage_left", "flag"];

pub struct DropOne;

impl SheetModule for DropOne {
    fn id(&self) -> &'static str {
        "sup-dropone"
    }
    fn title(&self) -> &'static str {
        DROP_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Drop-one: saving and mechanism coverage lost per item",
            "SUP-07",
            "PAL Ch5, Ch4.5.2 (quant Q51)",
        )?;
        theme::awaiting(ws, 3, "mechanism loadings in tblSuppTags (via Sup_Mech)", f)?;
        let rows = super::n(d);
        table(ws, "tblSupDropOne", HEAD, 0, DROP_HEADERS, rows)?;
        let exp = exposure_range(d);
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            let row = row_range(xr);
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &format!("=IF({}=\"YES\",1,0)", book::at("taking", xr)), None)?;
            fx(ws, r, 2, &format!("={}", book::at("active_usd_yr", xr)), Some(&f.money))?;
            fx(ws, r, 3, &format!("={}", total_at(xr)), None)?;
            fx(ws, r, 4, &format!("=SUMPRODUCT(({row}>0)*(({exp}-{row})<Sup_Cover_Min))"), None)?;
            fx(ws, r, 5, &format!("=IF(SUM({exp})=0,\"\",(SUM({exp})-D{xr})/SUM({exp}))"), Some(&f.pct))?;
            fx(
                ws,
                r,
                6,
                &format!("=IF(B{xr}=0,\"\",IF(D{xr}=0,\"untagged\",IF(E{xr}=0,\"redundant: no unique coverage\",\"unique coverage\")))"),
                None,
            )?;
        }
        theme::cf_formula(
            ws,
            HEAD + 1,
            6,
            HEAD + rows,
            6,
            &format!("=LEFT($G{},9)=\"redundant\"", HEAD + 2),
            theme::REF_OK,
        )?;
        let (x0, xl) = (x(0), super::last(d));
        let s = HEAD + rows + 3;
        super::figure(ws, cx, s, 0, "Redundant items", &format!("=COUNTIF($G${x0}:$G${xl},\"redundant*\")"), None)?;
        super::figure(
            ws,
            cx,
            s + 1,
            0,
            "Saving if all redundant items dropped ($/yr)",
            &format!("=SUMIFS($C${x0}:$C${xl},$G${x0}:$G${xl},\"redundant*\")"),
            Some(&f.money),
        )?;
        set_widths(ws, &[30.0, 7.0, 12.0, 11.0, 14.0, 12.0, 28.0])?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}
