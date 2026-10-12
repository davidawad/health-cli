# health JSON interface (`health/v1`)

`--format json` wraps every result in one envelope; `--format jsonl` streams
the records. The conventions are those of `biomarker/v1`.

```json
{
  "schema": "health/v1",
  "kind": "supplements",
  "generated_at": "2026-10-12T12:00:00Z",
  "count": 87,
  "data": [ ... ]
}
```

* `schema` is always `health/v1`. Within v1, fields are only ever added.
* `kind` names what `data` holds (`supplements`, `products`, `nutrition`,
  `import_xlsx`, `stats`, `doctor`, `audit_log`, `db_info`, ...).
* `count` is present when `data` is an array; extra `meta` fields (for
  example `sha256` and `price_date` on `import_xlsx`) sit next to it.
* Numbers are JSON numbers at full precision (`--precision` only affects
  text formats). Dates are ISO strings. Missing values are `null`.
  Booleans stored as integers (`taking`, `in_use`, `done`) are `0`/`1`,
  and `null` when the workbook cell was blank (unknown).
* Errors with `--format json`: `{"schema","kind":"error","error":{"kind","code","message"}}` on stderr.

Record fields are the table columns of [`data-model.md`](data-model.md);
`list supplements` additionally flattens the first dose schedule
(`taking`, `frequency`, `time_of_day`, `amount`, `amount_text`, `unit`) and
adds `sources` (comma-joined), `evidence` (count) and `research_url`.

Exit codes: 0 ok, 1 general, 2 usage, 3 not found, 4 invalid data,
5 database, 6 io, 7 config, 8 key.
