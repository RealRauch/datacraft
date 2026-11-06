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

# Compact a database into a new file (maintenance observations)
powershell ... oracle.ps1 compact C:\path\src.mdb C:\path\dst.mdb

# Dump page 0 facts: version, product string and the 0xE00..0x1000 tail words
powershell ... oracle.ps1 dump C:\path\candidate.mdb
```

Exit codes: `0` the command succeeded (for `open-test`: the engine accepted
the file), `1` failure (for `open-test`: the engine refused — the message
prints on stdout as `FAIL: <text>`).

## Hygiene rules

- Run against scratch directories only (`target/tmp`, `.zcode/`, `%TEMP%`) —
  never against committed fixtures (immutability rule) or user data.
- A failed `open-test` may leave a stale `<name>.ldb` lock file next to the
  candidate; the harness removes it after every call so scratch directories
  stay reusable (the incident that founded M63 was exactly such a leftover).
- `open-test` opens the database shared and closes it immediately — it does
  not modify the file's pages. Do not treat an opened file as pristine
  evidence without re-verifying its hash.
