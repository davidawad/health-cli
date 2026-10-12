//! AGE-12 `Age_VO2`: VO2 max test log with the annual test card and the
//! resting-HR proxy, a lifetime projection (decline per decade, steeper after
//! 50) against the catalogued thresholds, and a percentile / hazard lookup over a
//! norm table that ships empty (UNVERIFIED: FRIEND / Cooper norms not checked).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, as_fraction, at, head, kpi, last_num, rng, section, table, Banner, Col, AGE_NOW, ENTRY_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::WEARABLE;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_VO2";
const LOG_HR: u32 = TOP + 1;
const CARD_C: u16 = 9;
/// Projection rows (ages from today, one per year).
const YEARS: u32 = 71;
const BANDS: [u32; 7] = [20, 30, 40, 50, 60, 70, 80];

/// Female when `Sex` starts with f/F.
const FEMALE: &str = "LEFT(Sex&\" \",1)=\"f\"";

pub struct Vo2;

impl SheetModule for Vo2 {
    fn id(&self) -> &'static str {
        "age-vo2"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "VO2 max: tests, lifetime projection, percentile",
            "AGE-12 (L-C2, L-C3, L-C4, L-C9, C11.6) | OUT ch34 l.5-30, ch30 l.61-65, ch31 l.55; Uth 2004 (pending)",
            Banner::Awaiting("a VO2 max test, the wearable resting-HR feed (tblWearable) and the norm table below"),
        )?;
        theme::unverified(ws, 4, 0, "Uth 2004 (15.3 x HRmax/HRrest), FRIEND/Cooper norms, Decline_VO2_pct_decade", f)?;

        // Test log.
        section(ws, cx, TOP, "Tests (typed): lab, watch estimate or proxy")?;
        let cols = [
            Col::Date("date"),
            Col::In("vo2_measured"),
            Col::In("method"),
            Col::In("hr_max"),
            Col::In("hr_rest"),
            Col::f("proxy_uth", |r| {
                format!("=IF(AND(ISNUMBER(D{r}),ISNUMBER(E{r}),N(E{r})>0),Age_VO2_RHR_k*D{r}/E{r},\"\")")
            }),
            Col::f("best", |r| format!("=IF(ISNUMBER(B{r}),B{r},F{r})")),
        ];
        table(ws, cx, "tblAgeVO2", LOG_HR, 0, &cols, ENTRY_ROWS)?;
        let (l0, l1) = (LOG_HR + 1, LOG_HR + ENTRY_ROWS);
        for r in l0..=l1 {
            ws.set_cell_format(r, 5, &f.dec1)?;
        }
        let (dates, vo2, hrmax) = (rng(TITLE, 0, l0, l1), rng(TITLE, 1, l0, l1), rng(TITLE, 3, l0, l1));

        // Annual test card.
        let c = CARD_C;
        ws.write_string_with_format(TOP, c, "Annual test card", &f.section)?;
        let nw = WEARABLE.nrows(d);
        let (wd, wr) = (WEARABLE.range_n(nw, "date"), WEARABLE.range_n(nw, "rhr"));
        let card: [(&str, String, Option<&str>); 9] = [
            ("Latest measured VO2", last_num(&vo2), Some("0.0")),
            ("Last test date", format!("IF(COUNT({vo2})=0,\"\",MAXIFS({dates},{vo2},\">0\"))"), Some("yyyy-mm-dd")),
            ("Days since test", format!("IF(ISNUMBER({0}),Today-{0},\"\")", abs(c + 1, TOP + 2)), Some("0")),
            (
                "Status",
                format!("IF({0}=\"\",\"no test\",IF({0}>Age_VO2_test_days,\"DUE\",\"ok\"))", abs(c + 1, TOP + 3)),
                None,
            ),
            ("Resting HR, 30-day mean", format!("IFERROR(AVERAGEIFS({wr},{wd},\">=\"&(Today-30)),\"\")"), Some("0")),
            (
                "HRmax used",
                format!(
                    "IF(COUNT({hrmax})>0,{},IF(ISNUMBER(DOB),Age_HRmax_a-Age_HRmax_b*{AGE_NOW},\"\"))",
                    last_num(&hrmax)
                ),
                Some("0"),
            ),
            (
                "Proxy VO2 (estimate)",
                format!(
                    "IF(AND(ISNUMBER({0}),ISNUMBER({1})),Age_VO2_RHR_k*{1}/{0},\"\")",
                    abs(c + 1, TOP + 5),
                    abs(c + 1, TOP + 6)
                ),
                Some("0.0"),
            ),
            (
                "Start value for the projection",
                format!("IF(ISNUMBER({0}),{0},{1})", abs(c + 1, TOP + 1), abs(c + 1, TOP + 7)),
                Some("0.0"),
            ),
            ("Age now", AGE_NOW.to_string(), Some("0.0")),
        ];
        for (i, (label, formula, nf)) in card.iter().enumerate() {
            super::kv(ws, cx, TOP + 1 + i as u32, c, label, &format!("={formula}"), *nf)?;
        }
        theme::cf_formula(
            ws,
            TOP + 4,
            c + 1,
            TOP + 4,
            c + 1,
            &format!("={}=\"DUE\"", at(c + 1, TOP + 5)),
            theme::OUT_OF_REF,
        )?;
        kpi(cx, "vo2_latest", "Latest measured VO2 max (ml/kg/min)", TITLE, TOP + 1, c + 1, Some("0.0"))?;
        let (start, age_now) = (abs(c + 1, TOP + 8), abs(c + 1, TOP + 9));

        // Projection.
        let p_hr = l1 + 3;
        section(
            ws,
            cx,
            p_hr - 1,
            "Lifetime projection: decline per decade (Decline_VO2_pct_decade; Age_VO2_decline_post50_pct after 50)",
        )?;
        let rate = |age: &str| {
            format!(
                "IF({age}>=50,{},{})",
                as_fraction("Age_VO2_decline_post50_pct"),
                as_fraction("Decline_VO2_pct_decade")
            )
        };
        let first = p_hr + 2;
        let cols = [
            Col::f("age", |r| {
                if r == first {
                    format!("=IF(ISNUMBER({age_now}),ROUNDDOWN({age_now},0),\"\")")
                } else {
                    format!("=IF(OR(A{p}=\"\",A{p}>=100),\"\",A{p}+1)", p = r - 1)
                }
            }),
            Col::f("projected", |r| {
                if r == first {
                    format!("=IF(AND(ISNUMBER(A{r}),ISNUMBER({start})),{start},\"\")")
                } else {
                    let p = r - 1;
                    format!(
                        "=IF(OR(A{r}=\"\",B{p}=\"\",{rt}=\"\"),\"\",B{p}*(1-{rt})^(1/10))",
                        rt = rate(&format!("A{p}"))
                    )
                }
            }),
            Col::f("with_training", |r| {
                if r == first {
                    format!("=IF(B{r}=\"\",\"\",B{r}+Age_VO2_train_gain)")
                } else {
                    let p = r - 1;
                    format!("=IF(OR(B{r}=\"\",C{p}=\"\"),\"\",C{p}*B{r}/B{p})")
                }
            }),
            Col::f("floor", |r| format!("=IF(A{r}=\"\",\"\",IF({FEMALE},Age_VO2_floor_f,Age_VO2_floor_m))")),
            Col::f("hike", |r| format!("=IF(A{r}=\"\",\"\",Age_VO2_hike)")),
            Col::f("stairs", |r| format!("=IF(A{r}=\"\",\"\",Age_VO2_stairs)")),
            Col::f("marchand", |r| format!("=IF(A{r}=\"\",\"\",Age_VO2_marchand)")),
        ];
        table(ws, cx, "tblAgeVO2Projection", p_hr, 0, &cols, YEARS)?;
        let (p0, p1) = (p_hr + 1, p_hr + YEARS);
        for r in p0..=p1 {
            ws.set_cell_format(r, 1, &f.dec1)?;
            ws.set_cell_format(r, 2, &f.dec1)?;
        }
        let mut chart = Chart::new(ChartType::ScatterStraight);
        for (name, col) in [
            ("Projected", 1u16),
            ("With +training", 2),
            ("Independence floor", 3),
            ("Hiking (30)", 4),
            ("90 steps / 2 min (32)", 5),
            ("Marchand at 100+ (35)", 6),
        ] {
            chart
                .add_series()
                .set_name(name)
                .set_categories((TITLE, p0, 0, p1, 0))
                .set_values((TITLE, p0, col, p1, col));
        }
        chart.title().set_name("VO2 max projection to 100");
        chart.x_axis().set_name("Age");
        chart.y_axis().set_name("ml/kg/min");
        chart.set_width(640).set_height(320);
        ws.insert_chart(p_hr, CARD_C, &chart)?;

        // Norm table and lookup.
        let n_hr = p1 + 3;
        section(
            ws,
            cx,
            n_hr - 1,
            "Norm table (UNVERIFIED, empty: fill from FRIEND / Cooper; hazard ratios from OUT ch30 l.61-65)",
        )?;
        let cols = [
            Col::In("age_band"),
            Col::In("sex"),
            Col::f("key", |r| format!("=A{r}&\"|\"&B{r}")),
            Col::In("p25"),
            Col::In("p50"),
            Col::In("p75"),
            Col::In("p97_7_elite"),
            Col::In("hazard_vs_elite"),
            Col::In("source"),
        ];
        let rows = (BANDS.len() * 2) as u32;
        table(ws, cx, "tblAgeVO2Norms", n_hr, 0, &cols, rows)?;
        for (i, (band, sex)) in BANDS.iter().flat_map(|b| [(b, "m"), (b, "f")]).enumerate() {
            let r = n_hr + 1 + i as u32;
            ws.write_number(r, 0, f64::from(*band))?;
            ws.write_string(r, 1, sex)?;
            ws.write_string_with_format(r, 8, "pending: FRIEND registry / Cooper Institute norms", &f.unverified)?;
        }
        let (n0, n1) = (n_hr + 1, n_hr + rows);
        let key = rng(TITLE, 2, n0, n1);
        let colr = |c: u16| rng(TITLE, c, n0, n1);
        let lr = n1 + 2;
        let sex_key = format!("IF({FEMALE},\"f\",\"m\")");
        let band = |shift: i32| format!("MIN(80,MAX(20,FLOOR({age_now}{shift:+},10)))&\"|\"&{sex_key}");
        let pick = |c: u16, shift: i32| format!("INDEX({},MATCH({},{key},0))", colr(c), band(shift));
        ws.write_string_with_format(lr - 1, 0, "Lookup (latest measured VO2)", &f.section)?;
        let lookups: [(&str, String); 3] = [
            (
                "Bracket",
                format!(
                    "=IFERROR(IF(OR({v}=\"\",{p97}=\"\"),\"n/a\",IF({v}>={p97},\"elite\",IF({v}>={p75},\"high\",IF({v}>={p50},\"above median\",IF({v}>={p25},\"below median\",\"low\"))))),\"n/a\")",
                    v = abs(c + 1, TOP + 1),
                    p97 = pick(6, 0),
                    p75 = pick(5, 0),
                    p50 = pick(4, 0),
                    p25 = pick(3, 0)
                ),
            ),
            ("Hazard vs elite (from the table)", format!("=IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", pick(7, 0))),
            ("Elite for your sex, two decades younger", format!("=IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", pick(6, -20))),
        ];
        for (i, (label, formula)) in lookups.iter().enumerate() {
            super::kv(ws, cx, lr + i as u32, 0, label, formula, None)?;
        }
        ws.write_string_with_format(lr + 3, 0, "Hazard ratios are not additive with other risk views.", &f.note)?;
        set_widths(ws, &[16.0, 12.0, 12.0, 9.0, 9.0, 10.0, 12.0, 14.0, 34.0, 30.0, 12.0])?;
        Ok(())
    }
}
