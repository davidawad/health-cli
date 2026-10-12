# Security: encryption at rest, keys, audit trail

health-cli uses the same encryption design as biomarker-cli and genome-cli;
the long form (threat model, fsqlite facts, container format, key slots,
cost) is [`biomarker-cli/docs/security.md`](../../biomarker-cli/docs/security.md).
Summary of what applies here:

* **Sealed container.** The database lives in memory as an fsqlite image;
  every committing statement re-seals the whole image with
  XChaCha20-Poly1305 into `health.db` (atomic rename, mode 0600, directory
  0700). No plaintext journal, WAL or temp file is ever created. fsqlite's own
  `PRAGMA key` does not encrypt (see the biomarker doc), so it is not used.
* **Keys.** The default key slot is your SSH key (`~/.ssh/id_ed25519`, an age
  message to the public key), so there is nothing new to remember. More slots:
  `health key add-passphrase|add-file|add-ssh`; `HEALTH_KEY=raw:<hex>` for
  scripts. `health key status` lists them and `health doctor` shows how to
  recover on another machine.
* **Audit log.** `<db>.audit` is an encrypted, hash-chained record of every
  command that touched the database (command path only, never arguments:
  they may hold health data). `health audit log` verifies and prints it.
* **Output.** `--output FILE` writes mode 0600 and warns that it is
  plaintext; `--encrypt-output` writes an age file. `health sheet` writes a
  plaintext workbook by design (that is its job): the default path is
  outside the repo, under `~/desk`, and `*.xlsx` is git-ignored.
* **Fixtures.** Tests run on a synthetic workbook generated in code. No
  real data is committed.

The container is the same binary format as biomarker-cli's (magic `BMSEAL02`),
so the same recovery procedure and tooling apply.
