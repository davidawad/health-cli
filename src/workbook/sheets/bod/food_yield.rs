//! BOD-23 `Bod_Yield` (quant Q86; longevity.md D15, D16; BUILDABLE-NOW, GATE-FIELDS
//! passes: `tblNutrition` has calories, protein, price, yuka_score and portions).
//!
//! Row-aligned with `Data_Nutrition`: `$/g protein = price / (protein x portions)`
//! assuming `price` is per package, `portions` per package and `protein` / `calories`
//! per portion (blank when any is missing or zero); kcal per g protein; protein per
//! 100 kcal; Yuka score. Dominance: an item is on the frontier when no other item is
//! both cheaper per g protein AND fewer kcal per g protein (`COUNTIFS` of strictly
//! better items = 0). Rank by $/g protein (`COUNTIFS` of cheaper items + 1), data bars,
//! a scatter of $/g vs kcal/g protein and a scatter of Yuka vs protein density.
//! `tblProducts` has no macro columns, so products are not listed (they join when a
//! protein column appears there).

use rust_xlsxwriter::{Chart, ChartMarker, ChartMarkerType, ChartType, Worksheet};

use super::{at, head, kv, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::NUTRITION;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Yield";
const HR: u32 = TOP + 1;
const C_KCAL: u16 = 1;
const C_PROT: u16 = 2;
const C_PRICE: u16 = 3;
const C_PORT: u16 = 4;
const C_YUKA: u16 = 5;
const C_USD: u16 = 6;
const C_KPG: u16 = 7;
const C_P100: u16 = 8;
const C_FRONT: u16 = 9;
const C_RANK: u16 = 10;
const C_PLOT: u16 = 11;
const SC: u16 = 17;

/// Scatter of plot column `y` against plot column `x` over rows `r0..=r1`.
fn scatter(title: &str, xname: &str, yname: &str, r0: u32, r1: u32, x: u16, y: u16) -> Chart {
    let mut chart = Chart::new(ChartType::Scatter);
    chart
        .add_series()
        .set_name(yname)
        .set_categories((TITLE, r0, x, r1, x))
        .set_values((TITLE, r0, y, r1, y))
        .set_marker(ChartMarker::new().set_type(ChartMarkerType::Circle).set_size(7));
    chart.title().set_name(title);
    chart.x_axis().set_name(xname);
    chart.y_axis().set_name(yname);
    chart.legend().set_hidden();
    chart.show_na_as_empty_cell();
    chart.set_width(560).set_height(320);
    chart
}

pub struct FoodYield;

impl SheetModule for FoodYield {
    fn id(&self) -> &'static str {
        "bod-yield"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-nutrition"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Protein yield frontier, Yuka vs protein density, $/g protein",
            "BOD-23",
            "Q86, L-D15, L-D16 | KAU Ch23 (dominance frontier); OUT ch50:25; tblNutrition",
            Banner::Note(
                "Units: price per package, portions per package, calories and protein per portion. \
                 tblProducts has no macros: products join when a protein column appears there.",
            ),
        )?;
        let n = NUTRITION.nrows(cx.d);
        let (r0, r1) = (HR + 1, HR + n);
        let first = HR + 2;
        section(ws, cx, TOP, 0, "Nutrition items (row-aligned with tblNutrition)")?;
        let mirror = |h: &'static str| {
            let src = NUTRITION.range(cx.d, h);
            move |r: u32| format!("=IF(INDEX({src},{i})=\"\",\"\",INDEX({src},{i}))", i = r + 1 - first)
        };
        let (usd, kpg) = (rng(TITLE, C_USD, r0, r1), rng(TITLE, C_KPG, r0, r1));
        let num = |c: &str| format!("AND(ISNUMBER({c}),N({c})>0)");
        let mut cols = vec![
            Col::f("item", mirror("name")),
            Col::f("calories", mirror("calories")),
            Col::f("protein_g", mirror("protein")),
            Col::f("price", mirror("price")),
            Col::f("portions", mirror("portions")),
            Col::f("yuka", mirror("yuka_score")),
            Col::f("usd_per_g_protein", move |r| {
                let (p, q, n) = (at(C_PRICE, r), at(C_PROT, r), at(C_PORT, r));
                format!("=IF(AND(ISNUMBER({p}),{},{}),{p}/({q}*{n}),\"\")", num(&q), num(&n))
            }),
            Col::f("kcal_per_g_protein", move |r| {
                let (k, q) = (at(C_KCAL, r), at(C_PROT, r));
                format!("=IF(AND(ISNUMBER({k}),{}),{k}/{q},\"\")", num(&q))
            }),
            Col::f("protein_per_100kcal", move |r| {
                let (k, q) = (at(C_KCAL, r), at(C_PROT, r));
                format!("=IF(AND(ISNUMBER({q}),{}),100*{q}/{k},\"\")", num(&k))
            }),
        ];
        let (u2, k2) = (usd.clone(), kpg.clone());
        cols.push(Col::f("frontier", move |r| {
            let (u, k) = (at(C_USD, r), at(C_KPG, r));
            format!("=IF(AND(ISNUMBER({u}),ISNUMBER({k})),IF(COUNTIFS({u2},\"<\"&{u},{k2},\"<\"&{k})=0,1,0),\"\")")
        }));
        let u3 = usd.clone();
        cols.push(Col::f("rank_usd_per_g", move |r| {
            let u = at(C_USD, r);
            format!("=IF(ISNUMBER({u}),COUNTIFS({u3},\"<\"&{u})+1,\"\")")
        }));
        cols.push(super::plot_col("usd_plot", C_USD));
        cols.push(super::plot_col("kcal_g_plot", C_KPG));
        cols.push(super::plot_col("p100_plot", C_P100));
        cols.push(super::plot_col("yuka_plot", C_YUKA));
        table(ws, cx, "tblBodYield", HR, 0, &cols, n)?;
        super::fmt_rows(ws, r0, r1, C_PRICE, C_PRICE, theme::nf::MONEY)?;
        super::fmt_rows(ws, r0, r1, C_USD, C_USD, "$0.0000")?;
        super::fmt_rows(ws, r0, r1, C_KPG, C_P100, "0.0")?;
        theme::cf_data_bar(ws, r0, C_USD, r1, C_USD, theme::BAND_BLUEPRINT)?;
        let f = at(C_FRONT, r0 + 1);
        theme::cf_formula(ws, r0, 0, r1, 0, &format!("=${}{}=1", col(C_FRONT), r0 + 2), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, C_FRONT, r1, C_FRONT, &format!("={f}=1"), theme::OPTIMAL)?;
        theme::cf_three_color(ws, r0, C_YUKA, r1, C_YUKA, false)?;

        // Summary.
        let (names, rank, front, p100) = (
            rng(TITLE, 0, r0, r1),
            rng(TITLE, C_RANK, r0, r1),
            rng(TITLE, C_FRONT, r0, r1),
            rng(TITLE, C_P100, r0, r1),
        );
        let s = TOP;
        section(ws, cx, s, SC, "Summary")?;
        kv(ws, cx, s + 1, SC, "Items with $/g protein", &format!("=COUNT({usd})"), Some("0"))?;
        kv(ws, cx, s + 2, SC, "Items on the frontier", &format!("=SUM({front})"), Some("0"))?;
        kv(ws, cx, s + 3, SC, "Cheapest protein", &format!("=IFERROR(INDEX({names},MATCH(1,{rank},0)),\"\")"), None)?;
        kv(ws, cx, s + 4, SC, "  its $/g protein", &format!("=IF(COUNT({usd})=0,\"\",MIN({usd}))"), Some("$0.0000"))?;
        kv(
            ws,
            cx,
            s + 5,
            SC,
            "Densest protein (g/100 kcal)",
            &format!("=IF(COUNT({p100})=0,\"\",IFERROR(INDEX({names},MATCH(MAX({p100}),{p100},0)),\"\"))"),
            None,
        )?;
        kv(ws, cx, s + 6, SC, "  its g per 100 kcal", &format!("=IF(COUNT({p100})=0,\"\",MAX({p100}))"), Some("0.0"))?;
        ws.write_string_with_format(
            s + 8,
            SC,
            "Frontier: no other item is both cheaper per g protein and fewer kcal per g protein.",
            &cx.f.note,
        )?;

        let a = scatter(
            "$ per g protein vs kcal per g protein",
            "kcal per g protein",
            "$ per g protein",
            r0,
            r1,
            C_PLOT + 1,
            C_PLOT,
        );
        ws.insert_chart(s + 10, SC, &a)?;
        let b = scatter(
            "Yuka score vs protein per 100 kcal",
            "Yuka score",
            "g protein per 100 kcal",
            r0,
            r1,
            C_PLOT + 3,
            C_PLOT + 2,
        );
        ws.insert_chart(s + 28, SC, &b)?;
        set_widths(
            ws,
            &[24.0, 9.0, 9.0, 9.0, 9.0, 7.0, 12.0, 12.0, 12.0, 9.0, 10.0, 9.0, 9.0, 9.0, 9.0, 2.0, 2.0, 28.0, 20.0],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
