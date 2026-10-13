//! `Sup_Unit` (SUP-04) unit economics and `Sup_Churn` (SUP-05) churn ledger.
//!
//! Unit economics per supplement ($/dose, $/mg actually taken) are built now from
//! `tblSupp`; the cheapest-source-per-ingredient block reads `tblSuppIngredients`,
//! which is empty until ingredient amounts are recorded (GATE-FIELDS: placeholder).
//! The churn ledger needs `tblSupp.start_date` / `stop_date` (biomarker regimens; GATE-DATES placeholder without them).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{book, fx, head, rng, table, x, HEAD};
use crate::error::Result;
use crate::infra::data::SUPP_INGREDIENTS as ING;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const UNIT_TITLE: &str = "Sup_Unit";
pub const CHURN_TITLE: &str = "Sup_Churn";

const UNIT_HEADERS: &[&str] =
    &["name", "amount", "unit", "doses_per_wk", "usd_yr", "amount_per_yr", "usd_per_dose", "mg_factor", "usd_per_mg"];
/// First column of the ingredient block.
const IC: u16 = 11;

/// `Sup_Unit` cost per dose, aligned with the book.
pub fn usd_per_dose_range(cx: &Cx) -> String {
    rng(UNIT_TITLE, 6, cx.d)
}

pub struct Unit;

impl SheetModule for Unit {
    fn id(&self) -> &'static str {
        "sup-unit"
    }
    fn title(&self) -> &'static str {
        UNIT_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "Unit economics: $/dose, $/mg, cheapest source per ingredient", "SUP-04", "KAU Ch13 (quant Q54)")?;
        let rows = super::n(d);
        table(ws, "tblSupUnit", HEAD, 0, UNIT_HEADERS, rows)?;
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            for (c, h) in [(0u16, "name"), (1, "amount"), (2, "unit"), (3, "doses_per_wk"), (4, "usd_yr")] {
                let src = book::at(h, xr);
                fx(ws, r, c, &format!("=IF({src}=\"\",\"\",{src})"), if c == 4 { Some(&f.money) } else { None })?;
            }
            fx(
                ws,
                r,
                5,
                &format!("=IF(AND(ISNUMBER(B{xr}),ISNUMBER(D{xr})),B{xr}*D{xr}*365.25/7,\"\")"),
                Some(&f.dec1),
            )?;
            fx(
                ws,
                r,
                6,
                &format!("=IF(AND(ISNUMBER(D{xr}),N(D{xr})>0,E{xr}>0),E{xr}/(D{xr}*365.25/7),\"\")"),
                Some(&f.money),
            )?;
            fx(
                ws,
                r,
                7,
                &format!("=IF(C{xr}=\"mg\",1,IF(C{xr}=\"g\",1000,IF(OR(C{xr}=\"mcg\",C{xr}=\"ug\",C{xr}=\"µg\"),0.001,\"\")))"),
                None,
            )?;
            fx(
                ws,
                r,
                8,
                &format!("=IF(AND(ISNUMBER(H{xr}),ISNUMBER(F{xr}),N(F{xr})>0,E{xr}>0),E{xr}/(F{xr}*H{xr}),\"\")"),
                Some(&rust_xlsxwriter::Format::new().set_num_format("$0.0000")),
            )?;
        }
        theme::cf_data_bar(ws, HEAD + 1, 6, HEAD + rows, 6, "F4B183")?;

        // Cheapest source per ingredient (placeholder until tblSuppIngredients has rows).
        let ni = ING.nrows(d);
        theme::awaiting(ws, 3, "ingredient amounts per supplement/product in tblSuppIngredients (GATE-FIELDS)", f)?;
        ws.write_string_with_format(
            HEAD - 1,
            IC,
            "Cheapest source per ingredient (reads tblSuppIngredients)",
            &f.section,
        )?;
        let ih = [
            "source",
            "ingredient",
            "amount",
            "unit",
            "usd_per_serving",
            "usd_per_unit",
            "cheapest_usd_per_unit",
            "cheapest_source",
            "saving_usd_yr",
        ];
        table(ws, "tblSupUnitIngredient", HEAD, IC, &ih, ni)?;
        let (names, dpw) = (book::range(cx, "name"), book::range(cx, "doses_per_wk"));
        let per_dose = usd_per_dose_range(cx);
        let col = |k: u16| crate::workbook::col(IC + k);
        let (x0, xl) = (HEAD + 2, HEAD + 1 + ni);
        let cr = |k: u16| format!("${c}${x0}:${c}${xl}", c = col(k));
        for i in 0..ni {
            let (r, xr) = (HEAD + 1 + i, HEAD + 2 + i);
            for (k, h) in [(0u16, "supp_or_product"), (1, "ingredient"), (2, "amount"), (3, "unit")] {
                fx(ws, r, IC + k, &super::mirror(&super::data_cell(&ING, h, i)), None)?;
            }
            let (src, amt) = (format!("{}{xr}", col(0)), format!("{}{xr}", col(2)));
            fx(ws, r, IC + 4, &format!("=IFERROR(INDEX({per_dose},MATCH({src},{names},0)),\"\")"), Some(&f.money))?;
            let ps = format!("{}{xr}", col(4));
            fx(ws, r, IC + 5, &format!("=IF(AND(ISNUMBER({ps}),ISNUMBER({amt}),N({amt})>0),{ps}/{amt},\"\")"), None)?;
            let (ing, own) = (format!("{}{xr}", col(1)), format!("{}{xr}", col(5)));
            fx(ws, r, IC + 6, &format!("=IF({ing}=\"\",\"\",MINIFS({},{},{ing}))", cr(5), cr(1)), None)?;
            let best = format!("{}{xr}", col(6));
            fx(
                ws,
                r,
                IC + 7,
                &format!(
                    "=IF(OR({ing}=\"\",N({best})=0),\"\",INDEX({},SUMPRODUCT(MAX(({}={ing})*({}={best})*(ROW({})-{})))))",
                    cr(0),
                    cr(1),
                    cr(5),
                    cr(5),
                    x0 - 1
                ),
                None,
            )?;
            fx(
                ws,
                r,
                IC + 8,
                &format!(
                    "=IFERROR(IF(AND(ISNUMBER({own}),N({best})>0),({own}-{best})*{amt}*INDEX({dpw},MATCH({src},{names},0))*365.25/7,\"\"),\"\")"
                ),
                Some(&f.money),
            )?;
        }
        set_widths(
            ws,
            &[
                30.0, 9.0, 7.0, 10.0, 10.0, 12.0, 11.0, 9.0, 11.0, 3.0, 3.0, 24.0, 18.0, 9.0, 7.0, 12.0, 11.0, 12.0,
                20.0, 12.0,
            ],
        )?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

const CHURN_HEADERS: &[&str] = &[
    "name",
    "start_date",
    "stop_date",
    "weeks_on",
    "usd_wk",
    "unused_stock_usd",
    "washout_until",
    "readable_after_stop",
    "event_date",
    "cum_waste_usd",
];

pub struct Churn;

impl SheetModule for Churn {
    fn id(&self) -> &'static str {
        "sup-churn"
    }
    fn title(&self) -> &'static str {
        CHURN_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "Churn ledger: cost of starts and stops", "SUP-05", "PAL Ch11 (quant Q56)")?;
        super::dates_banner(
            ws,
            cx,
            3,
            "tblSupp.start_date / stop_date (GATE-DATES: biomarker regimens); stock left at a stop = Sup_Stock_wk weeks",
        )?;
        let rows = super::n(d);
        table(ws, "tblSupChurn", HEAD, 0, CHURN_HEADERS, rows)?;
        let (x0, xl) = (x(0), super::last(d));
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            for (c, h) in [(1u16, "start_date"), (2, "stop_date"), (3, "weeks_on")] {
                let src = book::at(h, xr);
                fx(ws, r, c, &format!("=IF({src}=\"\",\"\",{src})"), Some(if c == 3 { &f.dec1 } else { &f.date }))?;
            }
            fx(ws, r, 4, &format!("={}/(365.25/7)", book::at("usd_yr", xr)), Some(&f.money))?;
            fx(ws, r, 5, &format!("=IF(ISNUMBER(C{xr}),E{xr}*Sup_Stock_wk,0)"), Some(&f.money))?;
            fx(ws, r, 6, &format!("=IF(ISNUMBER(B{xr}),B{xr}+7*Sup_Washout_wk,\"\")"), Some(&f.date))?;
            fx(ws, r, 7, &format!("=IF(ISNUMBER(C{xr}),C{xr}+7*Sup_Washout_wk,\"\")"), Some(&f.date))?;
            fx(ws, r, 8, &format!("=IF(ISNUMBER(C{xr}),C{xr},IF(ISNUMBER(B{xr}),B{xr},\"\"))"), Some(&f.date))?;
            fx(
                ws,
                r,
                9,
                &format!("=IF(ISNUMBER(I{xr}),SUMIFS($F${x0}:$F${xl},$I${x0}:$I${xl},\"<=\"&I{xr}),\"\")"),
                Some(&f.money),
            )?;
        }
        let s = HEAD + rows + 3;
        super::figure(ws, cx, s, 0, "Dated starts/stops on file", &format!("=COUNT($I${x0}:$I${xl})"), None)?;
        super::figure(ws, cx, s + 1, 0, "Total unused stock ($)", &format!("=SUM($F${x0}:$F${xl})"), Some(&f.money))?;
        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        chart
            .add_series()
            .set_name("cumulative waste")
            .set_categories((CHURN_TITLE, HEAD + 1, 8, HEAD + rows, 8))
            .set_values((CHURN_TITLE, HEAD + 1, 9, HEAD + rows, 9));
        chart.title().set_name("Cumulative churn waste ($)");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.set_width(560).set_height(280);
        ws.insert_chart(s + 3, 0, &chart)?;
        set_widths(ws, &[30.0, 11.0, 11.0, 9.0, 9.0, 12.0, 12.0, 14.0, 11.0, 12.0])?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}
