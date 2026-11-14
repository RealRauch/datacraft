# BACKLOG_ARCHIVE.p80.md — Shard (Phase 80, append-only)

> Created 2026-11-14T20:34 — home for item blocks archived in the context of Phase 80 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 80. Header is write-once — never edited.

---

### [x] H60 — Atomic writes survive a crash but not power loss or a concurrent writer — 🟠
- **Location:** `crates/datacraft-core/src/atomic_write.rs:12` (fixed `<name>.tmp` staging path), `:22`/`:36` (`write_replacing`: write + rename without a flush), `:51` (`write_new` without a flush).
- **Problem:** The replace is atomic against crashes but not durable and not concurrency-safe: no `sync_all` before the rename and no parent-directory sync, so after a power loss the rename can be durable while the file's data blocks are not; the fixed temp name means two DataCraft instances writing the same sidecar or database share one staging file; Windows rename does not retry the sharing violations (OS error 32/33) that antivirus scanners and the search indexer cause on freshly written files; a symlinked target is replaced instead of followed; the existing POSIX mode is not restored. craft-libs' `craft-atomic` documents exactly these guarantees (unique `.{stem}.{pid}-{n}` staging, F_FULLFSYNC on Apple, directory sync, bounded rename backoff, symlink resolution, mode preservation, fault injection).
- **Approach:** Harden the existing module instead of adding a dependency: unique staging name, flush the staging file before the rename, best-effort parent-directory sync on Unix, bounded retry on the Windows sharing/lock errors, resolve symlink chains with a hop limit, restore the replaced file's mode. Keep the read-only refusal from F9 unchanged and split the module if it crosses the file-size gate.
- **Acceptance:** A fault-injection seam records create → flush → rename order; two writers never share a staging file; a simulated Windows lock retries and then succeeds; a read-only target still refuses; a crash between staging and rename leaves the target byte-identical and at most one documented orphan staging file; the existing atomic_write, jet write-through and sidecar suites stay green.
- **Steps:** none yet.
- **Done:** done in `e9e6668` (step 80.1) — unique pid+counter staging names, sync_all before the rename plus a best-effort Unix parent-dir sync, a bounded 32/33 rename retry with capped backoff, 16-hop symlink resolution, mode preservation, write_new flush, and the write_replacing_observed/StagingEvent fault-injection seam; modern Windows renames with POSIX semantics, so the sharing-violation path (MoveFileExW fallback volumes) is pinned by the simulated retry contract tests

---

### [x] H59 — CSV import rejects non-UTF-8 and non-comma files — 🟠
- **Location:** `crates/datacraft-core/src/import.rs:52` (`String::from_utf8`), `:85` (`csv::ReaderBuilder` without a delimiter); the CSV driver reads through the same assumption at `crates/datacraft-csv/src/lib.rs:415`.
- **Problem:** Import File and the CSV driver accept UTF-8 with a BOM and the comma delimiter only. A German Excel export (semicolon separator, Windows-1252 or UTF-16 text) fails with "not valid UTF-8", or imports as one column. Access opens the same file. craft-libs' `craft-csv` documents the missing behaviour: BOM UTF-8/UTF-16 → valid UTF-8 → Windows-1252 (Windows-1256 when it reads as Arabic); the delimiter is the one with the most consistent count over the first 20 non-blank lines, outside quotes.
- **Approach:** Decode and sniff in one shared core path used by the import and the driver, then hand the text to the existing L7 inference. Detection is deterministic and reports its choice (encoding, delimiter) in the preview; no silent lossy decode. The strict UTF-8 branch stays first so existing fixtures and suites stay byte-identical.
- **Acceptance:** A cp1252 semicolon file, a UTF-16LE BOM file and a tab-separated file each import with the right field count and text; ragged or invalid input still fails with a locatable reason; the preview names the detected encoding and delimiter; the existing UTF-8 suite is unchanged.
- **Steps:** none yet.
- **Done:** done in `01f0027` (step 80.2) — datacraft-core::import::dialect is the one shared decode path (valid UTF-8 first with/without BOM, then UTF-16LE/BE BOM streams, then Windows-1252 with precise refusals for undefined bytes and broken streams; the cp1256 Arabic variant is a documented remainder) plus a quote-aware consistent-count delimiter sniff (comma/semicolon/tab, comma default on ties); read_csv_file reports the dialect, ImportPreview.dialect carries it to every client, the Import File window renders it localized, and the CSV driver reads through the same path (table_io.rs split off lib.rs 488→366, L30)

## ✅ Done Index

> One-liner per completed item (details above).

- H60 — Atomic writes survive a crash but not power loss or a concurrent writer — done (done in `e9e6668` (step 80.1) — unique pid+counter staging names, sync_all before the rename plus a best-effort Unix parent-dir sync, a bounded 32/33 rename retry with capped backoff, 16-hop symlink resolution, mode preservation, write_new flush, and the write_replacing_observed/StagingEvent fault-injection seam; modern Windows renames with POSIX semantics, so the sharing-violation path (MoveFileExW fallback volumes) is pinned by the simulated retry contract tests)
- H59 — CSV import rejects non-UTF-8 and non-comma files — done (done in `01f0027` (step 80.2) — datacraft-core::import::dialect is the one shared decode path (valid UTF-8 first with/without BOM, then UTF-16LE/BE BOM streams, then Windows-1252 with precise refusals for undefined bytes and broken streams; the cp1256 Arabic variant is a documented remainder) plus a quote-aware consistent-count delimiter sniff (comma/semicolon/tab, comma default on ties); read_csv_file reports the dialect, ImportPreview.dialect carries it to every client, the Import File window renders it localized, and the CSV driver reads through the same path (table_io.rs split off lib.rs 488→366, L30))
