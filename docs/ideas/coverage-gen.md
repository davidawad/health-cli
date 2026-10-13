# Coverage: W6 GEN (genetics cross-cuts)

Every GEN id of [MASTER.md](MASTER.md) section 3, the sheet that implements it and its status in the
generated workbook. Code: `src/workbook/sheets/gen/` (one module per sheet, constants in
`gen/params.rs`, the variant panel and rule rows in `gen/variants.rs`; one registry line in
`sheets/mod.rs`). Tests: `tests/sheet_gen.rs` plus unit tests in the modules.

Status as shipped: **BN** full build over data on file; **PH** full view over an empty input with the
`PLACEHOLDER - awaiting data` banner, computing as soon as the input arrives; **NS** view ships, the
constants and rule rows it relies on are `UNVERIFIED` with `pending: <reference>`.

**How genotypes get in.** genome-cli keeps genotypes; the shared `tblGenome` (rsid, gene, k1/k2 genotype,
concordant) is empty until genome-cli exports calls. `Gen_CrossCut` resolves one call per panel variant
(typed override > k2 WGS > k1 array, alleles sorted so `TC` = `CT`) and every other GEN sheet reads calls
through `crosscut::call` / `crosscut::count_of`. So every genotype-driven view is real formulas over
`tblGenome` that light up when calls arrive, and works today if David types a call.

| id | sheet | master status | shipped | what it reads | what is computed (Excel formulas) |
| --- | --- | --- | --- | --- | --- |
| GEN-01 | Gen_CrossCut | NS | NS | tblGenome, Ref_Allele, Calc_Marker, tblLabs, tblSupp | Per variant (33-row panel): k1/k2 lookup by rsid (INDEX/MATCH), normalised call, effect-allele copies, k1 vs k2 agreement, modulated marker's n / latest / slope / status, related supplement in stack (SUMPRODUCT/SEARCH), verdict untested / no lab link / not measured / concordant / discordant (`Gen_Adverse_status`). Verdict and per-area roll-ups (COUNTIF, SUMPRODUCT), bar chart. KPIs `K_GEN_n_called`, `K_GEN_n_effect`. Allele table UNVERIFIED pending SNPedia / ClinVar. |
| GEN-02 | Gen_QC | BN | BN (kit QC live) + PH (rsID sections) | tblKits (Genetics sheet), tblGenome, Gen_CrossCut, Sex | A: per kit called sites, no-call rate, heterozygous share, het/hom-alt ratio, sex call vs profile, flag vs `Gen_QC_NoCall_max` / `Gen_QC_Het_lo/hi` (live on the genome-cli summary). B: row-aligned with tblGenome: normalised k1/k2, no-calls, both-called, agreement, stored-`concordant` check, concordance %, pie. C: each panel rsID called by k1 / k2 / both / missing. KPIs `K_GEN_qc_nocall`, `K_GEN_concordance`. |
| GEN-03 | Gen_Exposure | PH | PH | tblGenAnnot (typed / imported annotations), Gen_CrossCut, tblGenome, tblLabs | Per annotation: my call, matches my genotype, bridges to a measured marker, MISSING SOURCE flag (EV GRADE principle). By system: annotations, matching, bad / good matching, summed magnitude, bridges (COUNTIFS / SUMIFS), stacked bar. Autofilter, data bars, validation. MED-VERIFY note. |
| GEN-04 | Gen_Scorecard | PH | PH | Gen_CrossCut, Ref_Allele, Calc_Marker, tblMarkers | Link rows (seeded from panel variants with a lab direction, 20 typed rows): copies, predicted sign, latest's position in its reference band, observed side, hit; pairs, hits, misses, hit rate, one-sided `BINOM.DIST` p. Chart. KPI `K_GEN_scorecard_hit`. |
| GEN-05 | Gen_APOE | NS | NS | Gen_CrossCut (rs429358, rs7412), tblRefApoe, Calc_Marker, tblLabs, tblSupp | Key `rs429358|rs7412` -> 9-row lookup -> diplotype, e4 / e2 copies; [DX-RISK] neutral pattern line. Checklist of 9 e4-relevant markers: measurements, last, date, days since, status, applies, NOT TESTED / due (`Gen_Retest_days`); habits with stack match. Chart. KPI `K_GEN_apoe_e4`. Table UNVERIFIED pending SNPedia APOE. |
| GEN-06 | Gen_Lipid | PH | PH | Gen_CrossCut, Ref_Allele, Calc_Marker, tblLabs, tblMarkers | LPA card: risk copies at rs10455872 / rs3798220, Lp(a) n / latest, "Lp(a) NOT TESTED" card. Lipid genes (8 variants): copies x expected direction vs marker latest / status, banner per row; ApoB banner. History tables walked with MINIFS over tblLabs (LDL-C, ApoB) + LDL-C ref upper limit, scatter chart. |
| GEN-07 | Gen_MTHFR | PH | PH | Gen_CrossCut (rs1801133, rs1801131), Calc_Marker, tblMarkers, tblSupp | Combined genotype label (677TT / compound het / ...), homocysteine / folate / B12 n, latest, x ref_hi, status, days since; lab-first verdict ([DX-RISK] wording, "the lab decides"); folate and B12 forms in the stack (8 keyword rows); chart. |
| GEN-08 | Gen_PGx | NS | NS | Gen_CrossCut, tblRefPgx / tblRefPgxDrug / tblRefPgxMod, tblSupp, tblGenRx (typed) | Star alleles: copies, function delta, Hardy-Weinberg probability of the call from a typed gnomAD frequency (`Gen_HWE_rare`). Genes: activity = `Gen_PGx_base` + SUMIFS(copies x delta), phenotype by `Gen_PGx_poor_max` / `Gen_PGx_normal`, strongest inhibitor (MINIFS) and inducer (MAXIFS) present -> effective activity / phenotype, phenoconverted flag. 21 drug rows: in use (stack or Rx), effective phenotype, [DX-RISK] flag. Chip-unreliable banner. KPI `K_GEN_pgx_flags`. All rules UNVERIFIED pending CPIC. |
| GEN-09 | Gen_Caffeine | NS | NS | Gen_CrossCut (rs762551, rs1801260), Caffeine_HL, tblSupp | Metaboliser from C copies; personal half-life = override > `Gen_Caffeine_HL_slow` (blank, UNVERIFIED) for C carriers > `Caffeine_HL`; published as defined name **`G_Caffeine_HL`** (BOD-29 reads it). mg at bedtime from dose / bed times (MOD), 0-24 h residual curve default vs personal (line chart); caffeine sources taken in the evening (time_of_day SEARCH), melatonin note; CLOCK card vs typed MEQ (`Gen_MEQ_morning/evening`, UNVERIFIED). KPI `K_GEN_caffeine_hl`. |
| GEN-10 | Gen_Cards | NS | NS | Gen_CrossCut, Gen_APOE (e2), Sex | Haplogroup card from typed mt / Y haplogroups (Y only when Sex is male). Longevity alleles (FOXO3, APOE e2, CETP, IL6): copies, "small effect" label, carried / possible / share, bar chart. KPI `K_GEN_longevity_alleles`. |
| GEN-11 | Gen_Todo | BN | BN (lab side live; triggers await calls) | tblGenTodoRules, Gen_APOE (e4), Gen_CrossCut, tblLabs | 11 rules (rule schema): trigger copies, fires?, measurements (COUNTIFS) and last draw (MAXIFS over tblLabs), days since, action ORDER never measured / ORDER due again (`Gen_Retest_days`) / done / awaiting genotype. Open orders, never measured, awaiting genotype; chart. KPI `K_GEN_todo_open`. |
| (rules) | Ref_Allele | - | NS | - | `tblRefAllele` (33 variants), `tblRefApoe` (9), `tblRefPgx` (12), `tblRefPgxDrug` (21), `tblRefPgxMod` (13), all in `id, rule, value, unit, source, source_state, note`, every row UNVERIFIED with its pending reference. |

Counts: BN 2 (GEN-02, GEN-11), PH 4 (GEN-03, 04, 06, 07), NS 5 (GEN-01, 05, 08, 09, 10): as in MASTER section 5.

**Parameters** (`Params`, defined names): heuristic `Gen_Retest_days`, `Gen_QC_NoCall_max`, `Gen_QC_Het_lo/hi`,
`Gen_Adverse_status`, `Gen_HWE_rare`, `Gen_PGx_base`, `Gen_Caffeine_dose_mg`; UNVERIFIED `Gen_PGx_poor_max`,
`Gen_PGx_normal` (pending CPIC), `Gen_Caffeine_HL_slow` (blank, pending SNPedia rs762551 / Cornelis 2006),
`Gen_MEQ_morning/evening` (pending Horne & Ostberg 1976). The shared seed `Caffeine_HL` is reused.

**KPIs** (10, the per-workstream maximum): `K_GEN_n_called`, `K_GEN_n_effect`, `K_GEN_qc_nocall`,
`K_GEN_concordance`, `K_GEN_scorecard_hit`, `K_GEN_apoe_e4`, `K_GEN_pgx_flags`, `K_GEN_caffeine_hl`,
`K_GEN_longevity_alleles`, `K_GEN_todo_open`.

**Open items.** genome-cli genotype export into `tblGenome` (infra `data.rs`, not this workstream); every
allele / star-allele / drug-gene row needs checking against SNPedia / ClinVar / CPIC before it flips to
verified; gnomAD frequencies for the Hardy-Weinberg check; haplogroups from genome-cli.
