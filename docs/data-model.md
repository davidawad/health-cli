# Data model

Everything the legacy `Health Dashboard.xlsx` held outside labs and genetics.
Schema v1 lives in `src/migrations.rs`. Principles:

* **Inputs only.** `$/yr`, `$/mo`, weekly dose per day, totals, chronological
  age, ideal weight, water and protein targets, "qualifying meal?" are
  formulas in the workbook and are *derived*: the generator recomputes them
  as native Excel formulas.
* **Section-heading rows are a column** (`category`, `section`, `routine`),
  never rows.
* **Prices** carry `price_currency` (`USD`) and `price_date`: the date the
  price was last seen. Imported prices are stamped with the workbook's
  modification date (override: `--price-date`).
* **Unknown is `NULL`.** A blank `Taking` cell is `NULL`, not `0`.
* `position` keeps sheet order. For `physical_check` and `provider` it is the
  sheet row number, so the generated sheet keeps the original layout.

| table | columns (besides `id`) | notes |
|---|---|---|
| `profile_fact` | `key` (PK), `label`, `value`, `kind` text/number/date, `unit`, `position` | Dashboard "Base Stats": height, blood group, Rh, birthdate, haplogroups, ideal BMI, calories burned |
| `predisposition` | `position`, `kind`, `name`, `description`, `prevalence` | Dashboard disease / predisposition table |
| `source` | `name` (unique), `url`, `notes` | a protocol or product box ("Bryan Johnson's Protocol", "Nootrobox Rise", "Tally-Amplify", "HOP Box") |
| `supplement` | `position`, `name` (unique), `kind` Rx/Nt/Sp, `notes`, `experience`, `units_per_year`, `price`, `price_currency`, `price_date`, `price_note`, `buy_url` | `price_note` holds non-numeric prices ("rx only") |
| `dose_schedule` | `supplement_id`, `taking`, `frequency`, `time_of_day`, `amount`, `amount_text`, `unit` | one per supplement now; many are allowed. `amount_text` keeps "1/2 pill" |
| `supplement_source` | `supplement_id`, `source_id` | many-to-many ("Source" column is a comma list) |
| `evidence_ref` | `supplement_id` (nullable), `url`, `kd_work_id` (nullable), `stance` supporting/warning/neutral, `title`, `note`, `added_at` | the Research column imports as neutral URL refs |
| `product` | `position`, `category`, `name`, `price`, `price_currency`, `price_date`, `dosage`, `in_use`, `units_per_year`, `notes`, `url`, `alt_url` | |
| `nutrition_target` | `position`, `label`, 12 nutrient columns, `extra` | "Recommended DV" and "Your goal values" rows |
| `nutrition_item` | `position`, `category`, `name`, 12 nutrient columns, `ingredients_ok`, `no_prep`, `vegan`, `qualifying`, `portions`, `notes`, `price*`, `url`, `alt_url` | `total_fat` only where it was typed (else it is `SUM(unsat, trans, sat)`) |
| `routine_step` | `routine` morning/night, `position`, `name`, `effect_time`, `exec_seconds`, `kind`, `notes` | |
| `exercise`, `exercise_alt` | `position`, `name`, `muscles`; alternatives | |
| `vaccine`, `vaccine_dose` | `kind` vaccine/condition, `name`, `disease_date`, `disease_note`, `notes`; `dose_no`, `given_on`, `raw` | `raw` keeps non-date entries such as "NOTSURE" |
| `physical_check` | `position` (sheet row), `section`, `name`, `detail`, `done` | |
| `provider` | `position` (sheet row), `name`, `note`, `url` | the "Doctors" block |
| `size` | `position`, `category`, `name`, `value`, `unit`, `note` | inches, except shoes (`us`) |
| `resource_link` | `position`, `section` resources/workout, `label`, `url` | |
| `note` | `scope`, `position`, `text` | free text: Dashboard TODOs, the "Alternative" cost comparison |
| `intake_log` | `date` (PK), `net_kcal`, `energy_out_kcal`, `protein_g` | daily intake from `Weight & Age` columns F, G, H; weight stays in biomarker-cli |
| `import_batch` | `source_name`, `sha256`, `imported_at`, `counts` | provenance of each import |

## Sheet mapping caveats

* **Physicals** has no structural marker for section titles (they are just
  bold cells), so `Regular Checks`, `Genetic Tests` and `Referrals` are matched by name.
* **Sizes**: the cm column was `inches * 2.54`, including for the shoe size.
  Shoes are stored with unit `us` and get no cm value.
* **Nutrition** `Qualifying Meal?` is a formula on almost every row; only
  literal entries are stored (`qualifying`).
