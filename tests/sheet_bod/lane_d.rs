//! Lane D tests: BOD-16, BOD-24..30 (wearable, glycemic, eating window, alcohol, sleep,
//! architecture, caffeine, sleep hygiene). All placeholders over empty logs: structure,
//! key formulas and tables; after recalculation no error cell and the supplement-stack
//! caffeine at bedtime.

use calamine::Reader;

use crate::fixture::*;

const SHEETS: [(&str, &str); 8] = [
    ("Bod_Wearable", "BOD-16"),
    ("Bod_Glycemic", "BOD-24"),
    ("Bod_Window", "BOD-25"),
    ("Bod_Alcohol", "BOD-26"),
    ("Bod_Sleep", "BOD-27"),
    ("Bod_Arch", "BOD-28"),
    ("Bod_Caffeine", "BOD-29"),
    ("Bod_SleepHyg", "BOD-30"),
];

#[test]
fn lane_d_placeholders_have_banners_formulas_and_tables() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    for (s, id) in SHEETS {
        let t = sheet_text(&mut wb, s);
        assert!(t.contains(&format!("{id} PLACEHOLDER")), "{s}: status line");
        assert!(t.contains("PLACEHOLDER - awaiting data"), "{s}: awaiting banner");
    }
    let want: [(&str, &[&str]); 8] = [
        ("Bod_Caffeine", &["Caffeine_HL", "G_Caffeine_HL", "0.5^", "SUMPRODUCT(", "Ref_TimeSlots!", "Data_Supp!"]),
        ("Bod_Sleep", &["Bod_Sleep_min_h", "STDEV.S(", "MOD(", "Bod_Wake_SD_nights", "Data_Wearable!"]),
        (
            "Bod_Wearable",
            &["Bod_ACWR_acute", "Bod_ACWR_chronic", "Data_Wearable!", "Bod_Z_band", "Age_Zone2!", "SUMPRODUCT("],
        ),
        ("Bod_Glycemic", &["Bod_CGM_lo", "Bod_Spike_rise", "Bod_Fibre_g", "Bod_Cal!", "STDEV.S("]),
        ("Bod_Window", &["MINIFS(", "MAXIFS(", "Bod_Window_ref_h"]),
        ("Bod_Alcohol", &["Data_Wearable!", "Bod_Sleep!", "SUMPRODUCT("]),
        ("Bod_Arch", &["Bod_Cycle_min", "Bod_Arch_base_nights", "Data_Wearable!"]),
        (
            "Bod_SleepHyg",
            &["Bod_STOPBANG_flag", "Bod_Last_meal_gap_h", "Bod_Window!", "Bod_Caffeine!", "Bod_Glycemic!"],
        ),
    ];
    for (s, needles) in want {
        let f = formulas(&mut wb, s);
        for n in needles {
            assert!(f.contains(n), "{s}: no formula with {n}");
        }
    }
    let names = wb.sheet_names();
    for (s, _) in SHEETS {
        assert!(formulas(&mut wb, s).lines().count() >= 20, "{s}: formulas");
        let xml = sheet_xml(&p, &names, s);
        for part in ["<tableParts", "<conditionalFormatting", "<drawing "] {
            assert!(xml.contains(part), "{s}: {part}");
        }
        // Typed vocabularies and times are validated (Bod_Wearable has no typed input).
        assert!(s == "Bod_Wearable" || xml.contains("<dataValidations"), "{s}: data validation");
    }
    assert!(sheet_text(&mut wb, "Bod_Wearable").contains("UNVERIFIED - pending: Gabbett 2016"));
    assert!(sheet_text(&mut wb, "Bod_SleepHyg").contains("UNVERIFIED - pending: Chung 2008"));
    assert!(sheet_text(&mut wb, "Bod_Arch").contains("OUT ch58 l.25"));
    assert!(sheet_text(&mut wb, "Bod_Window").contains("not a target"));
    assert!(sheet_text(&mut wb, "Bod_Alcohol").contains("OUT ch60 rule 1"));

    let z = zip::ZipArchive::new(std::fs::File::open(&p).unwrap()).unwrap();
    let names: Vec<String> = z.file_names().filter(|n| n.starts_with("xl/tables/table")).map(String::from).collect();
    let tables: Vec<String> = names
        .iter()
        .map(|n| part(&p, n).split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    for t in [
        "tblBodWearable",
        "tblBodWearableWeeks",
        "tblBodHacks",
        "tblBodCGM",
        "tblBodFibre",
        "tblBodGlycemicDays",
        "tblBodMeals",
        "tblBodWindowDays",
        "tblBodAlcohol",
        "tblBodSleepLog",
        "tblBodStages",
        "tblBodCaffeine",
        "tblBodCaffeineDays",
        "tblBodCaffeineStack",
        "tblBodCaffeineDecay",
        "tblBodHygiene",
        "tblBodHygDays",
        "tblBodSTOPBANG",
    ] {
        assert!(tables.iter().any(|x| x == t), "table {t} missing");
    }
}

/// Excel serial of an ISO date.
fn serial(s: &str) -> f64 {
    (day(s) - day("1899-12-30")).num_days() as f64
}

fn hm(h: f64, m: f64) -> f64 {
    (h + m / 60.0) / 24.0
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    let syy: f64 = y.iter().map(|b| (b - my).powi(2)).sum();
    sxy / (sxx * syy).sqrt()
}

/// Typed rows for the lane D logs: (sheet, first header of the table, column, row, value).
enum V {
    N(f64),
    S(&'static str),
}

fn typed() -> Vec<(&'static str, &'static str, &'static str, u32, V)> {
    use V::{N, S};
    let mut v = Vec::new();
    // Sleep log: three nights, TIB 8 h each.
    for (i, (d, bed, wake, mins)) in [
        ("2024-04-20", hm(23.0, 0.0), hm(7.0, 0.0), 420.0),
        ("2024-04-21", hm(22.0, 30.0), hm(6.0, 30.0), 450.0),
        ("2024-04-22", hm(23.0, 30.0), hm(7.0, 30.0), 400.0),
    ]
    .into_iter()
    .enumerate()
    {
        let i = i as u32;
        v.push(("Bod_Sleep", "date", "date", i, N(serial(d))));
        v.push(("Bod_Sleep", "date", "bedtime", i, N(bed)));
        v.push(("Bod_Sleep", "date", "wake", i, N(wake)));
        v.push(("Bod_Sleep", "date", "minutes_asleep", i, N(mins)));
    }
    // Caffeine: coffee 07:30 100 mg, tea 14:00 50 mg.
    for (i, (t, src, mg)) in [(hm(7.0, 30.0), "coffee", 100.0), (hm(14.0, 0.0), "tea", 50.0)].into_iter().enumerate() {
        let i = i as u32;
        v.push(("Bod_Caffeine", "date", "date", i, N(serial("2024-04-22"))));
        v.push(("Bod_Caffeine", "date", "time", i, N(t)));
        v.push(("Bod_Caffeine", "date", "source", i, S(src)));
        v.push(("Bod_Caffeine", "date", "mg", i, N(mg)));
    }
    // Alcohol: 2, 0, 1 drinks on the three sleep-log evenings.
    for (i, (d, n)) in [("2024-04-20", 2.0), ("2024-04-21", 0.0), ("2024-04-22", 1.0)].into_iter().enumerate() {
        v.push(("Bod_Alcohol", "date", "date", i as u32, N(serial(d))));
        v.push(("Bod_Alcohol", "date", "drinks", i as u32, N(n)));
    }
    // Meals on 04-22: 08:00, 12:30, 19:00 eaten; a 16:00 snack skipped.
    for (i, (t, m, sk)) in [
        (hm(8.0, 0.0), "breakfast", "NO"),
        (hm(12.0, 30.0), "lunch", "NO"),
        (hm(19.0, 0.0), "dinner", "NO"),
        (hm(16.0, 0.0), "snack", "YES"),
    ]
    .into_iter()
    .enumerate()
    {
        let i = i as u32;
        v.push(("Bod_Window", "date", "date", i, N(serial("2024-04-22"))));
        v.push(("Bod_Window", "date", "time", i, N(t)));
        v.push(("Bod_Window", "date", "meal", i, S(m)));
        v.push(("Bod_Window", "date", "skipped", i, S(sk)));
    }
    // CGM: 04-22 fasting 90, post-meal 140 (spike), pre-meal 95, post-meal 120; 04-23 fasting 100.
    for (i, (d, t, mg, tag)) in [
        ("2024-04-22", hm(7.0, 0.0), 90.0, "fasting"),
        ("2024-04-22", hm(8.0, 30.0), 140.0, "post-meal"),
        ("2024-04-22", hm(12.0, 0.0), 95.0, "pre-meal"),
        ("2024-04-22", hm(13.0, 0.0), 120.0, "post-meal"),
        ("2024-04-23", hm(7.0, 0.0), 100.0, "fasting"),
    ]
    .into_iter()
    .enumerate()
    {
        let i = i as u32;
        v.push(("Bod_Glycemic", "cgm", "date", i, N(serial(d))));
        v.push(("Bod_Glycemic", "cgm", "time", i, N(t)));
        v.push(("Bod_Glycemic", "cgm", "mg_dl", i, N(mg)));
        v.push(("Bod_Glycemic", "cgm", "tag", i, S(tag)));
    }
    // Hygiene: 04-22, five rules kept.
    v.push(("Bod_SleepHyg", "date", "date", 0, N(serial("2024-04-22"))));
    for h in ["no_alcohol", "no_food_3h", "no_screens_2h", "wind_down_1h", "sauna_or_bath"] {
        v.push(("Bod_SleepHyg", "date", h, 0, S("YES")));
    }
    // Stages: two nights.
    for (i, (d, l, dp, rem, aw)) in
        [("2024-04-21", 240.0, 90.0, 100.0, 30.0), ("2024-04-22", 250.0, 60.0, 110.0, 20.0)].into_iter().enumerate()
    {
        let i = i as u32;
        v.push(("Bod_Arch", "date", "date", i, N(serial(d))));
        v.push(("Bod_Arch", "date", "light_min", i, N(l)));
        v.push(("Bod_Arch", "date", "deep_min", i, N(dp)));
        v.push(("Bod_Arch", "date", "rem_min", i, N(rem)));
        v.push(("Bod_Arch", "date", "awake_min", i, N(aw)));
    }
    v
}

/// Build the fixture workbook with the typed lane-D rows written over the blank inputs.
fn build_typed(dir: &std::path::Path) -> std::path::PathBuf {
    let mut plain = open(&build(dir));
    let mut wb = health_cli::workbook::build(&fixture(), &[]).unwrap();
    for (sheet, first, h, i, v) in typed() {
        let r = plain.worksheet_range(sheet).unwrap();
        let (hr, c) = if first == "cgm" {
            let (hr, c) = header(&r, "date", "mg_dl");
            match h {
                "date" => (hr, c - 2),
                "time" => (hr, c - 1),
                "mg_dl" => (hr, c),
                _ => (hr, c + 1),
            }
        } else {
            header(&r, first, h)
        };
        let ws = wb.worksheet_from_name(sheet).unwrap();
        let (row, col) = (hr + 1 + i, c as u16);
        match v {
            V::N(x) => ws.write_number(row, col, x).unwrap(),
            V::S(s) => ws.write_string(row, col, s).unwrap(),
        };
    }
    let p = dir.join("typed.xlsx");
    wb.save(&p).unwrap();
    p
}

#[test]
fn lane_d_sheets_build_on_an_empty_store() {
    let only: Vec<String> = ["bod-wearable", "bod-glycemic", "bod-window", "bod-alcohol", "bod-sleep", "bod-arch"]
        .into_iter()
        .chain(["bod-caffeine", "bod-hygiene"])
        .map(String::from)
        .collect();
    let mut wb = health_cli::workbook::build(&health_cli::workbook::Data::default(), &only).expect("builds");
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path().join("empty.xlsx");
    wb.save(&p).unwrap();
    let names = open(&p).sheet_names();
    for (s, _) in SHEETS {
        assert!(names.iter().any(|n| n == s), "{s} on an empty store");
    }
}

#[test]
fn lane_d_recalculates_without_errors_and_computes_typed_logs() {
    let dir = tempfile::TempDir::new().unwrap();
    let Some(so) = soffice() else {
        skip("soffice (LibreOffice) not installed: lane D recalculated values not checked");
        return;
    };
    let p = build_typed(dir.path());
    let mut wb = recalc(&so, dir.path(), &p);
    let errs: Vec<String> =
        errors(&mut wb).into_iter().filter(|e| SHEETS.iter().any(|(s, _)| e.starts_with(&format!("{s}!")))).collect();
    assert!(errs.is_empty(), "lane D formula errors: {errs:?}");

    // Caffeine: the tblSupp stack (100 mg "Day" = 08:00) and the typed day (04-22).
    let c = wb.worksheet_range("Bod_Caffeine").unwrap();
    let stack = 100.0 * 0.5f64.powf(14.5 / 6.0);
    let day = 100.0 * 0.5f64.powf(15.0 / 6.0) + 50.0 * 0.5f64.powf(8.5 / 6.0);
    close(num(&right_of(&c, "Supplement stack, mg at bedtime")), stack, "stack caffeine at bedtime");
    close(num(&right_of(&c, "Half-life used (h)")), 6.0, "half-life falls back to Caffeine_HL");
    close(num(&right_of(&c, "Latest logged day, mg at bedtime")), day, "logged day at bedtime");
    close(num(&right_of(&c, "Total at bedtime (stack + latest day)")), stack + day, "KPI cell");
    close(num(&at(&c, "hours_after", "mg_left", 6)), 50.0, "decay: half left after one half-life");
    close(num(&at(&c, "day", "total_mg", 27)), 150.0, "per-day total");

    // Sleep: TIB 8 h, wake SD over (420, 390, 450) min, 7-night debt.
    let s = wb.worksheet_range("Bod_Sleep").unwrap();
    close(num(&at(&s, "date", "tib_h", 0)), 8.0, "TIB across midnight");
    close(num(&at(&s, "date", "efficiency", 0)), 7.0 / 8.0, "efficiency");
    close(num(&at(&s, "date", "wake_sd_min", 2)), 30.0, "wake-time SD");
    close(num(&at(&s, "date", "debt_7n_h", 2)), 1.0 + 0.5 + (8.0 - 400.0 / 60.0), "7-night debt");
    close(num(&right_of(&s, "Mean asleep (h)")), (7.0 + 7.5 + 400.0 / 60.0) / 3.0, "sleep_h KPI cell");
    close(num(&right_of(&s, "Weekend drift: Sat/Sun wake minus Mon-Fri (min)")), 0.0, "weekend drift");

    // Alcohol vs the same night's sleep-log efficiency.
    let a = wb.worksheet_range("Bod_Alcohol").unwrap();
    let eff = [7.0 / 8.0, 7.5 / 8.0, 400.0 / 60.0 / 8.0];
    close(num(&right_of(&a, "r (drinks, next-night efficiency)")), pearson(&[2.0, 0.0, 1.0], &eff), "r drinks/eff");
    close(num(&right_of(&a, "Mean efficiency, 0-drink nights")), eff[1], "dry-night efficiency");

    // Eating window on 04-22: 08:00 to 19:00, one skipped snack.
    let w = wb.worksheet_range("Bod_Window").unwrap();
    close(num(&at(&w, "day", "window_h", 27)), 11.0, "window");
    close(num(&at(&w, "day", "meals_eaten", 27)), 3.0, "meals eaten");
    close(num(&at(&w, "day", "skipped", 27)), 1.0, "skipped");

    // CGM panel: one spike (140 after a fasting 90), 3 of 5 readings in 70-110.
    let g = wb.worksheet_range("Bod_Glycemic").unwrap();
    close(num(&right_of(&g, "Spikes (rise > Bod_Spike_rise)")), 1.0, "spikes");
    close(num(&right_of(&g, "Mean glucose (mg/dL)")), 109.0, "mean glucose");
    close(num(&right_of(&g, "% time in Bod_CGM_lo..Bod_CGM_hi")), 0.6, "time in range");

    // Hygiene joins for the night of 04-22 (last row of the per-day table).
    let h = wb.worksheet_range("Bod_SleepHyg").unwrap();
    close(num(&at(&h, "day", "score", 27)), 5.0, "score");
    close(num(&at(&h, "day", "meal_gap_h", 27)), 3.5, "last meal to bed");
    close(num(&at(&h, "day", "caffeine_gap_h", 27)), 8.5, "last caffeine to bed");
    close(num(&at(&h, "day", "first_cgm_next", 27)), 100.0, "next morning first CGM");
    close(num(&at(&h, "day", "sleep_h", 27)), 400.0 / 60.0, "sleep hours");
    assert_eq!(right_of(&h, "Chronotype (heuristic)").to_string(), "intermediate");
    assert_eq!(num(&right_of(&h, "Supplements in Before bed / Night slots")), 1.0, "Magnesium Glycinate");

    // Architecture: second night vs the first as baseline.
    let r = wb.worksheet_range("Bod_Arch").unwrap();
    close(num(&at(&r, "date", "deep_dev", 1)), 60.0 / 420.0 - 90.0 / 430.0, "deep share vs baseline");
    close(num(&at(&r, "date", "cycles", 1)), 440.0 / 90.0, "expected cycles");

    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let kt: String = k.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n");
    assert!(kt.contains("K_BOD_caffeine_bed") && kt.contains("K_BOD_sleep_h"), "lane D KPIs registered");
}
