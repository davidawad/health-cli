//! SCH-08 `Sch_PillLoad`: pill burden and regimen complexity. Per supplement taken:
//! doses per week from an editable frequency map, slots from `Sch_Dosing`, dosing
//! events and pills per day, and a complexity score (items, non-daily schedules,
//! food instructions, distinct slots; weights are heuristic Params after KAT
//! ch81.md:91 "fewer doses per day"). A slot x kind crowding heatmap and the
//! turnover per quarter (starts + stops over active items, PAL Ch5.3.1), which waits
//! on `tblSupp.start_date` / `stop_date` (GATE-DATES).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::dosing::{self, SLOTS};
use super::{dcell, head, kpi, kv, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::store::Rec;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_PillLoad";
const FC: u16 = 14;
const QUARTERS: u32 = 8;

pub struct PillLoad;

/// Doses per week guessed from a frequency text (the map is editable).
pub fn per_week(freq: &str) -> f64 {
    let l = freq.to_lowercase();
    let lead: Option<f64> =
        l.split(|c: char| !c.is_ascii_digit() && c != '.').find(|s| !s.is_empty()).and_then(|s| s.parse().ok());
    if l.contains("other day") {
        3.5
    } else if l.contains("bid") || l.contains("twice daily") || l.contains("/day") || l.contains("x/d") {
        7.0 * lead.unwrap_or(2.0)
    } else if l.contains("wk") || l.contains("week") {
        lead.unwrap_or(1.0)
    } else if l.contains("month") || l.contains("/mo") {
        lead.unwrap_or(1.0) * 12.0 / 52.0
    } else {
        7.0
    }
}

fn freq_map(d: &Data) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = Vec::new();
    for t in
        d.supplements.iter().filter_map(|s| s.s("frequency")).chain(["Daily", "Weekly", "2x/day"].map(String::from))
    {
        if !out.iter().any(|(x, _)| x.eq_ignore_ascii_case(&t)) {
            let w = per_week(&t);
            out.push((t, w));
        }
    }
    out
}

fn kinds(d: &Data) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for k in d.supplements.iter().filter_map(|s| s.s("kind")) {
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

impl SheetModule for PillLoad {
    fn id(&self) -> &'static str {
        "sch-pillload"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["sch-dosing"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Pill burden, regimen complexity, slot crowding, turnover",
            "SCH-08 (C6.2, Q55) | KAT ch81.md:91,147-161; PAL Ch5.3.1 (turnover); DIP ch29.md:21 (polypharmacy)",
            Banner::Awaiting(
                "supplement start/stop dates for the turnover trend (GATE-DATES); burden and complexity compute now",
            ),
        )?;
        // Frequency map.
        let fm = freq_map(d);
        let nf = fm.len() as u32;
        let fr = TOP + 1;
        section(ws, cx, fr - 1, FC, "Frequency map (doses per week)")?;
        table(ws, cx, "tblSchFreq", fr, FC, &[Col::In("frequency"), Col::In("per_week")], nf)?;
        for (i, (t, w)) in fm.iter().enumerate() {
            let r = fr + 1 + i as u32;
            ws.write_string_with_format(r, FC, t, &f.input)?;
            ws.write_number_with_format(r, FC + 1, *w, &f.input)?;
        }
        let (ff, fw) = (rng(TITLE, FC, fr + 1, fr + nf), rng(TITLE, FC + 1, fr + 1, fr + nf));

        // Per supplement.
        let ns = SUPP.nrows(d);
        let pr = TOP + 8;
        let (p0, p1) = (pr + 1, pr + ns);
        section(ws, cx, pr - 1, 0, "Per supplement (row-aligned with tblSupp and Sch_Dosing)")?;
        let cols = [
            Col::f("supplement", |_, i| mirror(&dcell(&SUPP, "name", i))),
            Col::f("taking", |_, i| mirror(&dcell(&SUPP, "taking", i))),
            Col::f("kind", |_, i| mirror(&dcell(&SUPP, "kind", i))),
            Col::f("frequency", |_, i| mirror(&dcell(&SUPP, "frequency", i))),
            Col::f("per_week", |r, _| format!("=IF(B{r}<>\"YES\",0,IFERROR(INDEX({fw},MATCH(D{r},{ff},0)),7))")),
            Col::f("slots", |_, i| {
                let t: Vec<String> =
                    (0..SLOTS.len()).map(|k| format!("COUNTIF({},\"?*\")", dosing::slot_cell(d, i, k))).collect();
                format!("={}", t.join("+"))
            }),
            Col::In("pills_per_dose"),
            Col::f("events_per_day", |r, _| format!("=IF(B{r}<>\"YES\",0,E{r}/7*MAX(1,F{r}))")),
            Col::f("pills_per_day", |r, _| format!("=H{r}*IF(ISNUMBER(G{r}),G{r},1)")),
            Col::f("non_daily", |r, _| format!("=IF(AND(B{r}=\"YES\",E{r}<7),1,0)")),
            Col::f("food_rule", |r, i| format!("=IF(AND(B{r}=\"YES\",{}<>\"\"),1,0)", dosing::food_cell(d, i))),
            Col::f("complexity", |r, _| {
                format!("=IF(B{r}<>\"YES\",0,Sch_CX_item+Sch_CX_nondaily*J{r}+Sch_CX_food*K{r})")
            }),
        ];
        table(ws, cx, "tblSchPillLoad", pr, 0, &cols, ns)?;
        for c in [7, 8] {
            super::fmt_rows(ws, p0, p1, c, theme::nf::DEC2)?;
        }
        theme::cf_data_bar(ws, p0, 8, p1, 8, theme::BAND_BLUEPRINT)?;
        let pr_rng = |c: u16| rng(TITLE, c, p0, p1);

        // Slot x kind heatmap (pills per day).
        let ks = kinds(d);
        let hr = p1 + 3;
        section(ws, cx, hr - 1, 0, "Slot crowding: pills per day by slot and kind")?;
        ws.write_string_with_format(hr, 0, "slot", &f.header)?;
        for (j, k) in ks.iter().enumerate() {
            ws.write_string_with_format(hr, 1 + j as u16, k, &f.header)?;
        }
        let tc = 1 + ks.len() as u16;
        ws.write_string_with_format(hr, tc, "all kinds", &f.header)?;
        let (kind, ppd) = (pr_rng(2), pr_rng(8));
        for (s, slot) in SLOTS.iter().enumerate() {
            let r = hr + 1 + s as u32;
            let sr = dosing::slot_range(d, s);
            ws.write_string(r, 0, *slot)?;
            for j in 0..ks.len() as u16 {
                let k = format!("{}${}", col(1 + j), hr + 1);
                ws.write_formula(r, 1 + j, format!("=SUMPRODUCT(({kind}={k})*({sr}<>\"\")*{ppd})").as_str())?;
            }
            ws.write_formula(r, tc, format!("=SUMPRODUCT(({sr}<>\"\")*{ppd})").as_str())?;
        }
        let h1 = hr + SLOTS.len() as u32;
        if !ks.is_empty() {
            theme::cf_three_color(ws, hr + 1, 1, h1, tc - 1, true)?;
        }
        theme::cf_formula(
            ws,
            hr + 1,
            tc,
            h1,
            tc,
            &format!("={}>Sch_Slot_crowd", super::at(tc, hr + 2)),
            theme::CRITICAL,
        )?;

        // Headline figures.
        let tot = rng(TITLE, tc, hr + 1, h1);
        let slots_used = format!("COUNTIF({tot},\">0\")");
        kv(ws, cx, TOP, 0, "Items taken", &format!("=COUNTIF({},\"YES\")", pr_rng(1)), None)?;
        kv(ws, cx, TOP + 1, 0, "Pills per day", &format!("=SUM({})", pr_rng(8)), Some(theme::nf::DEC1))?;
        kv(ws, cx, TOP + 2, 0, "Dosing events per day", &format!("=SUM({})", pr_rng(7)), Some(theme::nf::DEC1))?;
        kv(ws, cx, TOP + 3, 0, "Distinct slots used", &format!("={slots_used}"), None)?;
        kv(
            ws,
            cx,
            TOP + 4,
            0,
            "Regimen complexity score",
            &format!("=SUM({})+Sch_CX_slot*{slots_used}", pr_rng(11)),
            Some(theme::nf::DEC1),
        )?;
        kpi(cx, "pills_day", "Pills per day", TITLE, TOP + 1, 1, Some(theme::nf::DEC1))?;
        kpi(cx, "complexity", "Regimen complexity score", TITLE, TOP + 4, 1, Some(theme::nf::DEC1))?;

        // Turnover per quarter (GATE-DATES).
        let qr = h1 + 3;
        section(ws, cx, qr - 2, 0, "Turnover per quarter")?;
        theme::awaiting(ws, qr - 1, "tblSupp.start_date / stop_date (empty in the store)", f)?;
        let n = SUPP.nrows(d);
        let (st, sp) = (SUPP.range_n(n, "start_date"), SUPP.range_n(n, "stop_date"));
        let cols = [
            Col::f("quarter_start", |_, i| {
                format!("=DATE(YEAR(Today),INT((MONTH(Today)-1)/3)*3+1-3*{},1)", QUARTERS - 1 - i)
            }),
            Col::f("starts", |r, _| format!("=COUNTIFS({st},\">=\"&A{r},{st},\"<\"&EDATE(A{r},3))")),
            Col::f("stops", |r, _| format!("=COUNTIFS({sp},\">=\"&A{r},{sp},\"<\"&EDATE(A{r},3))")),
            Col::f("active_at_end", |r, _| {
                format!("=COUNTIFS({st},\"<\"&EDATE(A{r},3))-COUNTIFS({sp},\"<\"&EDATE(A{r},3))")
            }),
            Col::f("turnover", |r, _| format!("=IF(D{r}<=0,\"\",(B{r}+C{r})/D{r})")),
        ];
        table(ws, cx, "tblSchTurnover", qr, 0, &cols, QUARTERS)?;
        let (q0, q1) = (qr + 1, qr + QUARTERS);
        super::fmt_rows(ws, q0, q1, 0, theme::nf::DATE)?;
        super::fmt_rows(ws, q0, q1, 4, theme::nf::PCT)?;
        let mut chart = Chart::new(ChartType::Line);
        chart.add_series().set_name("turnover").set_categories((TITLE, q0, 0, q1, 0)).set_values((TITLE, q0, 4, q1, 4));
        chart.title().set_name("Stack turnover per quarter");
        chart.x_axis().set_num_format("mmm yy");
        chart.set_width(460).set_height(240);
        ws.insert_chart(qr, 7, &chart)?;
        let mut heat = Chart::new(ChartType::Column);
        heat.add_series().set_name("pills per day").set_categories((TITLE, hr + 1, 0, h1, 0)).set_values((
            TITLE,
            hr + 1,
            tc,
            h1,
            tc,
        ));
        heat.title().set_name("Pills per day by slot");
        heat.legend().set_hidden();
        heat.set_width(380).set_height(220);
        ws.insert_chart(hr - 1, tc + 2, &heat)?;
        set_widths(ws, &[26.0, 8.0, 8.0, 10.0, 9.0, 7.0, 9.0, 10.0, 10.0, 9.0, 9.0, 10.0])?;
        ws.set_column_width(FC, 16.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::per_week;

    #[test]
    fn frequencies_become_doses_per_week() {
        assert_eq!(per_week("Daily"), 7.0);
        assert_eq!(per_week("2x/wk"), 2.0);
        assert_eq!(per_week("3/wk"), 3.0);
        assert_eq!(per_week("Weekly"), 1.0);
        assert_eq!(per_week("2x/day"), 14.0);
        assert_eq!(per_week("every other day"), 3.5);
    }
}
