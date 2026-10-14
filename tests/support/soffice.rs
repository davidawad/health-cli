//! Headless LibreOffice for the recalculation tests. Every test that recalculates
//! a generated workbook finds `soffice` here, so the lookup and the "skip or fail"
//! rule live in one place (also pulled into the library's `pheno_check` by path).
//!
//! Without `soffice` a recalculation test records a SKIP and returns, unless the
//! gate sets `HEALTH_REQUIRE_RECALC=1` (or `RECALC_REQUIRED=1`): then a missing
//! LibreOffice is a test failure, never a silent pass. Install it without root
//! with `maintenance/scripts/install-soffice.sh`.

#![allow(dead_code)]

use std::process::Command;

/// Environment variables that turn a missing `soffice` into a failure.
pub const REQUIRE_VARS: [&str; 2] = ["HEALTH_REQUIRE_RECALC", "RECALC_REQUIRED"];

/// Whether the runner demands the recalculation tests actually run.
pub fn required() -> bool {
    REQUIRE_VARS.iter().any(|v| std::env::var(v).is_ok_and(|s| s == "1"))
}

/// Where to look: `$SOFFICE`, `PATH`, the user-space install of
/// `install-soffice.sh`, then the macOS app bundle and Homebrew.
fn candidates() -> Vec<String> {
    let mut c: Vec<String> = std::env::var("SOFFICE").into_iter().filter(|s| !s.is_empty()).collect();
    c.push("soffice".into());
    if let Some(home) = std::env::var_os("HOME") {
        c.push(format!("{}/.local/opt/libreoffice/bin/soffice", home.to_string_lossy()));
    }
    c.push("/Applications/LibreOffice.app/Contents/MacOS/soffice".into());
    c.push("/opt/homebrew/bin/soffice".into());
    c
}

/// A working `soffice`, or None when LibreOffice is not installed.
///
/// Panics instead of returning None when [`required`]: the caller's skip path
/// is then unreachable and the test fails loudly.
pub fn soffice() -> Option<String> {
    let found =
        candidates().into_iter().find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()));
    assert!(
        found.is_some() || !required(),
        "soffice (LibreOffice) not found but {} demands the recalculation tests run: \
         install it with maintenance/scripts/install-soffice.sh or point $SOFFICE at it",
        REQUIRE_VARS.join("/"),
    );
    found
}
