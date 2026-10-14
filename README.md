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

Prebuilt macOS binaries come from the `Release` workflow on a version tag;
on carbon install those rather than building (see
[`docs/release.md`](docs/release.md)).

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
| daily intake (net kcal, energy out, protein) | `intake_log` | Weight & Age (columns F-H) |

`Biomarkers`, `Bryans Biomarkers` and the weight column of `Weight & Age` belong to biomarker-cli;
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
health export regimens [-p P] [--start DATE] [--research DIR] [--apply]
                                       supplements -> biomarker regimens (dry run by default)
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

## Supplement regimens live in biomarker-cli

biomarker-cli (>= 0.7.1) owns per-person supplement regimens (taking, dose,
frequency, time slot, start and stop days) and the interaction grid, next to the
labs; the 87 `research/supplements/<slug>` documents are its interventions.
`health export regimens` turns the store's supplement rows into the
`biomarker regimen add SLUG -p PERSON ...` commands that carry them over:

* Names are matched to research slugs by name, slug or alias (case and punctuation
  ignored); the research index is compiled in, `--research DIR` reads another tree.
* Taken rows become `regimen add` with `--dose`/`--unit` (a text amount such as
  "1/2 pill" goes into `--note`), `--frequency`, `--slot` and `--start`. The store
  has no start dates yet, so taken rows report `needs-start` until `--start DATE`
  is given. Rows not taken, with an unknown status or with no slug are `skip`.
* The default is a dry run: the deterministic plan (`kind: regimen_plan`, one row
  per supplement with its `action`, `reason` and `argv`) on stdout. `--apply` runs
  it with `biomarker_cmd`, refuses while any row needs a start date, and skips
  regimens biomarker already has (same intervention and start day), so a re-run is
  safe. Neither side writes files: `-o` is refused and never passed to biomarker.

## The generated workbook

`health sheet [--out FILE]` rebuilds every sheet of the original workbook from the
store, plus the lab sheets from `biomarker query -p <person> --units us --format json`
and the Genetics sheet from `genome kits|summary --format json` (both optional: a
missing tool only drops its sheets, with a warning). Supplement taking status and
start/stop days come from `biomarker regimen ls -p <person>` and the `Sup_Regimen`
view adds taking-vs-not and `biomarker interaction grid -p <person> --catalog`; with
an older biomarker the store's `taking` column is used and the grid is a placeholder. The default output is
`~/desk/Personal/Medical/Health Dashboard (generated).xlsx` (setting `sheet_out`),
never the original; an existing file is only replaced when its name contains
`(generated)` (or with `--force`). The file is plaintext by nature, written mode 0600.

* Excel-native formulas only: `$/yr`, weekly dose per weekday (a lookup over a hidden
  frequency map), ages, BMI, totals, "qualifying meal", cost shares. No `FILTER` or
  `__xludf.DUMMYFUNCTION`.
* Real Excel tables, drop-downs for YES/NO, type, frequency, time, stance, step type,
  conditional formats (out-of-range labs against each row's reference low/high, BMI
  readout, time of day, evidence stance), and charts (weight, BMI, ages, cholesterol,
  glucose, A1c, weekly supplement breakdown, cost breakdown, protocols, genome).
* Section headings of the old sheets are a `Category` column in real tables.
* The workbook opens on `Nav`, an index of every sheet by workstream with its BN / PH / NS
  status (from `docs/ideas/coverage-*.md`), then the executive dashboard `Dash` (reads only
  `K_*` KPI and `Params` names), `Alerts`, `Changes`, `Scenario`, `YearReview`, `Checks`
  (data integrity, reconciliation, a formula-error sweep of every sheet and one
  ALL CHECKS PASS cell), `Export` (a redactable "book on me") and `Sources`
  ([`docs/ideas/coverage-glb.md`](docs/ideas/coverage-glb.md)).
* New views beyond the original: `Evidence`, `Protocols`, `Costs`, and a hidden `Lists`.
  `health sheet --list` names every sheet; `--only a,b` builds a subset.
  Add your own in one module and one registry line: [`docs/adding-a-view.md`](docs/adding-a-view.md).

Differences from the legacy workbook (documented, deliberate): `Vitals (Whoop)` (empty) and
`Bryans Biomarkers` (hidden) are not generated; the Biomarkers `Control` column and the
Weight & Age calories/protein-intake columns have no source in biomarker-cli; Genetics shows
kit counts instead of 23,000 SNPedia annotation rows (genome-cli stores genotypes only).

## Security

Same design as biomarker-cli: [`docs/security.md`](docs/security.md).

## Development

```console
$ just test-gate     # cargo fmt --check, clippy -D warnings, cargo test
```

Tests use a **synthetic** workbook generated in `tests/support/mod.rs`. Never
commit a real workbook or real personal data; `*.xlsx` is git-ignored.
