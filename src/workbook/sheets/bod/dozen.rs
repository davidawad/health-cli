//! BOD-22 `Bod_Dozen` (longevity.md D5, D6; BUILDABLE-NOW as an entry calculator):
//! Greger's Daily Dozen checklist.
//!
//! * `tblBodDozenRef` (rule table, `refs::RULE_HEADERS`): the 12 categories and daily
//!   servings (GREG ch20, via longevity.md D5; berries and other vegetables come from
//!   an image in the source and stay UNVERIFIED).
//! * `tblBodDozenMap`: one row per `tblNutrition` item with a keyword-suggested
//!   category (name + nutrition category), a typed override and typed servings per
//!   portion (blank = 1).
//! * `tblBodDozenEat`: a long log of nutrition items eaten (date, item, portions); the
//!   category and servings come through the mapper (INDEX/MATCH).
//! * `tblBodDozenLog`: the last 28 days of the log (anchored on the last `Bod_Cal`
//!   date, `Today` on an empty log), typed servings per category (whole numbers 0-10),
//!   and per category `total = typed + SUMIFS(eat log servings by date and category)`;
//!   `score = sum of MIN(total, target)` out of 24. Heatmap CF on the totals.
//! * `tblBodDozenWeek`: per category the last 7 and 28 days vs target (%), with a
//!   sparkline of the 28 daily totals. A line chart shows the daily score.

use rust_xlsxwriter::{DataValidation, DataValidationRule, Formula, Worksheet};

use super::{abs, at, cal_col, head, kv, list, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::NUTRITION;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Dozen";
const HR: u32 = TOP + 1;
const GREG: &str = "GREG ch20 (via longevity.md D5)";
const IMG: &str = "pending: GREG ch20 Figure 6 (image)";
/// (short key, category, daily servings, unit, verified, note).
const CATS: [(&str, &str, u32, &str, bool, &str); 12] = [
    ("beans", "Beans", 3, "servings/day", true, ""),
    ("berries", "Berries", 1, "servings/day", false, "count from Figure 6: confirm"),
    ("fruits", "Other fruits", 3, "servings/day", true, ""),
    ("crucif", "Cruciferous vegetables", 1, "servings/day", true, "30-80 g"),
    ("greens", "Greens", 2, "servings/day", true, "cruciferous or not"),
    ("veg", "Other vegetables", 2, "servings/day", false, "count from Figure 6: confirm"),
    ("flax", "Flaxseeds", 1, "tbsp/day", true, "ground (GREG ch27)"),
    ("nuts", "Nuts and seeds", 1, "servings/day", true, "30 g"),
    ("spices", "Herbs and spices", 1, "servings/day", true, "1/4 tsp turmeric"),
    ("grains", "Whole grains", 3, "servings/day", true, "GREG ch30"),
    ("drinks", "Beverages", 5, "glasses/day", true, "12 oz glasses"),
    ("exercise", "Exercise", 1, "session/day", true, "90 min moderate or 40 min vigorous"),
];
/// Keyword rules of the mapper's suggestion, first match wins (searched in " name category ").
const KEYWORDS: [(&str, &[&str]); 11] = [
    ("Berries", &["berr"]),
    ("Flaxseeds", &["flax"]),
    ("Beans", &["bean", "lentil", "chickpea", "tofu", "tempeh", "legume"]),
    ("Cruciferous vegetables", &["broccoli", "cabbage", "cauliflower", "brussels", "kale", "arugula", "cruciferous"]),
    ("Greens", &["spinach", "lettuce", "greens", "chard", "collard"]),
    ("Nuts and seeds", &[" nut", "almond", "walnut", "seed", "pecan", "cashew"]),
    ("Whole grains", &[" oat", "rice", "quinoa", "grain", "barley", "bread", "pasta"]),
    ("Herbs and spices", &["turmeric", "spice", "herb", "cinnamon", "ginger"]),
    ("Beverages", &[" tea ", "coffee", "water"]),
    ("Other fruits", &["fruit", "apple", "banana", "orange", "mango", "grape", "cherr", "pear"]),
    ("Other vegetables", &["vegetable", "carrot", "pepper", "tomato", "onion", "mushroom", "squash", "zucchini"]),
];
const DAYS: u32 = 28;
/// Mapper and eat-log columns.
const MC: u16 = 8;
const EC: u16 = 28;

/// The 12 category names (for drop-downs).
fn names() -> Vec<&'static str> {
    CATS.iter().map(|c| c.1).collect()
}

pub struct Dozen;

impl SheetModule for Dozen {
    fn id(&self) -> &'static str {
        "bod-dozen"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal", "data-daily", "data-nutrition"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Daily Dozen grid with nutrition-item mapper",
            "BOD-22",
            "L-D5, L-D6 | GREG ch20 (servings), ch27 (flax), ch30 (grains); berries / other vegetables UNVERIFIED",
            Banner::Unverified("GREG ch20 Figure 6 (image): servings for berries and other vegetables"),
        )?;
        // Rule table.
        section(ws, cx, TOP, 0, "Daily Dozen targets (tblBodDozenRef)")?;
        let rows: Vec<[String; 7]> = CATS
            .iter()
            .map(|(k, name, v, unit, ok, note)| {
                let (src, st) = if *ok { (GREG, "verified") } else { (IMG, "UNVERIFIED") };
                [
                    format!("dozen-{k}"),
                    (*name).into(),
                    v.to_string(),
                    (*unit).into(),
                    src.into(),
                    st.into(),
                    (*note).into(),
                ]
            })
            .collect();
        let refs: Vec<[&str; 7]> = rows.iter().map(|r| std::array::from_fn(|i| r[i].as_str())).collect();
        super::bmi::rule_table(ws, cx, "tblBodDozenRef", HR, 0, &refs)?;
        let target = |j: usize| abs(2, HR + 1 + j as u32);

        // Mapper, row-aligned with tblNutrition.
        let nm = NUTRITION.nrows(cx.d);
        let (iname, icat) = (NUTRITION.range(cx.d, "name"), NUTRITION.range(cx.d, "category"));
        section(ws, cx, TOP, MC, "Nutrition item -> category mapper (type a category or servings to override)")?;
        let suggest = |t: &str| {
            let mut f = String::from("\"\"");
            for (cat, keys) in KEYWORDS.iter().rev() {
                let any: Vec<String> = keys.iter().map(|k| format!("ISNUMBER(SEARCH(\"{k}\",{t}))")).collect();
                f = format!("IF(OR({}),\"{cat}\",{f})", any.join(","));
            }
            format!("={f}")
        };
        let mfirst = HR + 2;
        let idx = move |rng: &str, r: u32| format!("INDEX({rng},{})", r + 1 - mfirst);
        let mcols = vec![
            Col::f("item", |r| format!("=IF({i}=\"\",\"\",{i})", i = idx(&iname, r))),
            Col::f("nutrition_category", |r| format!("=IF({i}=\"\",\"\",{i})", i = idx(&icat, r))),
            Col::f("match_text", |r| format!("=\" \"&LOWER({}&\" \"&{})&\" \"", at(MC, r), at(MC + 1, r))),
            Col::f("suggested", move |r| if_item(&at(MC, r), &suggest(&at(MC + 2, r)))),
            Col::In("category"),
            Col::In("servings_per_portion"),
            Col::f("category_used", |r| {
                let (s, c) = (at(MC + 3, r), at(MC + 4, r));
                format!("=IF(TRIM({c})<>\"\",{c},{s})")
            }),
            Col::f("servings_used", |r| {
                let (u, s) = (at(MC + 6, r), at(MC + 5, r));
                format!("=IF({u}=\"\",\"\",IF(ISNUMBER({s}),{s},1))")
            }),
        ];
        table(ws, cx, "tblBodDozenMap", HR, MC, &mcols, nm)?;
        let (m0, m1) = (HR + 1, HR + nm);
        list(ws, m0, MC + 4, m1, &names())?;
        ws.add_data_validation(
            m0,
            MC + 5,
            m1,
            MC + 5,
            &DataValidation::new().allow_decimal_number(DataValidationRule::Between(0.0, 20.0)),
        )?;
        let (map_item, map_cat, map_srv) =
            (rng(TITLE, MC, m0, m1), rng(TITLE, MC + 6, m0, m1), rng(TITLE, MC + 7, m0, m1));

        // Eat log (long): items eaten -> category servings through the mapper.
        section(ws, cx, TOP, EC, "Items eaten (typed): category and servings through the mapper")?;
        let hit = |item: &str| format!("MATCH({item},{map_item},0)");
        let ecols = vec![
            Col::Date("eaten_on"),
            Col::In("item"),
            Col::In("portions"),
            Col::f("category", |r| {
                let i = at(EC + 1, r);
                format!("=IF(TRIM({i})=\"\",\"\",IFERROR(INDEX({map_cat},{})&\"\",\"\"))", hit(&i))
            }),
            Col::f("servings", |r| {
                let (i, p, c) = (at(EC + 1, r), at(EC + 2, r), at(EC + 3, r));
                format!("=IF(AND({c}<>\"\",ISNUMBER({p})),IFERROR({p}*INDEX({map_srv},{}),\"\"),\"\")", hit(&i))
            }),
        ];
        table(ws, cx, "tblBodDozenEat", HR, EC, &ecols, LOG_ROWS)?;
        let (e0, e1) = (HR + 1, HR + LOG_ROWS);
        ws.add_data_validation(
            e0,
            EC + 1,
            e1,
            EC + 1,
            &DataValidation::new().allow_list_formula(Formula::new(format!("={}", NUTRITION.range(cx.d, "name")))),
        )?;
        ws.add_data_validation(
            e0,
            EC + 2,
            e1,
            EC + 2,
            &DataValidation::new().allow_decimal_number(DataValidationRule::Between(0.0, 20.0)),
        )?;
        let (e_date, e_cat, e_srv) = (rng(TITLE, EC, e0, e1), rng(TITLE, EC + 3, e0, e1), rng(TITLE, EC + 4, e0, e1));

        // Grid: last 28 days x (typed, total) per category + score.
        let gr = HR + 12.max(nm) + 3;
        section(ws, cx, gr - 1, 0, "Last 28 days: typed servings (blue, 0-10) + items eaten = total; score out of 24")?;
        let cd = cal_col(cx, "date");
        let gfirst = gr + 2;
        let typed: Vec<String> = CATS.iter().map(|c| format!("in_{}", c.0)).collect();
        let mut g = vec![Col::f("date", move |r| {
            let back = DAYS - 1 - (r - gfirst);
            format!("=IF(COUNT({cd})=0,Today,MAX({cd}))-{back}")
        })];
        for h in &typed {
            g.push(Col::In(h));
        }
        for (j, c) in CATS.iter().enumerate() {
            let (ed, ec, es) = (&e_date, &e_cat, &e_srv);
            g.push(Col::f(c.0, move |r| {
                format!("=N({})+SUMIFS({es},{ed},{},{ec},\"{}\")", at(1 + j as u16, r), at(0, r), c.1)
            }));
        }
        g.push(Col::f("score", move |r| {
            let parts: Vec<String> = (0..12).map(|j| format!("MIN({},{})", at(13 + j as u16, r), target(j))).collect();
            format!("={}", parts.join("+"))
        }));
        table(ws, cx, "tblBodDozenLog", gr, 0, &g, DAYS)?;
        let (g0, g1) = (gr + 1, gr + DAYS);
        super::date_rows(ws, cx, 0, g0, g1)?;
        ws.add_data_validation(
            g0,
            1,
            g1,
            12,
            &DataValidation::new().allow_whole_number(DataValidationRule::Between(0, 10)),
        )?;
        for j in 0..12u16 {
            let c = 13 + j;
            let x = at(c, g0 + 1);
            let t = target(j as usize);
            theme::cf_formula(ws, g0, c, g1, c, &format!("=AND(ISNUMBER({x}),{x}>={t})"), theme::OPTIMAL)?;
            theme::cf_formula(ws, g0, c, g1, c, &format!("=AND(ISNUMBER({x}),{x}>0,{x}<{t})"), theme::REF_OK)?;
        }
        theme::cf_two_color(ws, g0, 25, g1, 25, theme::OUT_OF_REF, theme::BAND_OPT)?;

        // Per-category week / month compliance with a sparkline.
        let wr = g1 + 3;
        section(ws, cx, wr - 1, 0, "Compliance by category (% of target servings)")?;
        let gdate = rng(TITLE, 0, g0, g1);
        let wfirst = wr + 2;
        let wcols = vec![
            Col::f("category", |r| format!("=\"{}\"", CATS[(r - wfirst) as usize].1)),
            Col::f("target", move |r| format!("={}", target((r - wfirst) as usize))),
            Col::f("last7", |r| {
                let tot = rng(TITLE, 13 + (r - wfirst) as u16, g0, g1);
                format!("=SUMIFS({tot},{gdate},\">\"&(MAX({gdate})-7))")
            }),
            Col::f("pct7", |r| format!("=IFERROR({}/(7*{}),\"\")", at(2, r), at(1, r))),
            Col::f("last28", |r| format!("=SUM({})", rng(TITLE, 13 + (r - wfirst) as u16, g0, g1))),
            Col::f("pct28", |r| format!("=IFERROR({}/(28*{}),\"\")", at(4, r), at(1, r))),
            Col::f("trend", |_| "=\"\"".into()),
        ];
        table(ws, cx, "tblBodDozenWeek", wr, 0, &wcols, 12)?;
        super::fmt_rows(ws, wr + 1, wr + 12, 3, 3, "0%")?;
        super::fmt_rows(ws, wr + 1, wr + 12, 5, 5, "0%")?;
        theme::cf_three_color(ws, wr + 1, 3, wr + 12, 3, false)?;
        theme::cf_three_color(ws, wr + 1, 5, wr + 12, 5, false)?;
        for j in 0..12u32 {
            theme::sparkline(ws, wr + 1 + j, 6, &rng(TITLE, 13 + j as u16, g0, g1))?;
        }

        // Summary.
        let score = rng(TITLE, 25, g0, g1);
        let sc = MC;
        section(ws, cx, wr - 1, sc, "Summary")?;
        kv(ws, cx, wr, sc, "Score, latest day (of 24)", &format!("=INDEX({score},{DAYS})"), Some("0"))?;
        kv(ws, cx, wr + 1, sc, "Mean score, 28 days", &format!("=AVERAGE({score})"), Some("0.0"))?;
        kv(ws, cx, wr + 2, sc, "Days at 24/24", &format!("=COUNTIFS({score},\">=24\")"), Some("0"))?;
        kv(ws, cx, wr + 3, sc, "Items mapped", &format!("=COUNTIFS({map_cat},\"?*\")"), Some("0"))?;
        kv(ws, cx, wr + 4, sc, "Items eaten logged", &format!("=COUNTIFS({e_cat},\"?*\")"), Some("0"))?;

        let chart = super::line_chart(TITLE, "Daily Dozen score (of 24)", g0, g1, 0, &[("score", 25)]);
        ws.insert_chart(wr + 14, 0, &chart)?;
        let mut w = vec![12.0, 22.0, 6.0, 12.0, 30.0, 11.0, 26.0, 2.0, 24.0, 14.0, 4.0, 20.0, 20.0, 9.0, 20.0, 9.0];
        w.resize(28, 8.0);
        w.extend([12.0, 22.0, 9.0, 20.0, 9.0]);
        set_widths(ws, &w)?;
        Ok(())
    }
}

/// `formula` (with its leading `=`) when the item cell is not blank, else "".
fn if_item(item: &str, formula: &str) -> String {
    format!("=IF({item}=\"\",\"\",{})", &formula[1..])
}
