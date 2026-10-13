//! Typing values into the blue input cells of a generated workbook (as a person
//! would) and recalculating it in headless LibreOffice.

use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

use calamine::open_workbook;

use super::Book;

/// A value typed into a blue cell.
pub enum Typed {
    S(&'static str),
    N(f64),
}

/// Copy `src` to `dst` with `edits` typed into blank cells of the worksheet part `xml`.
pub fn type_into(src: &Path, dst: &Path, xml: &str, edits: &[(String, Typed)]) {
    let mut z = zip::ZipArchive::new(std::fs::File::open(src).unwrap()).unwrap();
    let mut w = zip::ZipWriter::new(std::fs::File::create(dst).unwrap());
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        if name == xml {
            let mut s = String::from_utf8(buf).unwrap();
            for (cell, v) in edits {
                let open = format!("<c r=\"{cell}\"");
                let at = s.find(&open).unwrap_or_else(|| panic!("{xml}: no cell {cell}"));
                let end = at + s[at..].find("/>").unwrap() + 2;
                let attrs = s[at + open.len()..end - 2].to_string();
                assert!(!attrs.contains('>'), "{cell} is not a blank cell");
                let new = match v {
                    Typed::S(t) => format!("{open}{attrs} t=\"inlineStr\"><is><t>{t}</t></is></c>"),
                    Typed::N(x) => format!("{open}{attrs}><v>{x}</v></c>"),
                };
                s.replace_range(at..end, &new);
            }
            buf = s.into_bytes();
        }
        w.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(&buf).unwrap();
    }
    w.finish().unwrap();
}

/// A1 reference of a zero-based (row, col).
pub fn a1(row: u32, col: u32) -> String {
    let mut c = col + 1;
    let mut s = String::new();
    while c > 0 {
        s.insert(0, (b'A' + ((c - 1) % 26) as u8) as char);
        c = (c - 1) / 26;
    }
    format!("{s}{}", row + 1)
}

pub fn soffice() -> Option<String> {
    ["soffice", "/Applications/LibreOffice.app/Contents/MacOS/soffice", "/opt/homebrew/bin/soffice"]
        .iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()))
        .map(|c| (*c).to_string())
}

pub fn recalc(soffice: &str, dir: &Path, input: &Path) -> Book {
    let profile = dir.join("lo-profile");
    std::fs::create_dir_all(profile.join("user")).unwrap();
    std::fs::write(
        profile.join("user/registrymodifications.xcu"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>"#,
    )
    .unwrap();
    let outdir = dir.join("recalculated");
    let out = Command::new(soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(input)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    open_workbook(outdir.join(input.file_name().unwrap())).unwrap()
}

/// Excel serial of a date.
pub fn serial(y: i32, m: u32, d: u32) -> f64 {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    (chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap() - base).num_days() as f64
}
