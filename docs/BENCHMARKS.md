# DataCraft benchmarks — family A against DAO (phase 103, item M134)

> Status: first measured delivery (2026-11-27, phase 103 step 103.3).
> Instrument: `tools/dao-oracle/bench.ps1` (method + harness in one place;
> the DAO-oracle README documents the black-box ground rules — timing is
> own observation, clean-room discipline untouched).
> This is an honesty and target-metric instrument (roadmap §18), never a
> marketing surface. Every number carries its environment; a mixed
> scoreboard is the expected outcome.

## Method (roadmap §18 controls)

- **Family A, engine level:** DataCraft headless over the shipped
  `datacraft-cli` (release build) against `DAO.DBEngine.120` COM
  automation — the phase-49 oracle stack — on the same machine against
  the same generated Jet4 `.mdb` files. Both sides pay their client
  stack: DataCraft numbers include process start or the persistent
  `repl` JSON-RPC round-trip; DAO numbers include the PowerShell COM
  marshaling. Neither side is corrected.
- **Fixtures:** generated deterministically at bench time by
  `bench.ps1 gen` (never committed — only the recipe is tracked):
  table `a` (id LONG primary-indexed, idx LONG indexed, val TEXT(50);
  wide adds extra TEXT(255)), table `b` (100 join rows), the saved
  `QJoin` equality-join query, and the index-free insert table `c`.
- **Statistics:** N = 10 runs per point, median + p25/p75. Insert
  throughput is one timed block of 1000 rows (`rows_per_s`).
- **Warm file cache is the documented default** (not cleanly
  enforceable on Windows). No other load ran during the measurements.
- **Environment (captured by `bench.ps1 env`, travels with every
  number):** DAO/ACE engine version 16.0 · Windows 10.0.26200 · AMD
  Ryzen 5 3600X · 16 GB RAM · power plan **Energiesparmodus
  (Power Saver)** — recorded as-is; a re-run on the balanced plan is
  expected to move absolute numbers, not the ranking.
- **Benchmarks never run in CI** (runner variance); the harness is
  Windows-local, PowerShell 5.1-compatible, no dependencies.

## Results (medians in ms; p25–p75 in the harness output)

### Cold open (file → catalog; `*_open`)

| Class | DAO OpenDatabase | CLI end-to-end | CLI minus startup |
|---|---|---|---|
| 10k narrow | 2.26 | 9.39 | ≈ 2.2 (startup 7.20) |
| 10k wide | 2.25 | 9.61 | ≈ 2.4 |
| 100k narrow | 2.02 | 14.76 | ≈ 7.6 |
| 100k wide | 2.22 | 16.37 | ≈ 9.2 |
| 1M narrow | 2.57 | 73.40 | ≈ 66 |

Read: the pure open work is comparable at 10k and grows mildly to 100k
(catalog + health check + facts). The DAO column pays only COM
marshaling; the CLI column pays a full process start — the third
column is the honest comparison and shows parity at 10k.

### Full scan (`*_scan`, MoveNext loop vs one shadow-engine SELECT)

| Class | DAO | DataCraft CLI |
|---|---|---|
| 10k narrow | 55.8 | 25.8 |
| 10k wide | 54.6 | 30.9 |
| 100k narrow | 592.2 | 265.6 |
| 100k wide | 581.0 | 307.9 |
| 1M narrow | 5 883.6 | 2 715.5 |

Read: DataCraft decodes ~2× faster than the DAO COM loop at both sizes
— the row decode is in-process Rust against the same file.

### Filtered query on an indexed field (`idx = 42`; `*_filter`)

| Class | DAO OpenRecordset | DataCraft shadow SQL |
|---|---|---|
| 10k narrow | 1.59 | 23.6 |
| 10k wide | 1.52 | 26.5 |
| 100k narrow | 8.19 | 230.2 |
| 100k wide | 8.30 | 262.2 |
| 1M narrow | 173.5 | 2 377.4 |

Read: **Access wins, as the roadmap predicted** (§18: "the full-scan
cache … and the M15 index leaves are open"). The shadow engine
materializes the whole table before filtering; the engine uses the
B-tree. This is the target metric for §16 stage 1 (windowed reads) and
the M15 index-page work — no change claimed until those land.

### Join query (`QJoin`, equality inner join + WHERE; `*_join`)

| Class | DAO | DataCraft native exec |
|---|---|---|
| 10k narrow | 2.22 | 14.3 |
| 10k wide | 2.27 | 17.1 |
| 100k narrow | 2.15 | 142.5 |
| 100k wide | 2.29 | 170.1 |
| 1M narrow | 2.19 | 1 419.3 |

Read: Access wins; the native-exec join materializes FROM tables
before the hash-free equality join, while the engine drives the
primary index. Same target metrics as the filtered row.

### Insert throughput (1000 rows, `rows_per_s`; index-free table `c`)

| Class | DAO AddNew (transaction) | DataCraft journal batch |
|---|---|---|
| 10k narrow | 8 551 | 5 330 |
| 10k wide | 7 131 | 4 718 |
| 100k narrow | 8 234 | 4 800 |
| 100k wide | 8 752 | 4 796 |
| 1M narrow | 9 311 | 3 633 |

Read: same order of magnitude; DAO is ~1.6–1.8× ahead. The DataCraft
number pays the wire round-trip, the preview materialization and the
verified atomic swap. **Boundary note (honest gap):** batch inserts
into a table whose Jet index B-tree grew multi-page are refused by the
M15 boundary ("index root page … is not a leaf") — the insert points
therefore run on the index-free table `c` on both sides. Indexed
inserts are exactly what the M15 index-page work has to deliver.

### Compact (`*_compact`; DAO rewrites to a fresh file, DataCraft swaps in place)

| Class | DAO CompactDatabase | DataCraft CLI |
|---|---|---|
| 10k narrow | 20.2 | 93.2 |
| 10k wide | 21.5 | 105.7 |
| 100k narrow | 143.3 | 636.8 |
| 100k wide | 167.8 | 744.9 |
| 1M narrow | 1 621.8 | 10 894.9 |

Read: DataCraft compact pays the full read-everything/write-verified
path plus a process start per run (the CLI number includes startup ≈
7–10 ms and the in-place swap; DAO rewrites to a separate file). The
remaining wide/100k cells are open and filled by a re-run of the same
harness.

### RAM footprint (host process working set after open + one scan)

| Class | DataCraft CLI working set |
|---|---|
| 10k narrow | 8.6 MB |
| 100k | 26.8 MB |
| 1M | 205.1 MB |

The DAO counterpart would be the COM host process (PowerShell +
accessibility of the engine in-proc) — not comparable as a process
number; recorded as an open cell with the caveat.

## Honest gaps (the scoreboard's other half)

1. **M15 index pages:** the writer maintains single-page index roots
   only; multi-page B-trees refuse batch writes. This boundary also
   blocks indexed-insert benchmarking (the insert table is index-free).
2. **Shadow-engine materialization:** filter/join numbers measure the
   documented stage-1 gap (§16), not steady-state intent — the
   windowed-read work owns the improvement.
3. **Family B (UX/task level)** — app start to first row, datasheet
   scrolling against the 60 FPS design target, report preview
   rendering: not measured here; they need the interactive
   methodology from roadmap §18 and stay open.
4. **Compact at scale:** DataCraft compact pays the full
   read-everything/write-verified re-encode — at 1M rows that is
   ~6.7× the engine's compact. That is the price of the verified
   atomic swap; the M15/compact follow-ups own any improvement.
5. **Open growth:** the CLI open cost grows with the file (≈ 2 ms of
   open work at 10k, ≈ 66 ms at 1M) — the health check and facts read
   scale with the file; the windowed-read item owns the fix if it
   matters in practice.
6. **Power plan:** the recorded numbers ran on the Power Saver plan;
   the ranking is expected to hold, the absolutes will move.
7. **Name AutoCorrect:** an Access-application setting, not a DAO
   engine one — not applicable to this harness's engine-level numbers;
   it matters for future Access-app (family B) methodology only.

## Reproducing

```bash
# Build the CLI the harness times (release, once)
cargo build --release -p datacraft-cli

# Environment capture
powershell -NoProfile -ExecutionPolicy Bypass -File tools/dao-oracle/bench.ps1 env

# Generate a class (never committed)
powershell ... bench.ps1 gen -Path C:\temp\bench\n100k.mdb -Count 100000
powershell ... bench.ps1 gen -Path C:\temp\bench\w100k.mdb -Count 100000 -Wide

# One measurement point (median over 10 runs)
powershell ... bench.ps1 dao-scan -Runs 10 -Path C:\temp\bench\n100k.mdb
powershell ... bench.ps1 cli-scan -Runs 10 -Path C:\temp\bench\n100k.mdb
```
