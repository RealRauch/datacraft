# DAO oracle — Windows-local Jet acceptance harness

Black-box oracle for the Jet create-path work (item M63, phase 49). It drives
the real Jet engine installed with Microsoft Access (`DAO.DBEngine.120` COM
automation) to **create** reference databases and to **open** candidate files,
and reports the engine's verdict verbatim. This is own black-box observation of
files we generate — clean-room discipline is untouched (no writer source is
read; `docs/ARCHITECTURE.md` §12).

The oracle answers two questions the CI fixture suite cannot:

1. **Does the real Jet engine accept this file?** (`open-test` — the same
   engine gates Access; the localized error message names the refusal class)
2. **What does a real writer emit?** (`new-db`, `add-table`, `insert`,
   `compact` — reference files for byte-level differential study)

## Usage (Git Bash, Windows with Access installed)

```bash
# Create a fresh Jet4 database (must not exist)
powershell -NoProfile -ExecutionPolicy Bypass -File tools/dao-oracle/oracle.ps1 new-db C:\path\scratch\fresh.mdb

# DDL/DML through the engine (Jet SQL, one statement per call)
powershell ... oracle.ps1 exec C:\path\scratch\fresh.mdb "CREATE TABLE t1 (a LONG, b TEXT(50))"
powershell ... oracle.ps1 exec C:\path\scratch\fresh.mdb "INSERT INTO t1 VALUES (1, 'x')"

# Acceptance check: opens the file with the Jet engine and reports OK/FAIL
powershell ... oracle.ps1 open-test C:\path\scratch\candidate.mdb

# Enumerate the engine's TableDefs (name list + count) — the M63 visibility check
powershell ... oracle.ps1 listdefs C:\path\scratch\candidate.mdb

# Run a SELECT through the engine and print the rows ('|'-separated)
powershell ... oracle.ps1 query C:\path\scratch\candidate.mdb "SELECT Id, Name FROM T ORDER BY Id"

# Compact a database into a new file (maintenance observations)
powershell ... oracle.ps1 compact C:\path\src.mdb C:\path\dst.mdb

# Dump page 0 facts: version, product string and the 0xE00..0x1000 tail words
powershell ... oracle.ps1 dump C:\path\candidate.mdb
```

Exit codes: `0` the command succeeded (for `open-test`: the engine accepted
the file), `1` failure (for `open-test`: the engine refused — the message
prints on stdout as `FAIL: <text>`).

## Real-Access verification checklist (step 49.3, item M63 — user gate)

The automated DAO oracle (this harness) accepted the create path on
2026-11-07; the remaining gate is a human check in **real Access**. Ten
minutes, in order — every step must hold before 49.3 is marked done:

1. **Create.** `datacraft-cli exec "{\"CreateDatabaseFromTemplate\":{\"path\":\"C:\temp\m63.mdb\",\"template\":\"contacts\"}}"` (or the desktop: File ▸ New from template). Expected: no error.
2. **Open in Access.** Double-click `m63.mdb`. Expected: opens without a
   repair prompt; the standard untrusted-location content bar may appear
   (Access shows it for any database outside the Trusted Locations — enable
   content or continue, it never gates table reads/writes); the navigation
   pane shows **Contacts** plus the hidden system tables (Access options ▸
   show system objects).
3. **Read.** Open Contacts. Expected: the four template rows (Ada Lovelace, Grace Hopper, Edsger Dijkstra, Margaret Hamilton) with correct e-mail/phone/birthday cells.
4. **Write from Access.** Add a row, edit a cell, delete the added row. Expected: all three work without a write-conflict message.
5. **Reopen in DataCraft.** Open the same file in DataCraft. Expected: the edits from step 4 are visible; no decode errors.
6. **Write from DataCraft.** Insert a row through the DataCraft datasheet. Expected: persists.
7. **Reopen in Access.** Expected: the DataCraft row is there; the autonumber continues from the last value.
8. **Schema.** In Access, run Design on Contacts (view only) and create a small second table. Expected: no repair; the second table appears in DataCraft on reopen.
9. **Drop.** Delete the second table in Access. Expected: gone in DataCraft on reopen.
10. **Hygiene.** After every Access session, no stray `.ldb` next to the file (the incident that started M63).

Report any deviation with the exact step number and the Access message text —
the differential loop (oracle + byte bisection) resumes from there. Evidence
basis: FORMAT-JET "Engine-accepted create shape" and "Index sort keys and leaf
maintenance".

## Hygiene rules

- Run against scratch directories only (`target/tmp`, `.zcode/`, `%TEMP%`) —
  never against committed fixtures (immutability rule) or user data.
- A failed `open-test` may leave a stale `<name>.ldb` lock file next to the
  candidate; the harness removes it after every call so scratch directories
  stay reusable (the incident that founded M63 was exactly such a leftover).
- `open-test` opens the database shared and closes it immediately — it does
  not modify the file's pages. Do not treat an opened file as pristine
  evidence without re-verifying its hash.
