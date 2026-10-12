//! Encryption at rest, as in biomarker-cli: the store is a sealed container,
//! the wrong key is refused, tampering is detected and the audit log chains.

mod support;

use predicates::prelude::*;
use support::Env;

#[test]
fn wrong_key_is_refused_with_the_key_exit_code() {
    let e = Env::new();
    e.import();
    e.cmd()
        .env("HEALTH_KEY", "raw:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
        .args(["stats"])
        .assert()
        .failure()
        .code(8);
}

#[test]
fn a_flipped_byte_is_detected() {
    let e = Env::new();
    e.import();
    let mut bytes = std::fs::read(e.db()).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(e.db(), bytes).unwrap();
    e.cmd().args(["stats"]).assert().failure().code(5).stderr(predicate::str::contains("tampered"));
}

#[test]
fn a_plaintext_database_is_not_created_without_the_flag() {
    let e = Env::new();
    e.import();
    assert!(!std::fs::read(e.db()).unwrap().starts_with(b"SQLite format 3"));
    e.cmd().args(["db", "info"]).assert().success();
    let sidecars: Vec<_> = ["-wal", "-shm", "-journal"].iter().map(|s| format!("{}{s}", e.db().display())).collect();
    assert!(sidecars.iter().all(|p| !std::path::Path::new(p).exists()), "no plaintext sidecars");
}

#[test]
fn the_audit_log_chains_and_detects_tampering() {
    let e = Env::new();
    e.import();
    e.run(&["stats"]);
    let log = e.json(&["audit", "log"]);
    let commands: Vec<&str> = log["data"].as_array().unwrap().iter().map(|r| r["command"].as_str().unwrap()).collect();
    assert_eq!(&commands[..2], ["import xlsx", "stats"]);
    assert_eq!(log["verified"], true);
    // Arguments (paths, values) are never logged.
    assert!(!log.to_string().contains("Health Dashboard"));
    let audit = format!("{}.audit", e.db().display());
    let mut b = std::fs::read(&audit).unwrap();
    let mid = b.len() / 2;
    b[mid] ^= 0x01;
    std::fs::write(&audit, b).unwrap();
    e.cmd().args(["audit", "log"]).assert().failure();
}

#[test]
fn key_status_lists_the_slot_and_doctor_explains_recovery() {
    let e = Env::new();
    e.import();
    let status = e.json(&["key", "status"]);
    assert!(status["count"].as_u64().unwrap() >= 1);
    let doctor = e.run(&["doctor"]);
    assert!(doctor.contains("recover") && doctor.contains("unlocks with its"), "{doctor}");
}
