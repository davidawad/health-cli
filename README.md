# health-cli

Rust CLI that is the **source of truth for non-lab personal health data**:
supplements, products, nutrition, routines, workouts, vaccines, physicals,
sizes and resource links. It replaces `Health Dashboard.xlsx` as an *input*
(`health import xlsx`) and then generates the workbook as an *output*
(`health sheet`). It is a sibling of
[`biomarker-cli`](../biomarker-cli) (labs) and [`genome-cli`](../genome-cli)
(genetics) and copies their conventions: FrankenSQLite
([fsqlite](https://crates.io/crates/fsqlite)) storage in an XChaCha20-Poly1305
sealed container unlocked by your SSH key, an encrypted tamper-evident audit
log, the versioned `health/v1` JSON envelope, and `doctor`, `key`, `db`,
`audit` and `config` subcommands. It builds a single binary, `health`.

Personal health data never leaves the machine: the database is encrypted at
rest, the repo holds code and synthetic fixtures only, and there is no
network code.

## Quick start

```console
$ cargo build --release
$ health import xlsx "~/desk/Personal/Medical/Health Dashboard.xlsx"   # read-only
SHEET                 ROWS_SEEN  ITEMS  HEADERS  DERIVED  OTHER
...
$ health doctor            # encryption, keys, permissions, audit chain
$ health stats             # row counts per table
$ health list supplements  # also: products nutrition routine workout vaccines ...
$ health sheet --out ~/desk/Personal/Medical/Health\ Dashboard\ \(generated\).xlsx
```

The first command that opens the database creates it, encrypted with your
`~/.ssh/id_ed25519` (the same key biomarker and genome use). Where it lives:
`~/.local/share/health-cli/health.db` (`health db path`), config in
`~/.config/health-cli/config.toml` (`health config path`).

## What is stored

Only inputs. Anything derivable (\$/yr, \$/month, weekly dose per day,
totals, chronological age, ideal weight, protein targets) is recomputed by
the workbook generator as native Excel formulas and never stored. Prices keep
their currency and the date they were last seen. See
[`docs/data-model.md`](docs/data-model.md) for every table and for how each
sheet of the legacy workbook maps onto it.

| entity | table(s) | source sheet |
|---|---|---|
| base facts (height, blood group, haplogroups...) | `profile_fact`, `predisposition`, `note` | Dashboard |
| supplements, dose schedule, protocols, evidence | `supplement`, `dose_schedule`, `source`, `supplement_source`, `evidence_ref` | Supplements |
| products | `product` | Products |
| nutrition items and targets | `nutrition_item`, `nutrition_target` | Nutrition |
| routines | `routine_step` | Routine |
| workout | `exercise`, `exercise_alt` | Workout |
| vaccines and conditions | `vaccine`, `vaccine_dose` | Vaccines  Conditions |
| physical checks, doctors | `physical_check`, `provider` | Physicals |
| sizes, resource links | `size`, `resource_link` | Sizes, Resources |

`Biomarkers`, `Bryans Biomarkers` and `Weight & Age` belong to biomarker-cli;
`Genetics` belongs to genome-cli; `Vitals (Whoop)` is empty. The importer
skips them and says so.

## Import rules

* The workbook is opened read-only and never modified.
* Section-heading rows ("Skin & Hair", "Core Meals", "JACKETS") become a
  `category`/`section` column, not rows. Total/sum rows are derived and skipped.
  `rows_seen = items + headers + derived + other` is reported per sheet, so
  every non-blank row of the sheet is accounted for.
* A formula that is itself an input (`=365 * 0.5 / 4`, `=11 - 1.5`) is stored as
  its cached value. For Google Sheets exports, cells wrapped in
  `__xludf.DUMMYFUNCTION("..."), cached)` already carry the cached value.
* The supplements `Source` column is a comma list: each name becomes a `source`
  (case-insensitive: `HOP Box` = `Hop Box`) linked many-to-many.
* Re-importing needs `--replace` (it wipes health data first).

## Commands

```
health import xlsx PATH [--replace] [--price-date YYYY-MM-DD]
health list <entity> [--category X]    facts supplements doses sources evidence products
                                       nutrition targets routine workout vaccines
                                       vaccine-doses physicals providers sizes resources notes
health stats
health sheet --out FILE                generate the workbook
health db path|init|info|check|backup|rekey|migrate|vacuum|encrypt
health key status|add-ssh|add-passphrase|add-file|remove
health audit log
health doctor
health config show|set|unset|path|keys
health completions SHELL | health man
```

Every command takes `--format table|json|jsonl|csv|tsv|yaml|toon|markdown|html|org`,
`--columns`, `--output FILE`, `--encrypt-output` (age) and friends. JSON
output is the `health/v1` envelope ([`docs/json-schema.md`](docs/json-schema.md)).
Exit codes: 0 ok, 1 error, 2 usage, 3 not found, 4 invalid data, 5 database,
6 io, 7 config, 8 key.

## Security

Same design as biomarker-cli: [`docs/security.md`](docs/security.md).

## Development

```console
$ just test-gate     # cargo fmt --check, clippy -D warnings, cargo test
```

Tests use a **synthetic** workbook generated in `tests/support/mod.rs`. Never
commit a real workbook or real personal data; `*.xlsx` is git-ignored.
