//! `Sup_Cost` (SUP-02, SUP-03): cost attribution by protocol and research stance,
//! protocol x stance matrix, waterfall, Pareto with HHI / Gini, the "spend with no
//! evidence" call-out, and the spend KPIs ($/day, $/month, TER vs budget, by time of
//! day and by kind). An item listed under several protocols is split equally among them.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::book::{self, STANCE_LABELS};
use super::{figure, fx, head, kpi, lit, n, protocols, table, HEAD};
use crate::error::Result;
use crate::infra::data::PRODUCTS;
use crate::infra::theme;
use crate::workbook::sheets::lists;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Cost";
/// First column of the Pareto block.
const PC: u16 = 12;

pub struct Cost;

impl SheetModule for Cost {
    fn id(&self) -> &'static str {
        "sup-cost"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Cost attribution, concentration and spend KPIs",
            "SUP-02, SUP-03",
            "PAL Ch14, Ch4.5.1; KAU Ch24; FABER ch16",
        )?;
        let br = |h: &str| book::range(cx, h);
        let (active, protos, nproto, stance) = (br("active_usd_yr"), br("protocols"), br("n_protocols"), br("stance"));
        let split = format!("{active}/({nproto}+({nproto}=0))");
        let total = format!("SUM({active})");

        // A. By protocol (rows: each protocol, then items with none).
        let mut names = protocols(d);
        names.push("(none)".into());
        let np = names.len() as u32;
        ws.write_string_with_format(
            HEAD - 1,
            0,
            "A. Annual spend by source protocol (multi-protocol items split equally)",
            &f.section,
        )?;
        table(ws, "tblSupCostProtocol", HEAD, 0, &["protocol", "n_active", "usd_yr", "share"], np)?;
        let attr = |name_cell: &str, extra: &str| {
            if name_cell == "\"(none)\"" {
                format!("SUMPRODUCT(({nproto}=0)*{active}{extra})")
            } else {
                format!("SUMPRODUCT(ISNUMBER(SEARCH(\", \"&{name_cell}&\",\",\", \"&{protos}&\",\"))*{split}{extra})")
            }
        };
        for (i, p) in names.iter().enumerate() {
            let (r, xr) = (HEAD + 1 + i as u32, HEAD + 2 + i as u32);
            ws.write_string(r, 0, p)?;
            let key = if p == "(none)" { lit(p) } else { format!("$A{xr}") };
            let count = if p == "(none)" {
                format!("=SUMPRODUCT(({nproto}=0)*({active}>0))")
            } else {
                format!("=SUMPRODUCT(ISNUMBER(SEARCH(\", \"&$A{xr}&\",\",\", \"&{protos}&\",\"))*({active}>0))")
            };
            fx(ws, r, 1, &count, None)?;
            fx(ws, r, 2, &format!("={}", attr(&key, "")), Some(&f.money))?;
            fx(ws, r, 3, &format!("=IF({total}=0,0,C{xr}/{total})"), Some(&f.pct))?;
        }
        let a_last = HEAD + 1 + np; // Excel row of the last protocol row
        theme::cf_data_bar(ws, HEAD + 1, 2, HEAD + np, 2, "9BC2E6")?;

        // Waterfall data next to it: protocol bars floating on a hidden base, then the total.
        ws.write_string_with_format(HEAD - 1, 5, "Waterfall: total -> by protocol", &f.section)?;
        table(ws, "tblSupCostWaterfall", HEAD, 5, &["step", "base", "usd_yr"], np + 1)?;
        for i in 0..=np {
            let (r, xr) = (HEAD + 1 + i, HEAD + 2 + i);
            if i < np {
                fx(ws, r, 5, &format!("=A{xr}"), None)?;
                let base = if i == 0 { "=0".to_string() } else { format!("=G{}+H{}", xr - 1, xr - 1) };
                fx(ws, r, 6, &base, Some(&f.money))?;
                fx(ws, r, 7, &format!("=C{xr}"), Some(&f.money))?;
            } else {
                ws.write_string(r, 5, "Total")?;
                fx(ws, r, 6, "=0", Some(&f.money))?;
                fx(ws, r, 7, &format!("={total}"), Some(&f.money))?;
            }
        }
        let wf = (TITLE, HEAD + 1, 5u16, HEAD + 1 + np, 5u16);
        let chart = theme::hidden_base_stack(
            false,
            "Annual spend: protocol waterfall",
            wf,
            (TITLE, HEAD + 1, 6, HEAD + 1 + np, 6),
            &[("usd_yr", (TITLE, HEAD + 1, 7, HEAD + 1 + np, 7))],
        );
        ws.insert_chart(HEAD + np + 4, 5, &chart)?;

        // B. By stance, with the "no evidence" call-out.
        let mut r = a_last + 2;
        ws.write_string_with_format(
            r - 1,
            0,
            "B. Annual spend by research stance (from tblEvidence via Ref_Stance)",
            &f.section,
        )?;
        table(ws, "tblSupCostStance", r, 0, &["stance", "n_active", "usd_yr", "share"], STANCE_LABELS.len() as u32)?;
        let b0 = r + 2;
        for (i, s) in STANCE_LABELS.iter().enumerate() {
            let (rr, xr) = (r + 1 + i as u32, b0 + i as u32);
            ws.write_string(rr, 0, *s)?;
            fx(ws, rr, 1, &format!("=COUNTIFS({stance},A{xr},{active},\">0\")"), None)?;
            fx(ws, rr, 2, &format!("=SUMIFS({active},{stance},A{xr})"), Some(&f.money))?;
            fx(ws, rr, 3, &format!("=IF({total}=0,0,C{xr}/{total})"), Some(&f.pct))?;
        }
        r += STANCE_LABELS.len() as u32 + 2;
        let none_cell = format!("{TITLE}!$C${}", b0 + 3);
        ws.write_string_with_format(r, 0, "Spend with no evidence on file ($/yr)", &f.bad)?;
        fx(ws, r, 2, &format!("={none_cell}"), Some(&f.money))?;
        kpi(
            cx,
            "usd_no_evidence",
            "Active spend with no evidence on file ($/yr)",
            &none_cell,
            Some(theme::nf::MONEY0),
        )?;

        // C. Protocol x stance matrix.
        r += 2;
        ws.write_string_with_format(r - 1, 0, "C. Protocol x stance ($/yr)", &f.section)?;
        let mut hdr = vec!["protocol"];
        hdr.extend(STANCE_LABELS);
        table(ws, "tblSupCostMatrix", r, 0, &hdr, np)?;
        let hx = r + 1;
        for (i, p) in names.iter().enumerate() {
            let (rr, xr) = (r + 1 + i as u32, r + 2 + i as u32);
            ws.write_string(rr, 0, p)?;
            let key = if p == "(none)" { lit(p) } else { format!("$A{xr}") };
            for j in 0..STANCE_LABELS.len() as u16 {
                let sc = format!("{}${hx}", crate::workbook::col(1 + j));
                fx(ws, rr, 1 + j, &format!("={}", attr(&key, &format!("*({stance}={sc})"))), Some(&f.money0))?;
            }
        }
        theme::cf_two_color(ws, r + 1, 1, r + np, STANCE_LABELS.len() as u16, "FFFFFF", "F4B183")?;
        r += np + 3;

        // D. Spend KPIs (SUP-03).
        ws.write_string_with_format(r - 1, 0, "D. Spend KPIs", &f.section)?;
        let np_ = PRODUCTS.nrows(d);
        let (pp, pu, pin) =
            (PRODUCTS.range_n(np_, "price"), PRODUCTS.range_n(np_, "units_per_year"), PRODUCTS.range_n(np_, "in_use"));
        let k0 = r + 1;
        let rows: [(&str, String); 8] = [
            ("Supplements $/yr", format!("={total}")),
            ("Supplements $/month", format!("={total}/12")),
            ("Supplements $/day", format!("={total}/365.25")),
            ("Products in use $/yr", format!("=SUMPRODUCT(({pin}=\"YES\")*{pp}*{pu})")),
            ("Total $/yr", format!("=B{k0}+B{}", k0 + 3)),
            (
                "Budget $/yr (Params Sup_Budget_yr)",
                "=IF(ISNUMBER(Sup_Budget_yr),Sup_Budget_yr,\"set Sup_Budget_yr\")".into(),
            ),
            (
                "TER: total / budget",
                format!("=IF(ISNUMBER(Sup_Budget_yr),IF(Sup_Budget_yr>0,B{}/Sup_Budget_yr,\"\"),\"\")", k0 + 4),
            ),
            ("Active items", format!("=COUNTIF({},\"YES\")", br("taking"))),
        ];
        for (i, (label, fm)) in rows.iter().enumerate() {
            let fmt = if i == 6 {
                &f.pct
            } else if i == 7 {
                &f.int
            } else {
                &f.money
            };
            figure(ws, cx, r + i as u32, 0, label, fm, Some(fmt))?;
        }
        let cellk = |k: u32| format!("{TITLE}!$B${k}");
        kpi(cx, "usd_month", "Active supplement spend per month", &cellk(k0 + 1), Some(theme::nf::MONEY))?;
        kpi(cx, "usd_day", "Active supplement spend per day", &cellk(k0 + 2), Some(theme::nf::MONEY))?;
        kpi(
            cx,
            "ter",
            "Total spend / budget (TER)",
            &format!("IF(ISNUMBER({}),{},\"n/a\")", cellk(k0 + 6), cellk(k0 + 6)),
            Some(theme::nf::PCT),
        )?;
        r += 10;
        ws.write_string_with_format(r - 1, 0, "By time of day", &f.section)?;
        table(ws, "tblSupCostSlot", r, 0, &["time_of_day", "n_active", "usd_yr"], lists::TIMES.len() as u32 + 1)?;
        let (slot, kind) = (br("time_of_day"), br("kind"));
        for (i, t) in lists::TIMES.iter().chain(std::iter::once(&"")).enumerate() {
            let rr = r + 1 + i as u32;
            let crit = if t.is_empty() { "\"\"".to_string() } else { lit(t) };
            ws.write_string(rr, 0, if t.is_empty() { "(blank)" } else { t })?;
            fx(ws, rr, 1, &format!("=COUNTIFS({slot},{crit},{active},\">0\")"), None)?;
            fx(ws, rr, 2, &format!("=SUMIFS({active},{slot},{crit})"), Some(&f.money))?;
        }
        r += lists::TIMES.len() as u32 + 4;
        ws.write_string_with_format(r - 1, 0, "By kind (Rx / Nt / Sp)", &f.section)?;
        table(ws, "tblSupCostKind", r, 0, &["kind", "n_active", "usd_yr"], lists::TYPES.len() as u32 + 1)?;
        for (i, t) in lists::TYPES.iter().chain(std::iter::once(&"")).enumerate() {
            let rr = r + 1 + i as u32;
            let crit = if t.is_empty() { "\"\"".to_string() } else { lit(t) };
            ws.write_string(rr, 0, if t.is_empty() { "(blank)" } else { t })?;
            fx(ws, rr, 1, &format!("=COUNTIFS({kind},{crit},{active},\">0\")"), None)?;
            fx(ws, rr, 2, &format!("=SUMIFS({active},{kind},{crit})"), Some(&f.money))?;
        }

        pareto(ws, cx)?;
        set_widths(ws, &[34.0, 10.0, 12.0, 10.0, 10.0, 20.0, 10.0, 10.0, 3.0, 3.0, 3.0, 3.0, 6.0, 30.0, 11.0, 10.0])?;
        Ok(())
    }
}

/// E. Pareto: items by active spend, cumulative share, HHI and Gini.
fn pareto(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let br = |h: &str| book::range(cx, h);
    let (key, names, active) = (br("rank_key"), br("name"), br("active_usd_yr"));
    let rows = n(d);
    ws.write_string_with_format(HEAD - 1, PC, "E. Pareto by active spend", &f.section)?;
    table(ws, "tblSupPareto", HEAD, PC, &["k", "name", "usd_yr", "cum_share"], rows)?;
    let x0 = HEAD + 2;
    for k in 0..d.supplements.len() as u32 {
        let (r, xr) = (HEAD + 1 + k, x0 + k);
        ws.write_number(r, PC, f64::from(k + 1))?;
        fx(ws, r, PC + 1, &format!("=INDEX({names},MATCH(LARGE({key},M{xr}),{key},0))"), None)?;
        fx(ws, r, PC + 2, &format!("=INDEX({active},MATCH(LARGE({key},M{xr}),{key},0))"), Some(&f.money))?;
        fx(ws, r, PC + 3, &format!("=IF(SUM({active})=0,0,SUM($O${x0}:O{xr})/SUM({active}))"), Some(&f.pct))?;
    }
    let xl = x0 + rows - 1;
    let (kr, vr) = (format!("$M${x0}:$M${xl}"), format!("$O${x0}:$O${xl}"));
    let s = HEAD + rows + 3;
    let na = format!("COUNTIF({active},\">0\")");
    let stats: [(&str, String, &rust_xlsxwriter::Format); 4] = [
        ("HHI (sum of squared weights)", format!("=SUMPRODUCT({}^2)", br("weight")), &f.dec2),
        (
            "Gini (active items)",
            format!("=IF({na}<2,0,2*SUMPRODUCT(({kr}<={na})*({na}+1-{kr})*{vr})/({na}*SUM({vr}))-({na}+1)/{na})"),
            &f.dec2,
        ),
        ("Top-10 share of spend", format!("=IF(SUM({vr})=0,0,SUMPRODUCT(({kr}<=10)*{vr})/SUM({vr}))"), &f.pct),
        ("Active items", format!("={na}"), &f.int),
    ];
    for (i, (label, fm, fmt)) in stats.iter().enumerate() {
        figure(ws, cx, s + i as u32, PC + 1, label, fm, Some(fmt))?;
    }
    kpi(cx, "hhi", "Spend concentration (HHI)", &format!("{TITLE}!$O${}", s + 1), Some(theme::nf::DEC2))?;

    let top = rows.min(20);
    let mut bars = Chart::new(ChartType::Column);
    bars.add_series().set_name("usd_yr").set_categories((TITLE, HEAD + 1, PC + 1, HEAD + top, PC + 1)).set_values((
        TITLE,
        HEAD + 1,
        PC + 2,
        HEAD + top,
        PC + 2,
    ));
    let mut line = Chart::new(ChartType::Line);
    line.add_series()
        .set_name("cumulative share")
        .set_categories((TITLE, HEAD + 1, PC + 1, HEAD + top, PC + 1))
        .set_values((TITLE, HEAD + 1, PC + 3, HEAD + top, PC + 3))
        .set_secondary_axis(true);
    bars.combine(&line);
    bars.title().set_name("Pareto: top items by annual spend");
    bars.set_width(620).set_height(320);
    ws.insert_chart(s + 6, PC, &bars)?;
    Ok(())
}
