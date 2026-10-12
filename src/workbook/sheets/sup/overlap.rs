//! `Sup_Overlap` (SUP-09) stacked-dose detector by active ingredient, `Sup_DRI`
//! (SUP-10) micronutrient upper-limit ledger plus the WHO reference-dose / Rx /
//! supplement tag checker, and `Ref_UL` (the UL rule table, UNVERIFIED until checked
//! against NIH ODS / IOM). Ingredient amounts come from `tblSuppIngredients`, empty
//! until recorded (GATE-FIELDS): the ingredient blocks are placeholders, the
//! name-overlap check and the WHO tag checker work now.

use rust_xlsxwriter::Worksheet;

use super::{book, data_cell, fx, head, inputs, match_idx, mirror, rule_rows, table, write_rules, x, Rule, HEAD};
use crate::error::Result;
use crate::infra::data::{TableDef, PRODUCTS, SUPP_INGREDIENTS as ING, V};
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const OVERLAP_TITLE: &str = "Sup_Overlap";
pub const DRI_TITLE: &str = "Sup_DRI";

const UL_SRC: &str = "pending: NIH ODS / IOM DRI tables, adults 19+";
const UL_STATE: &str = "UNVERIFIED";
/// Upper limits (UL) per day, adults: typed from the IOM tables, not yet re-checked.
pub const UL: &[Rule] = &[
    ("UL-VIT-A", "vitamin a", Some(3000.0), "mcg", UL_SRC, UL_STATE, "Vitamin A (preformed retinol, mcg RAE)"),
    ("UL-VIT-D", "vitamin d", Some(4000.0), "IU", UL_SRC, UL_STATE, "Vitamin D (100 mcg)"),
    ("UL-VIT-E", "vitamin e", Some(1000.0), "mg", UL_SRC, UL_STATE, "Vitamin E (supplemental alpha-tocopherol)"),
    ("UL-VIT-C", "vitamin c", Some(2000.0), "mg", UL_SRC, UL_STATE, "Vitamin C"),
    ("UL-NIACIN", "niacin", Some(35.0), "mg", UL_SRC, UL_STATE, "Niacin (supplemental forms)"),
    ("UL-B6", "b6", Some(100.0), "mg", UL_SRC, UL_STATE, "Vitamin B6"),
    ("UL-FOLATE", "fol", Some(1000.0), "mcg", UL_SRC, UL_STATE, "Folic acid (synthetic, mcg)"),
    ("UL-CALCIUM", "calcium", Some(2500.0), "mg", UL_SRC, UL_STATE, "Calcium (19-50 y; 2000 mg from 51 y)"),
    ("UL-IRON", "iron", Some(45.0), "mg", UL_SRC, UL_STATE, "Iron"),
    ("UL-ZINC", "zinc", Some(40.0), "mg", UL_SRC, UL_STATE, "Zinc"),
    ("UL-SELENIUM", "selenium", Some(400.0), "mcg", UL_SRC, UL_STATE, "Selenium"),
    ("UL-MAGNESIUM", "magnesium", Some(350.0), "mg", UL_SRC, UL_STATE, "Magnesium (supplemental only)"),
    ("UL-IODINE", "iodine", Some(1100.0), "mcg", UL_SRC, UL_STATE, "Iodine"),
    ("UL-COPPER", "copper", Some(10000.0), "mcg", UL_SRC, UL_STATE, "Copper"),
    ("UL-MANGANESE", "manganese", Some(11.0), "mg", UL_SRC, UL_STATE, "Manganese"),
    ("UL-CHOLINE", "choline", Some(3500.0), "mg", UL_SRC, UL_STATE, "Choline"),
    ("UL-BORON", "boron", Some(20.0), "mg", UL_SRC, UL_STATE, "Boron"),
    ("UL-MOLYBDENUM", "molybdenum", Some(2000.0), "mcg", UL_SRC, UL_STATE, "Molybdenum"),
];

const WHO_SRC: &str = "WHO ch00.md:4346-4420 (Model List 2019, section 27)";
/// WHO Model List reference strengths (vitamins and minerals), as quoted in the clinical catalog.
pub const WHO: &[Rule] = &[
    ("WHO-COLECALCIFEROL", "vitamin d", Some(1000.0), "IU", WHO_SRC, "verified", "colecalciferol 400 / 1000 IU"),
    ("WHO-ERGOCALCIFEROL", "ergocalciferol", Some(50000.0), "IU", WHO_SRC, "verified", "ergocalciferol 50,000 IU"),
    ("WHO-NICOTINAMIDE", "nicotinamide", Some(50.0), "mg", WHO_SRC, "verified", "nicotinamide 50 mg"),
    ("WHO-PYRIDOXINE", "pyridoxine", Some(25.0), "mg", WHO_SRC, "verified", "pyridoxine 25 mg"),
    ("WHO-B6", "b6", Some(25.0), "mg", WHO_SRC, "verified", "pyridoxine 25 mg"),
    ("WHO-RIBOFLAVIN", "riboflavin", Some(5.0), "mg", WHO_SRC, "verified", "riboflavin 5 mg"),
    ("WHO-THIAMINE", "thiamine", Some(50.0), "mg", WHO_SRC, "verified", "thiamine 50 mg"),
    ("WHO-ASCORBIC", "vitamin c", Some(50.0), "mg", WHO_SRC, "verified", "ascorbic acid 50 mg"),
    ("WHO-CALCIUM", "calcium", Some(500.0), "mg", WHO_SRC, "verified", "calcium 500 mg"),
    ("WHO-IODINE", "iodine", None, "", WHO_SRC, "verified", "iodine (strength not quoted)"),
    ("WHO-RETINOL", "retinol", None, "", WHO_SRC, "verified", "retinol (strength not quoted)"),
];

fn ul_rows(_: &Data) -> Vec<Vec<V>> {
    rule_rows(UL)
}

pub static REF_UL: TableDef = TableDef {
    id: "ref-ul",
    sheet: "Ref_UL",
    table: "tblRefUL",
    headers: RULE_HEADERS,
    rows: ul_rows,
    needs_labs: false,
};

const OV_HEADERS: &[&str] =
    &["source", "ingredient", "amount", "unit", "doses_per_day", "daily_amount", "n_sources", "total_daily", "flag"];
/// First column of the name-overlap block.
const NC: u16 = 11;

fn ov_rng(d: &Data, c: u16) -> String {
    crate::workbook::range(OVERLAP_TITLE, c, HEAD + 2, HEAD + 1 + ING.nrows(d))
}

pub struct Overlap;

impl SheetModule for Overlap {
    fn id(&self) -> &'static str {
        "sup-overlap"
    }
    fn title(&self) -> &'static str {
        OVERLAP_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "Redundancy and stacked doses by active ingredient", "SUP-09", "PAL Ch4; KAU Ch24 (quant Q48)")?;
        theme::awaiting(
            ws,
            3,
            "ingredient amounts in tblSuppIngredients (GATE-FIELDS); UL comparison is on Sup_DRI",
            f,
        )?;
        let ni = ING.nrows(d);
        table(ws, "tblSupOverlap", HEAD, 0, OV_HEADERS, ni)?;
        let (names, dpw) = (book::range(cx, "name"), book::range(cx, "doses_per_wk"));
        let (x0, xl) = (HEAD + 2, HEAD + 1 + ni);
        for i in 0..ni {
            let (r, xr) = (HEAD + 1 + i, HEAD + 2 + i);
            for (k, h) in [(0u16, "supp_or_product"), (1, "ingredient"), (2, "amount"), (3, "unit")] {
                fx(ws, r, k, &mirror(&data_cell(&ING, h, i)), None)?;
            }
            fx(
                ws,
                r,
                4,
                &format!("=IF(A{xr}=\"\",\"\",IFERROR(INDEX({dpw},MATCH(A{xr},{names},0))/7,1))"),
                Some(&f.dec2),
            )?;
            fx(ws, r, 5, &format!("=IF(ISNUMBER(C{xr}),C{xr}*N(E{xr}),\"\")"), Some(&f.dec2))?;
            fx(ws, r, 6, &format!("=IF(B{xr}=\"\",\"\",COUNTIF($B${x0}:$B${xl},B{xr}))"), None)?;
            fx(
                ws,
                r,
                7,
                &format!("=IF(B{xr}=\"\",\"\",SUMIFS($F${x0}:$F${xl},$B${x0}:$B${xl},B{xr},$D${x0}:$D${xl},D{xr}))"),
                Some(&f.dec2),
            )?;
            fx(ws, r, 8, &format!("=IF(N(G{xr})>1,\"stacked: \"&G{xr}&\" sources\",\"\")"), None)?;
        }
        theme::cf_formula(ws, HEAD + 1, 8, HEAD + ni, 8, &format!("=$I{}<>\"\"", HEAD + 2), theme::OUT_OF_REF)?;

        // Works now: the same name twice in the stack, or both a supplement and a product.
        ws.write_string_with_format(HEAD - 1, NC, "Name overlap (works without ingredient data)", &f.section)?;
        table(
            ws,
            "tblSupNameOverlap",
            HEAD,
            NC,
            &["name", "taking", "times_in_stack", "also_a_product", "flag"],
            super::n(d),
        )?;
        let pn = PRODUCTS.range(d, "name");
        let c = |k: u16| col(NC + k);
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, NC, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, NC + 1, &mirror(&book::at("taking", xr)), None)?;
            fx(ws, r, NC + 2, &format!("=COUNTIF({names},{}{xr})", c(0)), None)?;
            fx(ws, r, NC + 3, &format!("=IF(COUNTIF({pn},{}{xr})>0,\"YES\",\"NO\")", c(0)), None)?;
            fx(
                ws,
                r,
                NC + 4,
                &format!("=IF({}{xr}>1,\"listed twice\",IF({}{xr}=\"YES\",\"also a product\",\"\"))", c(2), c(3)),
                None,
            )?;
        }
        set_widths(ws, &[24.0, 18.0, 9.0, 7.0, 10.0, 11.0, 9.0, 11.0, 18.0, 3.0, 3.0, 28.0, 8.0, 12.0, 12.0, 16.0])?;
        Ok(())
    }
}

const DRI_HEADERS: &[&str] = &[
    "nutrient",
    "keyword",
    "ul",
    "unit",
    "supplements_daily",
    "food_daily",
    "total_daily",
    "pct_of_ul",
    "flag",
    "source_state",
];
/// First column of the WHO / Rx tag checker.
const WC: u16 = 12;

pub struct Dri;

impl SheetModule for Dri {
    fn id(&self) -> &'static str {
        "sup-dri"
    }
    fn title(&self) -> &'static str {
        DRI_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Micronutrient upper-limit ledger and guideline-dose checker",
            "SUP-10",
            "DIP ch164:343,387; WHO ch00:4346-4420; GREG ch35; HELMS ch02; UL values pending NIH ODS",
        )?;
        theme::unverified(
            ws,
            3,
            0,
            "UL values in Ref_UL: NIH ODS / IOM DRI tables (MED-VERIFY); ingredient amounts await tblSuppIngredients",
            f,
        )?;
        let nu = UL.len() as u32;
        table(ws, "tblSupDRI", HEAD, 0, DRI_HEADERS, nu)?;
        let (ing, unit, daily) = (ov_rng(d, 1), ov_rng(d, 3), ov_rng(d, 5));
        inputs(ws, cx, HEAD + 1, HEAD + nu, 5, None)?;
        for i in 0..nu {
            let (r, xr) = (HEAD + 1 + i, HEAD + 2 + i);
            let ul = |h: &str| format!("={}", data_cell(&REF_UL, h, i));
            fx(ws, r, 0, &ul("note"), None)?;
            fx(ws, r, 1, &ul("rule"), None)?;
            fx(ws, r, 2, &ul("value"), None)?;
            fx(ws, r, 3, &ul("unit"), None)?;
            fx(ws, r, 4, &format!("=SUMIFS({daily},{ing},\"*\"&B{xr}&\"*\",{unit},D{xr})"), Some(&f.dec1))?;
            fx(ws, r, 6, &format!("=E{xr}+N(F{xr})"), Some(&f.dec1))?;
            fx(ws, r, 7, &format!("=IF(N(C{xr})>0,G{xr}/C{xr},\"\")"), Some(&f.pct))?;
            fx(
                ws,
                r,
                8,
                &format!("=IF(H{xr}=\"\",\"\",IF(H{xr}>=1,\"over UL\",IF(H{xr}>=Sup_UL_Warn,\"near UL\",\"\")))"),
                None,
            )?;
            fx(ws, r, 9, &ul("source_state"), None)?;
        }
        theme::cf_formula(ws, HEAD + 1, 7, HEAD + nu, 8, &format!("=$I{}=\"over UL\"", HEAD + 2), theme::CRITICAL)?;
        theme::cf_formula(ws, HEAD + 1, 7, HEAD + nu, 8, &format!("=$I{}=\"near UL\"", HEAD + 2), theme::OUT_OF_REF)?;
        theme::cf_formula(
            ws,
            HEAD + 1,
            9,
            HEAD + nu,
            9,
            &format!("=$J{}=\"UNVERIFIED\"", HEAD + 2),
            theme::UNVERIFIED,
        )?;

        // WHO reference strengths (rule table on this sheet) and the per-item checker.
        let wr = HEAD + nu + 4;
        ws.write_string_with_format(wr - 1, 0, "WHO Model List reference strengths (tblSupWHO)", &f.section)?;
        let who = write_rules(ws, cx, DRI_TITLE, "tblSupWHO", wr, 0, WHO)?;
        let (kw, dose, wunit) = (&who[1], &who[2], &who[3]);
        ws.write_string_with_format(
            HEAD - 1,
            WC,
            "Rx / supplement / WHO-essential tags and dose vs WHO strength",
            &f.section,
        )?;
        let hdr = ["name", "kind_tag", "who_essential", "who_strength", "who_unit", "amount", "unit", "ratio_to_who"];
        table(ws, "tblSupWHOCheck", HEAD, WC, &hdr, super::n(d))?;
        let c = |k: u16| col(WC + k);
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            let name = format!("{}{xr}", c(0));
            fx(ws, r, WC, &format!("={}", book::at("name", xr)), None)?;
            let kind = book::at("kind", xr);
            fx(
                ws,
                r,
                WC + 1,
                &format!(
                    "=IF({kind}=\"Rx\",\"Rx\",IF({kind}=\"Nt\",\"nutrient\",IF({kind}=\"Sp\",\"supplement\",\"\")))"
                ),
                None,
            )?;
            let idx = match_idx(kw, &name);
            fx(ws, r, WC + 2, &format!("=IF({idx}>0,\"WHO essential (vitamin/mineral)\",\"\")"), None)?;
            fx(
                ws,
                r,
                WC + 3,
                &format!("=IF({idx}>0,IF(ISNUMBER(INDEX({dose},{idx})),INDEX({dose},{idx}),\"\"),\"\")"),
                None,
            )?;
            fx(ws, r, WC + 4, &format!("=IF({idx}>0,INDEX({wunit},{idx})&\"\",\"\")"), None)?;
            fx(ws, r, WC + 5, &mirror(&book::at("amount", xr)), None)?;
            fx(ws, r, WC + 6, &mirror(&book::at("unit", xr)), None)?;
            fx(
                ws,
                r,
                WC + 7,
                &format!(
                    "=IF(AND(ISNUMBER({a}{xr}),ISNUMBER({s}{xr}),{u}{xr}={w}{xr}),{a}{xr}/{s}{xr},\"\")",
                    a = c(5),
                    s = c(3),
                    u = c(6),
                    w = c(4)
                ),
                Some(&f.dec2),
            )?;
        }
        set_widths(
            ws,
            &[
                34.0, 12.0, 9.0, 6.0, 12.0, 10.0, 10.0, 9.0, 9.0, 12.0, 3.0, 3.0, 28.0, 10.0, 26.0, 10.0, 8.0, 8.0,
                6.0, 10.0,
            ],
        )?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}
