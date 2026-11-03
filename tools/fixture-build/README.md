# tools/fixture-build — rebuild the programmability fixture

Rebuilds `fixtures/programmability.accdb` in a scratch directory with real
Microsoft Access (COM automation) and compares the rebuilt object inventory
against the committed fixture. The object texts (`objects/`, UTF-8 in the
repository) are converted at build time into the encodings Access expects
(UTF-16LE for forms/reports/macros, cp1252 for modules); table DDL, row
inserts and query SQL live as plain SQL text under `sql/`.

Prerequisites, provenance and the full creation recipe:
`docs/FIXTURE-SOURCES.md` ("Programmability fixture"). Windows + Access
only — the tool is never part of CI; the committed fixture stays the
evidence and CI keeps verifying its manifest hash offline.

```
powershell -NoProfile -ExecutionPolicy Bypass -File build.ps1 [-OutDir <dir>] [-CliPath <datacraft-cli.exe>] [-KeepScratch]
```

The inventory comparison runs through the `datacraft-cli` snapshot (the
`mdb-tools` oracle is Linux CI-only); the rebuilt file's sha256 is recorded
under the scratch directory and intentionally differs from the committed
pin — creation timestamps live inside the file, so acceptance compares the
object inventory, not bytes. The script fails fast on any COM error, never
writes outside the scratch directory, never opens the committed fixture,
and leaves no `MSACCESS.EXE` process or `.laccdb` lock file behind.
