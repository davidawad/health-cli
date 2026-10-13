//! The rule tables this workstream owns, in the shared schema
//! `id, rule, value, unit, source, source_state, note` (`refs::RULE_HEADERS`):
//!
//! * `Ref_Vax` (SCH-01): `rule` = keyword matched in the vaccine name, `unit` = the
//!   parameter (`series_doses`, `series_gap_months`, `booster_months`, `live`),
//!   `value` its number. Every interval is UNVERIFIED pending the current CDC adult
//!   schedule (the library's tables are stripped; DIP ch148.md:23,165 says to use it).
//! * `Ref_Screen` (SCH-02): `rule` = screening item key, `unit` = `sex` (0 all, 1 male,
//!   2 female), `start_age`, `stop_age`, `interval_months` (0 = once), `framing`
//!   (1 guideline, 2 Attia practice - not guideline), `risk_only` (1 = applies only
//!   when ticked). Attia's practice ships with its file + line; mainstream ages are
//!   UNVERIFIED pending USPSTF.

use rust_xlsxwriter::{TableStyle, Worksheet};

use crate::error::Result;
use crate::infra::data::{write_table, TableDef, V};
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{Cx, Data, SheetModule};

const CDC: &str = "pending: CDC adult immunization schedule (current year)";
const U: &str = "UNVERIFIED";

/// A vaccine rule: keywords, series doses, gap between series doses (months),
/// booster interval (months, 0 = none), live (1/0), note.
type Vaccine = (&'static [&'static str], f64, f64, f64, f64, &'static str);

const VACCINES: &[Vaccine] = &[
    (&["tetanus", "tdap", "td/"], 1.0, 0.0, 120.0, 0.0, "toxoid: booster every 10 years after the primary series"),
    (&["influenza", "flu"], 1.0, 0.0, 12.0, 0.0, "annual; schedules updated yearly (DIP ch148.md:23)"),
    (&["covid"], 1.0, 0.0, 12.0, 0.0, "updated formulation; interval changes by season"),
    (&["zoster", "shingles"], 2.0, 2.0, 0.0, 0.0, "recombinant: 2 doses 2-6 months apart"),
    (&["pneumo"], 1.0, 0.0, 0.0, 0.0, "conjugate; age/risk based"),
    (&["hepatitis b", "hep b"], 3.0, 1.0, 0.0, 0.0, "0, 1, 6 months (gap shown is the shortest)"),
    (&["hepatitis a", "hep a"], 2.0, 6.0, 0.0, 0.0, "0 and 6-12 months"),
    (&["hpv"], 3.0, 2.0, 0.0, 0.0, "0, 1-2, 6 months"),
    (&["mmr", "measles"], 2.0, 1.0, 0.0, 1.0, "live attenuated (DIP ch148.md:113-115: deferred in pregnancy)"),
    (&["varicella", "chickenpox"], 2.0, 1.0, 0.0, 1.0, "live attenuated"),
    (&["yellow fever"], 1.0, 0.0, 0.0, 1.0, "live attenuated; travel"),
    (&["typhoid"], 1.0, 0.0, 24.0, 0.0, "injectable; the oral form is live"),
    (&["meningococcal", "menacwy"], 1.0, 0.0, 60.0, 0.0, "risk based"),
];

fn vax_rows(_: &Data) -> Vec<Vec<V>> {
    let mut out = Vec::new();
    for (keys, series, gap, booster, live, note) in VACCINES {
        for k in *keys {
            for (unit, v) in
                [("series_doses", *series), ("series_gap_months", *gap), ("booster_months", *booster), ("live", *live)]
            {
                out.push(vec![
                    V::S(format!("VAX-{:03}", out.len() + 1)),
                    V::S((*k).into()),
                    V::N(v),
                    V::S(unit.into()),
                    V::S(CDC.into()),
                    V::S(U.into()),
                    V::S((*note).into()),
                ]);
            }
        }
    }
    out
}

/// A screening item (SCH-02).
pub struct Item {
    pub key: &'static str,
    pub label: &'static str,
    /// 0 all, 1 male, 2 female.
    pub sex: f64,
    pub start: f64,
    /// 120 = no upper age.
    pub stop: f64,
    /// 0 = once.
    pub interval: f64,
    /// 1 guideline, 2 Attia practice.
    pub framing: f64,
    /// Applies only when ticked (e.g. smoking history).
    pub risk_only: bool,
    /// `tblLabs.marker_id` whose last draw counts as "done" ("" = none).
    pub marker: &'static str,
    /// Keyword matched in `tblPhys.name` ("" = none).
    pub phys: &'static str,
    pub source: &'static str,
    pub state: &'static str,
    /// The start age is a heuristic default even when the interval is cited.
    pub heuristic_start: bool,
    pub note: &'static str,
}

const fn item(key: &'static str, label: &'static str, sex: f64, start: f64, stop: f64, interval: f64) -> Item {
    Item {
        key,
        label,
        sex,
        start,
        stop,
        interval,
        framing: 1.0,
        risk_only: false,
        marker: "",
        phys: "",
        source: "",
        state: U,
        heuristic_start: false,
        note: "",
    }
}

/// Every screening item, in display order.
pub fn items() -> Vec<Item> {
    vec![
        Item {
            phys: "colon",
            source: "pending: USPSTF colorectal cancer screening (2021)",
            ..item("colon-guideline", "Colorectal: colonoscopy (guideline)", 0.0, 45.0, 75.0, 120.0)
        },
        Item {
            framing: 2.0,
            phys: "colon",
            source: "OUT ch22.md:31",
            state: "verified",
            note: "colonoscopy by 40, repeat every 2-3 years depending on findings",
            ..item("colon-attia", "Colorectal: colonoscopy (Attia practice)", 0.0, 40.0, 120.0, 24.0)
        },
        Item {
            marker: "ldl-c",
            phys: "lipid",
            source: "pending: USPSTF / ACC-AHA lipid screening",
            ..item("lipids", "Lipid panel", 0.0, 20.0, 120.0, 60.0)
        },
        Item {
            marker: "hba1c",
            phys: "a1c",
            source: "pending: USPSTF prediabetes and type 2 diabetes screening (2021)",
            note: "DIP ch97.md:211: screen at BMI >= 25 (>= 23 Asian-American) with another risk factor",
            ..item("diabetes", "Diabetes screen (A1c / glucose)", 0.0, 35.0, 70.0, 36.0)
        },
        Item {
            framing: 2.0,
            marker: "lpa",
            phys: "lp(a)",
            source: "OUT ch14.md:163",
            state: "verified",
            heuristic_start: true,
            note: "Lp(a) once per lifetime",
            ..item("lpa", "Lp(a) once per lifetime", 0.0, 18.0, 120.0, 0.0)
        },
        Item {
            marker: "psa",
            phys: "psa",
            source: "pending: USPSTF prostate cancer screening (2018)",
            ..item("psa", "Prostate: PSA", 1.0, 55.0, 69.0, 24.0)
        },
        Item {
            phys: "mammo",
            source: "pending: USPSTF breast cancer screening (2024)",
            ..item("mammo", "Breast: mammography", 2.0, 40.0, 74.0, 24.0)
        },
        Item {
            framing: 2.0,
            phys: "pap",
            source: "OUT ch22.md:37",
            state: "verified",
            heuristic_start: true,
            note: "Pap yearly in Attia's practice; ages are heuristic",
            ..item("cervical", "Cervical: Pap (Attia practice)", 2.0, 21.0, 65.0, 12.0)
        },
        Item {
            risk_only: true,
            phys: "lung",
            source: "pending: USPSTF lung cancer screening (2021)",
            note: "smokers only (OUT ch22.md:15)",
            ..item("lung", "Lung: low-dose CT (smoking history)", 0.0, 50.0, 80.0, 12.0)
        },
        Item {
            framing: 2.0,
            phys: "skin",
            source: "heuristic",
            state: "heuristic",
            note: "OUT ch22 (longevity.md B19): intervals user-supplied",
            ..item("skin", "Skin check", 0.0, 18.0, 120.0, 12.0)
        },
        Item {
            framing: 2.0,
            phys: "dexa",
            source: "OUT ch35.md:19-25",
            state: "verified",
            heuristic_start: true,
            note: "Attia measures BMD yearly with DEXA",
            ..item("dexa", "DEXA (bone density, lean mass)", 0.0, 40.0, 120.0, 12.0)
        },
        Item {
            framing: 2.0,
            phys: "hear",
            source: "OUT ch28.md:29",
            state: "verified",
            heuristic_start: true,
            note: "audiogram annually",
            ..item("hearing", "Hearing (audiogram)", 0.0, 50.0, 120.0, 12.0)
        },
        Item {
            framing: 2.0,
            phys: "cogn",
            source: "heuristic",
            state: "heuristic",
            note: "annual slot (longevity.md B21)",
            ..item("cognition", "Cognition check", 0.0, 50.0, 120.0, 12.0)
        },
    ]
}

fn screen_rows(_: &Data) -> Vec<Vec<V>> {
    let mut out = Vec::new();
    for it in items() {
        let heur = ("heuristic", "heuristic");
        let own = (it.source, it.state);
        let fields = [
            ("sex", it.sex, own),
            ("start_age", it.start, if it.heuristic_start { heur } else { own }),
            ("stop_age", it.stop, if it.stop >= 120.0 { heur } else { own }),
            ("interval_months", it.interval, own),
            ("framing", it.framing, ("MASTER.md SCH-02 (label)", "verified")),
            ("risk_only", if it.risk_only { 1.0 } else { 0.0 }, own),
        ];
        for (unit, v, (src, state)) in fields {
            out.push(vec![
                V::S(format!("SCR-{:03}", out.len() + 1)),
                V::S(it.key.into()),
                V::N(v),
                V::S(unit.into()),
                V::S(src.into()),
                V::S(state.into()),
                if it.note.is_empty() { V::E } else { V::S(it.note.into()) },
            ]);
        }
    }
    out
}

macro_rules! def {
    ($name:ident, $id:literal, $sheet:literal, $table:literal, $rows:expr) => {
        pub static $name: TableDef =
            TableDef { id: $id, sheet: $sheet, table: $table, headers: RULE_HEADERS, rows: $rows, needs_labs: false };
    };
}
def!(REF_VAX, "ref-vax", "Ref_Vax", "tblRefVax", vax_rows);
def!(REF_SCREEN, "ref-screen", "Ref_Screen", "tblRefScreen", screen_rows);

/// `SUMIFS(value, rule, <key>, unit, "<unit>")` over a rule table (no `=`).
pub fn lookup(def: &TableDef, cx: &Cx, key: &str, unit: &str) -> String {
    format!(
        "SUMIFS({},{},{key},{},\"{unit}\")",
        def.range(cx.d, "value"),
        def.range(cx.d, "rule"),
        def.range(cx.d, "unit")
    )
}

/// A `Ref_*` sheet holding one rule table, UNVERIFIED rows in orange.
pub struct RuleSheet(pub &'static TableDef);

impl SheetModule for RuleSheet {
    fn id(&self) -> &'static str {
        self.0.id
    }
    fn title(&self) -> &'static str {
        self.0.sheet
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let rows = (self.0.rows)(cx.d);
        write_table(ws, self.0, &rows, cx, TableStyle::Light1)?;
        theme::cf_formula(ws, 1, 5, rows.len() as u32, 5, "=$F2=\"UNVERIFIED\"", theme::UNVERIFIED)?;
        ws.set_column_width(1, 18.0)?;
        ws.set_column_width(3, 18.0)?;
        ws.set_column_width(4, 50.0)?;
        ws.set_column_width(6, 60.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_rows_follow_the_shared_schema_and_cite_or_pend() {
        let d = Data::default();
        for def in [&REF_VAX, &REF_SCREEN] {
            let rows = (def.rows)(&d);
            assert!(!rows.is_empty());
            for r in &rows {
                assert_eq!(r.len(), RULE_HEADERS.len());
                let (V::S(src), V::S(state)) = (&r[4], &r[5]) else { panic!("source and state are text") };
                if state == U {
                    assert!(src.starts_with("pending: "), "{src}");
                } else {
                    assert!(["verified", "heuristic"].contains(&state.as_str()), "{state}");
                }
            }
            let mut ids: Vec<String> = rows.iter().map(|r| format!("{:?}", r[0])).collect();
            ids.dedup();
            assert_eq!(ids.len(), rows.len(), "{}: ids unique", def.table);
        }
        // Every vaccine keyword carries its four parameters; MMR is live.
        let vax = vax_rows(&d);
        assert_eq!(vax.len() % 4, 0);
        assert!(vax.iter().any(|r| r[1] == V::S("mmr".into()) && r[3] == V::S("live".into()) && r[2] == V::N(1.0)));
        // Attia's colonoscopy practice is cited, the guideline ages pend USPSTF.
        let it = items();
        assert!(it.iter().any(|i| i.key == "colon-attia" && i.source == "OUT ch22.md:31"));
        assert!(it.iter().any(|i| i.key == "lpa" && i.interval == 0.0));
        assert_eq!(screen_rows(&d).len(), it.len() * 6);
    }
}
