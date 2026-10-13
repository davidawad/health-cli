//! SCH-06 `Sch_PreDraw`: pre-draw planner. Biotin per supplement (name match or
//! `tblSuppIngredients`, mg -> mcg) summed over what is taken; for each planned draw
//! the biotin stop-by time (`Sch_Biotin_hold_h`, DIP ch98.md:233) with a countdown and
//! the fasting hours against `Sch_Fast_h` (KAT ch46.md:77); per past panel (`tblPanels`)
//! a "biotin within the hold window" flag and the immunoassays on it; a prep checklist.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{dcell, fmt_rows, head, kv, list, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{LABS, PANELS, SUPP, SUPP_INGREDIENTS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_PreDraw";
const UR: u32 = TOP + 5;
const PLAN_ROWS: u32 = 8;
/// Biotin-by-supplement block first column.
const BC: u16 = 15;
/// Immunoassay markers flagged on a past panel (streptavidin-biotin platforms).
const IMMUNO: [&str; 4] = ["tsh", "ft4", "ft3", "troponin"];
const CHECKS: [(&str, &str); 5] = [
    ("Fast 10 h before a lipid panel", "KAT ch46.md:77"),
    ("A1c needs no fast", "DIP ch97.md:197"),
    ("Hold biotin 24-48 h (more if > 5,000 mcg/day)", "DIP ch98.md:233"),
    ("Same lab and method as last time", "heuristic"),
    ("Bring the medication list (Sch_MedRec)", "DIP ch07"),
];

pub struct PreDraw;

impl SheetModule for PreDraw {
    fn id(&self) -> &'static str {
        "sch-predraw"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(
            ws,
            cx,
            "Before the next blood draw: biotin hold, fasting, prep",
            "SCH-06 (C5.1, C5.11) | DIP ch98.md:233; KAT ch46.md:77; DIP ch97.md:197; FDA 2017/2019 biotin communication (EXT)",
            Banner::Note("Past panels have no biotin dose dates (GATE-DATES): type the last dose to resolve the flag."),
        )?;
        // Biotin by supplement (row-aligned with tblSupp).
        let ns = SUPP.nrows(d);
        let (b0, b1) = (UR + 1, UR + ns);
        let ni = SUPP_INGREDIENTS.nrows(d);
        let (is, ii, ia, iu) = (
            SUPP_INGREDIENTS.range_n(ni, "supp_or_product"),
            SUPP_INGREDIENTS.range_n(ni, "ingredient"),
            SUPP_INGREDIENTS.range_n(ni, "amount"),
            SUPP_INGREDIENTS.range_n(ni, "unit"),
        );
        section(ws, cx, UR - 1, BC, "Biotin by supplement (tblSupp, tblSuppIngredients)")?;
        let cols = [
            Col::f("supplement", |_, i| mirror(&dcell(&SUPP, "name", i))),
            Col::f("taking", |_, i| mirror(&dcell(&SUPP, "taking", i))),
            Col::f("amount", |_, i| mirror(&dcell(&SUPP, "amount", i))),
            Col::f("unit", |_, i| mirror(&dcell(&SUPP, "unit", i))),
            Col::f("biotin_mcg_per_dose", |r, _| {
                let ing = |u: &str| format!("SUMIFS({ia},{is},P{r},{ii},\"*biotin*\",{iu},\"{u}\")");
                format!(
                    "=IF(P{r}=\"\",0,IF(AND(ISNUMBER(SEARCH(\"biotin\",P{r})),ISNUMBER(R{r})),R{r}*IF(LOWER(S{r})=\"mg\",1000,IF(LOWER(S{r})=\"mcg\",1,0)),0)+{}+1000*{})",
                    ing("mcg"),
                    ing("mg")
                )
            }),
            Col::f("biotin_mcg_taken", |r, _| format!("=IF(Q{r}=\"YES\",T{r},0)")),
        ];
        table(ws, cx, "tblSchBiotin", UR, BC, &cols, ns)?;
        let total = format!("SUM({})", rng(TITLE, BC + 5, b0, b1));
        kv(ws, cx, TOP, 0, "Biotin taken (mcg per dose day)", &format!("={total}"), Some("#,##0"))?;
        kv(
            ws,
            cx,
            TOP + 1,
            0,
            "Above the interference level?",
            &format!("=IF({total}>Sch_Biotin_mcg,\"yes: above \"&TEXT(Sch_Biotin_mcg,\"#,##0\")&\" mcg/day\",\"no\")"),
            None,
        )?;
        let biotin = super::cell(TITLE, TOP, 1);

        // Planned draws.
        section(ws, cx, UR - 1, 0, "Planned draws (blue cells are yours)")?;
        let cols = [
            Col::Date("draw_date"),
            Col::In("draw_hour"),
            Col::In("tests"),
            Col::In("fasting_required"),
            Col::In("last_biotin_dose"),
            Col::In("last_meal"),
            Col::f("draw_at", |r, _| format!("=IF(ISNUMBER(A{r}),A{r}+IF(ISNUMBER(B{r}),B{r},Sch_Draw_h)/24,\"\")")),
            Col::f("biotin_stop_by", |r, _| format!("=IF(G{r}=\"\",\"\",G{r}-Sch_Biotin_hold_h/24)")),
            Col::f("days_to_stop", |r, _| format!("=IF(H{r}=\"\",\"\",H{r}-Today)")),
            Col::f("biotin", |r, _| {
                format!(
                    "=IF(G{r}=\"\",\"\",IF(AND({biotin}=0,E{r}=\"\"),\"no biotin in stack\",IF(ISNUMBER(E{r}),IF(E{r}>H{r},\"biotin within hold: annotate immunoassays\",\"held\"),IF(I{r}<0,\"hold window open: no biotin now\",\"stop biotin in \"&TEXT(I{r},\"0.0\")&\" d\"))))"
                )
            }),
            Col::f("fasting_h", |r, _| format!("=IF(AND(ISNUMBER(F{r}),ISNUMBER(G{r})),(G{r}-F{r})*24,\"\")")),
            Col::f("fasting", |r, _| {
                format!("=IF(D{r}<>\"YES\",\"\",IF(K{r}=\"\",\"enter last meal\",IF(K{r}>=Sch_Fast_h,\"ok\",\"fast too short\")))")
            }),
        ];
        table(ws, cx, "tblSchDraws", UR, 0, &cols, PLAN_ROWS)?;
        let (p0, p1) = (UR + 1, UR + PLAN_ROWS);
        list(ws, p0, 3, p1, &["YES", "NO"])?;
        for c in [4, 5, 6, 7] {
            fmt_rows(ws, p0, p1, c, "yyyy-mm-dd hh:mm")?;
        }
        fmt_rows(ws, p0, p1, 8, theme::nf::DEC1)?;
        fmt_rows(ws, p0, p1, 10, theme::nf::DEC1)?;
        for (c, bad) in [(9u16, "biotin within hold: annotate immunoassays"), (11, "fast too short")] {
            let tl = super::at(c, p0 + 1);
            theme::cf_formula(ws, p0, c, p1, c, &format!("={tl}=\"{bad}\""), theme::CRITICAL)?;
        }
        let dates = rng(TITLE, 0, p0, p1);
        kv(
            ws,
            cx,
            TOP + 2,
            0,
            "Next planned draw",
            &format!("=IF(COUNTIFS({dates},\">=\"&Today)=0,\"\",MINIFS({dates},{dates},\">=\"&Today))"),
            Some(theme::nf::DATE),
        )?;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("days until biotin stop")
            .set_categories((TITLE, p0, 0, p1, 0))
            .set_values((TITLE, p0, 8, p1, 8));
        chart.title().set_name("Biotin stop countdown per planned draw");
        chart.x_axis().set_num_format("yyyy-mm-dd");
        chart.set_width(480).set_height(240);
        ws.insert_chart(p1 + 2, 0, &chart)?;

        // Past panels.
        let np = PANELS.nrows(d);
        let pr = p1.max(b1) + 16;
        let (q0, q1) = (pr + 1, pr + np);
        let nl = LABS.nrows(d);
        let (lp, lm) = (LABS.range_n(nl, "panel_id"), LABS.range_n(nl, "marker_id"));
        section(ws, cx, pr - 1, 0, "Past panels (tblPanels)")?;
        let cols = [
            Col::f("panel_id", |_, i| mirror(&dcell(&PANELS, "panel_id", i))),
            Col::f("date", |_, i| mirror(&dcell(&PANELS, "date", i))),
            Col::f("fasting", |_, i| mirror(&dcell(&PANELS, "fasting", i))),
            Col::f("immunoassays", |r, _| {
                let terms: Vec<String> = IMMUNO.iter().map(|m| format!("COUNTIFS({lp},A{r},{lm},\"{m}\")")).collect();
                format!("=IF(A{r}=\"\",\"\",{})", terms.join("+"))
            }),
            Col::In("last_biotin_dose"),
            Col::f("biotin_flag", |r, _| {
                format!(
                    "=IF(A{r}=\"\",\"\",IF(ISNUMBER(E{r}),IF(AND(E{r}>=B{r}-Sch_Biotin_hold_h/24,E{r}<=B{r}+1),\"biotin within hold window\",\"clear\"),IF({biotin}>0,\"unknown: no dose dates\",\"no biotin in stack\")))"
                )
            }),
        ];
        table(ws, cx, "tblSchPastDraws", pr, 0, &cols, np)?;
        fmt_rows(ws, q0, q1, 1, theme::nf::DATE)?;
        fmt_rows(ws, q0, q1, 4, "yyyy-mm-dd hh:mm")?;
        let tl = super::at(5, q0 + 1);
        theme::cf_formula(ws, q0, 5, q1, 5, &format!("={tl}=\"biotin within hold window\""), theme::CRITICAL)?;
        theme::cf_formula(ws, q0, 5, q1, 5, &format!("=LEFT({tl},7)=\"unknown\""), theme::PLACEHOLDER)?;

        // Prep checklist.
        let cr = q1 + 3;
        section(ws, cx, cr - 1, 0, "Prep checklist")?;
        let cols = [Col::In("item"), Col::In("source"), Col::In("done")];
        table(ws, cx, "tblSchPrep", cr, 0, &cols, CHECKS.len() as u32)?;
        for (i, (item, src)) in CHECKS.iter().enumerate() {
            let r = cr + 1 + i as u32;
            ws.write_string(r, 0, *item)?;
            ws.write_string(r, 1, *src)?;
        }
        let (k0, k1) = (cr + 1, cr + CHECKS.len() as u32);
        list(ws, k0, 2, k1, &["YES", "NO"])?;
        kv(
            ws,
            cx,
            k1 + 2,
            0,
            "Prep done",
            &format!("=COUNTIF({},\"YES\")/{}", rng(TITLE, 2, k0, k1), CHECKS.len()),
            Some(theme::nf::PCT),
        )?;
        set_widths(ws, &[30.0, 16.0, 16.0, 10.0, 16.0, 16.0, 16.0, 16.0, 9.0, 34.0, 9.0, 14.0])?;
        ws.set_column_width(BC, 24.0)?;
        Ok(())
    }
}
