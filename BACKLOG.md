# BACKLOG.md — Open items (As of: 2026-11-03T00:06

> **This file contains only OPEN items.** Completed items are moved to `docs/archive/BACKLOG_ARCHIVE.md` unchanged after completion; this file keeps only a one-liner per item in the Done Index (below). Findings/fix-ideas/decisions are not deleted — moved to the archive.
> Legend: 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · 🔵 test gap
> Working order: sequential by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), each item test-first, commit per item.

> **Model references** (`model:` lines) point at `storytold/wordcraft` @ `fd7f662` (2026-11-01); the named paths and values are verified against that revision.

---

## 🔴 CRITICAL

> No open items.

---

## 🟠 HIGH

> No open items.

---

## 🟡 MEDIUM

> No open items.

### [ ] M13 — Release-Engineering (Tags, release.yml, Checksums, Smoke) — 🟡
- **Location:** `.github/workflows/release.yml` (new), `Cargo.toml` (version policy), `docs/PLAYBOOK.md` (release section), Third-Party-Notices
- **Approach:** Tag-triggered release workflow (mirror of the CI gates): build `datacraft`, `datacraft-cli` and `datacraft-mcp` for Windows/macOS/Linux (rusqlite statically bundled), archives with sha256 checksums + third-party notices. Post-Phase-30 additions (ADR-7): the composed binaries bundle the CSV driver through the composite fallback, so the per-artifact smoke also opens a throwaway CSV directory headlessly (the driver set is proven inside the artifact), and the notices name the `csv` crate.
- **Acceptance:** The workflow file lints (YAML/Actions conventions like ci.yml); the local release smoke opens `testV2000.mdb` headless *and* a CSV directory; the tag policy is documented; no binary lives in the repo.

### [ ] M15 — Index creation in the Jet writer — 🟡
- **Location:** `crates/datacraft-storage/src/jet/encode.rs` (`num_idx`/`num_real_idx`), `crates/datacraft-storage/src/jet/writer_tables.rs`, `crates/datacraft-storage/src/port_write.rs`, `crates/datacraft-storage/src/sqlite/mapping.rs`
- **Problem:** `encode_tdef` writes zero index blocks; `FieldProperties.indexed` is serialization/UI metadata only. Generated tables and samples cannot demonstrate indexed lookups or uniqueness, and a template's `Indexed` flag does nothing.
- **Approach:** Encode TDEF index descriptors and index pages for `IndexMode::{DuplicatesOk, NoDuplicates}`, maintain them on insert/update/delete, and emit `CREATE INDEX`/`UNIQUE` for SQLite. Keep existing index blocks readable and untouched on unrelated writes.
- **Acceptance:** Indexes survive reopen and are visible in Access; uniqueness is enforced with a precise error; SQLite emits the matching DDL; tests cover create/insert/update/delete and the unsupported shapes.

### [ ] M16 — Relationship and foreign-key creation — 🟡
- **Location:** Jet writer (`MSysRelationships` rows and the required indexes), `crates/datacraft-storage/src/sqlite/mapping.rs` (foreign-key DDL), `crates/datacraft-core/src/port.rs` (create/delete relationship surface)
- **Problem:** Relationships are read-only on both backends. Generated samples cannot populate the relationship mapper, and a template cannot express referential integrity.
- **Approach:** Add a create/delete relationship path through the port and a matching bus command. Jet writes `MSysRelationships` plus the required primary/foreign indexes; SQLite emits `FOREIGN KEY` constraints. Reads must round-trip through the existing `relationships()` API. Post-Phase-30 (ADR-7): landing this flips the jet and sqlite drivers' `relationships` capability from `ReadOnly` to `Yes` — the availability map (the disabled Relationships affordance on CSV stays) and the mapper affordances update automatically; the flip is pinned in the driver capability invariants.
- **Acceptance:** Created relationships survive reopen and appear in the mapper; Access shows them; SQLite enforces the constraint; the capability flip is pinned (`ReadOnly` → `Yes` for jet/sqlite, CSV stays `No`); tests cover create/delete, duplicates and unsupported shapes; canonical gates green.

### [ ] M19 — Engine/driver publishing: crate split, standalone MCP binary, Semver policy (Phase 31) — 🟡
- **Location:** `crates/datacraft-storage/` (split into `datacraft-jet` + `datacraft-sqlite`), `crates/datacraft-mcp/` (new binary crate), workspace `Cargo.toml`, packaging documentation
- **Problem:** The engine (`datacraft-core`) and all drivers share one workspace with lockstep versions; external consumers and contributors cannot depend on stable published crates, and the pure-Rust Jet story is diluted by the C-backed `rusqlite` dependency in the same crate.
- **Approach:** split the storage crate into per-driver crates (Jet stays pure Rust with its oracle suite; SQLite moves its dependency into its own crate), extract `datacraft-mcp` as its own binary crate, then publish in dependency order (engine → jet → sqlite → csv → mcp) with a Semver/MSRV/changelog policy. Details are packaged when Phase 31 starts (ADR-7 records the decision).
- **Acceptance:** each crate builds and publishes independently; the workspace still builds as one unit; the publishing checklist and versioning policy are documented; no behavior change.

### [ ] M21 — Retire the vestigial StorageEngine trait (ADR-7 single seam) — 🟡
- **Location:** `crates/datacraft-storage/src/engine.rs`, `crates/datacraft-storage/src/mock.rs` (`InMemoryEngine`), `crates/datacraft-storage/tests/engine_boundary.rs`; `docs/ARCHITECTURE.md` §6 (already annotated as the historical record)
- **Problem:** ADR-7 made the core-owned `StoragePort` the single driver seam; the Phase-2 `StorageEngine` trait survives with one mock implementor and pins an outdated contract (Jet3-unaware, no capabilities, no create/drop/alter surface) right next to the real one — two seams confuse contributors and the public library story (M19).
- **Approach:** Port the still-valuable contract assertions of `engine_boundary.rs` onto the port layer (they fold into the existing driver-capability invariants), then remove the trait, the mock and the boundary suite. NOTE: deleting an existing test suite is a **High-class gate** (PLAYBOOK §0.5) — package with an explicit human release; coverage must not drop (the ported assertions keep the lines covered).
- **Acceptance:** One seam only (`StoragePort`); the boundary contract lives in port-level invariants; coverage ≥ 95 % holds; ARCHITECTURE §6 cites the removal; canonical verification set green.

### [ ] R1 — Cross-cutting code review round (Phase 34) — 🟡
- **Location:** whole workspace (review only — findings become BACKLOG items; fixes only via the inline-fix lane or packaged follow-ups)
- **Problem:** Thirty-plus phases of feature work have never had a deliberate cross-cutting review pass; the per-phase gates verify the canonical set, but nobody has walked the LESSONS checklist across module boundaries since the driver model, the capability vocabulary and the CSV driver landed.
- **Approach:** One review phase before publishing (Phase 34), ordered per `docs/LESSONS.md`: (1) storage & drivers — security self-review (LESSONS #7: crafted `.mdb`/`.accdb`/`.csv` against the parsers), error precision and locatability (LESSONS #4), clean-room spot check (LESSONS #12); (2) core — dispatch/journal/capability-gate consistency, event/snapshot shape stability; (3) UI — paint-code discipline (no logic in paint), i18n parity, affordance coverage; (4) public API surface audit for Phase 31 (every `pub` item that would be published: needed, documented, semver-sane). Findings are triaged in-phase: Low-class ≤10-liners via the inline-fix lane (with F-retrospectives), everything else packaged.
- **Acceptance:** Every LESSONS checklist item walked with a written verdict; findings triaged (fixed inline or BACKLOG items with locations + acceptance criteria); no HIGH finding left unassigned; canonical verification set green.

### [ ] R2 — Documentation sweep before the release (Phase 35) — 🟡
- **Location:** `AGENTS.md` (§1 workspace list + dependency diagram lack `datacraft-csv`), `README.md` (architecture table + diagram), `docs/ARCHITECTURE.md` (§5 command-enum preview is the Phase-4 target shape, §6 "remaining write-path work" still lists M7/M8/ACE-write although done, §13 phase-mapping table ends at Phase 9), `docs/FORMAT-JET.md` (no CSV-driver note), `docs/TESTING.md` (taxonomy vs. capability-invariant/kittest suites)
- **Problem:** The documentation has drifted across phases 10-30: the entry points an agent or contributor reads first (AGENTS.md, README architecture) describe a four-crate workspace, ARCHITECTURE carries stale milestone and mapping sections, and the driver model (ADR-7) is documented in the ADR but not in the navigation docs. A release must not ship with entry-point docs that contradict the repository.
- **Approach:** One sweep phase right before the release (Phase 35), after Phase 31 freezes the crate structure: (1) AGENTS.md + README architecture/diagrams reflect the real crate graph; (2) ARCHITECTURE §5/§6/§13 rewritten to the current truth (preview sections replaced or explicitly dated); (3) FORMAT-JET gains the CSV driver section, TESTING taxonomy matches the live suites; README compatibility matrix re-verified date-stamped. Append-only archives stay untouched; every claimed number is checked against the code.
- **Acceptance:** No tracked doc contradicts the repository state (crate lists, phase mappings, remaining-work claims); the compatibility matrix is date-accurate; a fresh-contributor walkthrough (clone → read AGENTS/README → build → open a CSV directory) works without hitting stale guidance; canonical verification set green.

### [ ] M22 — Publish-ready API vocabulary: non_exhaustive and the capabilities default (Phase 31 prep) — 🟡
- **Location:** `crates/datacraft-core/src/port.rs` (`PortError`), `crates/datacraft-core/src/port/capabilities.rs` (`Support`, `DriverCapabilities`), `crates/datacraft-core/src/command.rs` (`Command`), semver policy of step 31.4
- **Problem:** The vocabulary enums are closed: every new `Support`/`PortError`/`Command` variant is a breaking change for third-party drivers compiled against the published engine. Worse, the trait default `DriverCapabilities::full()` makes a driver that forgets the override *lie* — it reports every capability as available and the dispatcher then lets commands through that the port refuses at call time. Both surface only after crates.io publishing (M19), when they become semver pain instead of a cheap pre-split decision.
- **Approach:** Before the crate split (31.2): mark the wire-facing enums `#[non_exhaustive]` (`Support`, `PortError`; `#[non_exhaustive]` on the `DriverCapabilities` struct for its fields), decide the default question explicitly — conservative default (a forgetful driver declares nothing) versus making `capabilities()` required — pin the decision in ADR-7 and the 31.4 Semver policy, and add a meta-test that every registered shipped driver overrides `driver_id`/`capabilities` explicitly.
- **Acceptance:** Adding a vocabulary variant does not break a compiled out-of-tree driver (proven by `datacraft-csv` compiling unchanged against the extended core); the default story is decided, documented and meta-tested; canonical verification set green.

### [ ] M23 — Registered drivers visible in the snapshot — 🟡
- **Location:** `crates/datacraft-core/src/snapshot.rs` (`UiStateSnapshot`), `crates/datacraft-storage/src/detecting.rs` (registry introspection)
- **Problem:** Agents can discover templates in the snapshot, but not *which* drivers the composed binary can open — they learn a source family only by attempting to open it and reading the failure. The registry exists (built-in probes plus `with_fallback`); only the projection is missing.
- **Approach:** The composite exposes its driver set — driver id, display format, offline capability summary — for the built-ins and any registered fallbacks; the snapshot carries it additively next to `templates`. No UI surface is mandated; CLI/MCP agents read it over the wire and can pre-flight an open against the reported families.
- **Acceptance:** `inspect()` on a composed binary lists jet, sqlite and csv with ids and formats; the wire change is additive; the driver capability invariants are unaffected; canonical verification set green.

---

## 🟢 LOW

> No open items.

### [ ] I6 — Failure-reason localization (post-V1) — 🟢
- **Location:** `crates/datacraft-core/src/core.rs` + dispatch modules (stable failure codes first), `crates/datacraft-ui/src/i18n/` (UI-side mapping); ADR-4
- **Problem:** `Event::Failed.reason` strings are English on the wire (~120 sites plus the Phase-30 capability rejections, e.g. `unsupported: the {driver} driver cannot write rows for this source`); localizing them inside core would put language into core (ADR-4 violation), so V1 keeps them English — agents key on data, not prose.
- **Approach:** Two steps, post-V1: (1) core gains stable machine-readable failure codes (enum or code prefix) alongside the English text — a domain concern, not a setting; the capability-rejection reasons map onto the `DisabledReason` vocabulary where they overlap; (2) the UI maps known codes to localized text via its catalog; unknown reasons render in English unchanged.
- **Acceptance:** Every dispatch failure carries a stable code (including the capability gates); the UI renders localized text for known codes and English otherwise; the wire gains codes without breaking existing consumers; English-substring tests (command_errors.rs, cli_contract.rs, server_e2e.rs, driver_caps.rs) switch to code assertions.

### [ ] L6 — Ship samples next to the binary and in release archives — 🟢
- **Location:** release packaging (Phase 26, M13), `crates/datacraft-ui/src/samples.rs` (runtime resolution)
- **Problem:** The backstage Samples page resolves `samples/` from `DATACRAFT_SAMPLES_DIR`, the executable directory, or the dev workspace. Installed builds have no installer step that places the folder next to the binary, so the page falls back to a path that does not exist.
- **Approach:** Copy `samples/` into every release archive next to `datacraft`/`datacraft-cli`/`datacraft-mcp`, document the resolution order, and add a smoke check that an extracted archive finds and opens the demo.
- **Acceptance:** Release archives contain `samples/`; the backstage page resolves them without repo paths; a packaged smoke test opens the demo; no binary or sample is committed into the repo twice.

### [ ] L7 — CSV type inference beyond Short Text — 🟢
- **Location:** `crates/datacraft-csv/src/lib.rs` (`collect_tables`)
- **Problem:** Every CSV column is Short Text (255) — deterministic and honest, but numeric/date columns read and sort as text, and the datasheet parses accordingly.
- **Approach:** Conservative, deterministic inference per column over a full scan (integer → LongInteger, float → Double, ISO date → DateTime; anything else stays Short Text), decided at open and stable across opens; quoted values stay text (CSV semantics — quoting is the user's escape hatch); the rule is documented in the driver docs and pinned by tests; ambiguous columns never guess.
- **Acceptance:** Inference is deterministic and documented; quoted numbers remain text; tests pin each inferred kind and the Short-Text fallback; canonical verification set green.

### [ ] L8 — Server sources: SourceSpec for the open commands (ADR-7 future path) — 🟢
- **Location:** `crates/datacraft-core/src/command.rs` (`OpenDatabase`/`CreateDatabase`), `state.rs`/`snapshot.rs` (path → source), UI backstage, sidecar path derivation (ADR-1)
- **Problem:** The driver model is ready for server drivers (Postgres/SQL Server per ADR-7's documented future path), but the wire and the UI assume a filesystem path (`path: PathBuf`, file-stem names, a sidecar next to the file).
- **Approach:** Additive `SourceSpec { Path, Uri }` on the open commands (the existing field keeps working for file sources), the snapshot gains a source display, the sidecar applies to `Path` sources only; the first actual server driver decides credentials/SSL and stays synchronous behind the no-async rule (ADR-7 records the constraint honestly — OCI is C-only, pure-Rust TDS/Postgres clients are async-first).
- **Acceptance:** A URI source opens through all four clients; file behavior is byte-identical; ADR-7 gains the extension note when the first server driver is planned; canonical verification set green.

### [ ] L9 — Composite fallback keeps the unknown-file format hint — 🟢
- **Location:** `crates/datacraft-storage/src/detecting.rs` (`open`), `crates/datacraft-csv/src/lib.rs` (`open`)
- **Problem:** With a registered fallback (the composed binaries), an unrecognized *file* now answers the fallback's "not a CSV directory — this driver opens directories of *.csv tables" instead of the sniff's precise "expected a Jet/ACE header (01 00 00 00) or a SQLite header" reason. The hint regressed exactly for the most common user mistake — opening a random file (LESSONS #4).
- **Approach:** Chain the reasons: the composite attaches the sniff hint when the fallback rejects (or the CSV driver's rejection text names the expected families) — one precise, user-renderable message either way, pinned by a composed-port test.
- **Acceptance:** An unknown file through a fallback-composed port names both the path and the expected source families; unknown paths without a fallback behave unchanged; a test pins the message; canonical verification set green.

---

## 🔵 TEST GAPS

> No open items.

### [ ] T6 — Semantic-Conformance-Hinweise fürs Code-Review (Phase 34) — 🔵
- **Location:** Review-Ziele: `crates/datacraft-core/src/expr/` (coerce/funcs), `query_engine.rs`, `sql_engine.rs` + Shadow-SQLite-Materialisierung; Referenz-Oberflächen: Expression-Service, QBE-Engine, SQL-Fenster
- **Hinweis 1 — Currency/Präzision:** Shadow-SQLite hält Currency als REAL → `SUM`/`AVG` über Money rundet binär statt auf 4 Dezimalstellen. Spot-Check: SUM über 3 Cent-Beträge, Multiplikation Geld×Zahl.
- **Hinweis 2 — Kollation:** SQLite `BINARY` ≠ Access kulturabhängig (ä/ö/ü/ß, case-insensitiv) in ORDER BY/WHERE/Vergleichen. Spot-Check: Sortiere [Ärger, Ass, Berg], Filter `Name = 'müller'` case-insensitiv.
- **Hinweis 3 — Datum:** SQLite hat keinen DATE-Typ → Literale `#…#`, `Datum + 1`, Komponenten-/Format-Funktionen über Text-Realität. Spot-Check: Datum+1, Jahresfilter, Format „Langes Datum".
- **Hinweis 4 — LIKE-Wildcards:** Access `* ? # [..]` vs SQL `% _` im SQL-Fenster — Übersetzung oder dokumentierte Einzäunung. Spot-Check: `LIKE 'B*'` in QBE und im SQL-Fenster.
- **Hinweis 5 — Promotion/Overflow:** Int-Overflow, Mixed-Type-Promotion-Matrix (Tiefe von `coerce.rs`). Spot-Check: 32767+1, Int×Int, Text+Zahl.
- **Hinweis 6 — NULL-Kanten:** `NOT IN` mit NULL, `Nz()`-Rückgabetypen, IIf-Zweig-Auswertung. Spot-Check: je ein Fall pro Kante.
- **Hinweis 7 — Multivalue-Prädikate im SQL-Pfad:** Attachment-/Multivalue-Felder in SQL → mögliche stille Leerergebnisse. Spot-Check: WHERE über Multivalue-Spalte.
- **Acceptance:** Jeder Cluster mit 3–5 Vektoren gegen alle drei Oberflächen gespielt; Divergenzen werden Findings (→ 34.5 Triage) oder bewusst eingezäunt/dokumentiert; Ergebnis landet als Folge-Items bzw. in TESTING/FORMAT-JET-Doku.

---

## ✅ DONE INDEX

> (One line per completed item, with commit hash; details in archive.)
- K1 — Initialize git repository with GitHub Actions quality gates — done (done in `db6e6d5` — workflow with fmt/clippy/test-matrix/coverage/file-size jobs; cargo jobs guarded until workspace; amended in 6e2a3df (foreign .tmp-brand files unstaged))
- K2 — Author architecture blueprint (docs/ARCHITECTURE.md) — done (done in `f28165d` — docs/ARCHITECTURE.md: principles→mechanisms, crate graph, command API, storage decision record, phase mapping)
- H1 — Create Cargo workspace and dependency manifest — done (done in `68aab5f` + `930a289` — workspace with core/storage/ui crates, edition 2024, deps via cargo add (eframe 0.37.0), committed Cargo.lock)
- H2 — Coverage tooling and exception policy (docs/TESTING.md) — done (done in `16e6c4d` — docs/TESTING.md with exception registry #1 (datacraft-ui crate exclusion; coverage attribute unstable on stable), ci.yml coverage job updated)
- H3 — Toolchain pinning and lint configuration — done (done in `6e2a3df` — rust-toolchain.toml pinned to 1.98.1, MSRV 1.98 in workspace.package, shared lints (unsafe_code forbid, clippy all))
- K3 — Repair invalid CI workflow (hashFiles in job-level if) — done (done in the `fix(ci)` commit — preflight job pattern replaces invalid hashFiles-in-job-if; first valid run 41242950916 in progress)
- T1 — Boundary coverage below gate — error display arms and mock write-path validations untested — done (done in `2084be0` — contract+colocated tests close gap; coverage 81.92% -> 98.17%; found by CI gate (run 41242950916), lesson: canonical verification set runs per step)
- M1 — Decide persistence format of the form/report design layer — done (decided 2026-10-31 via ADR-1 in docs/ARCHITECTURE.md §7 (human offline, default ratified by blanket phase 3-9 release): versioned .datacraft sidecar (serde JSON) for phases 5-8; native accdb embedding only after ACE writer)
- T2 — Real-row tail semantics — mask bytes and var_table entries disagree with synthetic model — done (commit: null-mask bit index = column id (row.rs); oracle green on both fixture writers; CI oracle job added)
- H4 — Define StorageEngine trait boundary — done (steps 2.1 (test 4a507ec, feat 6b7dbc8))
- H5 — Pure-Rust Jet page layer and file header decoder — done (steps 2.2 (test 442f1c9, feat 984006f))
- H6 — Pure-Rust table definition and data decoder — done (steps 2.3 (test b2c8f63, feat 367a07b))
- H7 — libmdb test oracle integration and golden-file suite — done (steps 2.4 + T2 fixes (a200dee, 92bce57, 683a88f); CI oracle job added; T3 keeps fixture-pin follow-up open)
- H11 — Versioned schema sidecar document (serde persistence) — done (steps 3.2 (test 5432ef9, feat a72c5e6))
- H10 — Access data types and field properties — core domain model — done (step 3.1 (test 7cf9a31, feat 1271f33))
- H13 — Command dispatch core — commands, events, state, storage port — done (step 4.1 (test baa4f16, feat f796480))
- H14 — State query API and deterministic test IDs — done (step 4.2 (test da774ad, feat 1b4a9be))
- H15 — JSON-RPC transport codec stub (headless seam) — done (step 4.3 (test ceecb24, feat a0c5137))
- H17 — App shell and layout — four-region egui shell over the core — done (step 5.1 (test 5ed94ed, feat 666acd7))
- H12 — GUID field kind — plain Replication ID columns unmodelable — done (decided: dedicated FieldKind::Guid; implemented with H16 (tests f4c96d3/e1bb1bb, adapter 67a3b64); decision documented in docs/FORMAT-JET.md "Domain mapping")
- H16 — StoragePort adapter — Jet4 backend implements the core port — done (step within Phase 5 (tests 96bc864/aacc8ab, feat 67a3b64); includes the multi-page TDEF decoder extension; seam tests on both sides per LESSONS #2)
- H18 — Ribbon bar — spec-driven Access-style ribbon — done (step 5.2 (test 483bca0, feat 14ec049))
- H19 — Navigation pane — snapshot-driven object list — done (step 5.3 (feat a706624))
- H20 — Tabbed document workspace — strip and placeholders — done (step 5.4 (test 4a3a280, feat 015459d))
- H22 — Shared grid core — virtual-scroll and hit-test math — done (step 6.1 (test 62820e4, feat c256a10))
- H21 — Compose the real Jet4Port in the UI binary (composition root) — done (step 6.2 (feat 8410069): App::new takes the port; main.rs composes Jet4Port as the documented composition root)
- H23 — Datasheet view — port read_rows, row mapping, real composition — done (step 6.2 (tests 704fc10/e606a07/2a7d3be, feats 8b97c26/8410069))
- H24 — Table design view — read-only design grid and kind display names — done (step 6.3 (core 3e6fe9a, ui 06c2982); design-time schema editing deferred to H9 (documented in the UI))
- H25 — Relationship mapper pane — port relationships and canvas — done (step 7.1 (feat a7e2a6f))
- H26 — QBE grid — query model, commands, sidecar persistence — done (step 7.2 (feat 7cb66ba))
- H27 — Query engine integration — predicates, execution, result view — done (step 7.3 (feat ec62b7b))
- H28 — WYSIWYG canvas — snap-to-grid math and form skeleton — done (step 8.1 (feat 414499e))
- H29 — Access control types — palette, placement commands, drag — done (step 8.2 (feat e959928))
- H30 — Form runtime — data binding, record navigation, sidecar file IO — done (step 8.3 (feat 7a189b8); report canvas deferred as H31)
- H32 — MCP stdio endpoint — serve loop and datacraft-mcp binary — done (step 9.1 (feat 617e259))
- H33 — Headless E2E harness — full stack over the wire — done (step 9.2 (test 245b49a))
- H8 — CLI client as fourth headless command-bus client — done (step 9.4 (test 917174f, feat d31c759))
- F1 — Ribbon spec/mapping seam was unguarded — Create tab panic — done (fixed immediately: total command_for mapping + seam test in ui crate (next commit); no unreachable! left in paint code)
- H9 — Jet4 writer workstream (create tables, write rows) — done (Phase 10, steps 10.1-10.3 (checkpoint 73f540b): writer core + create/insert + round-trip acceptance; Access-repair gate documented as manual follow-up)
- H31 — Report canvas — print layout and preview — done (Phase 10, steps 10.4-10.5 (checkpoint 1440c64): print-geometry designer + preview runtime; grouped/aggregate bands deferred to M1)
- H34 — Final documentation and release prep — done (Phase 10, step 10.9 (checkpoint 9468b51): README/ROADMAP/ARCHITECTURE/FORMAT-JET reflect the Phase 1-10 feature set)
- L1 — Jet 3 (Access 97) read support — done (Phase 10, step 10.6 (checkpoint e1627cb): Jet3 read through the same boundary; jump-table rows = documented Unsupported remainder; real A97 fixture unpinned)
- L2 — Jet4 column type 0x11 (MSysAccessObjects.Data) — study and decide mapping — done (Phase 10, step 10.7 (checkpoint 8faeac3): 0x11 = JetType::LongBinary (fixed OLE compound docs); oracle skip documented)
- T3 — Fixture pin integrity — restore pristine testV2000, keep Access-touched variant, manifest + CI hash gate — done (Phase 10, step 10.8 (checkpoint aafb794): pristine restored (126bad49…), touched variant kept, manifest + CI hash gate, immutability rule in TESTING.md)
- H35 — SQL-Integration: SQLite-Datenquelle (Backend, Detection, Format) — done (Phase 13.1 — SqlitePort (catalog, affinity schema, rows, FKs), DetectingPort magic sniff, DatabaseFormat::Sqlite; tests ca61638, feat 5a04e94)
- H36 — SQL-Integration: SQLite-Writes + Row-Insert im Command-Bus — done (Phase 13.2 — StoragePort::insert_row, Command::InsertRow/Event::RowInserted with unit validation, SqlitePort create/drop/insert; tests 99ddb0b, feat 390c79c)
- H37 — SQL-Integration: SQL über alle Quellen (In-Memory-Engine + UI) — done (Phase 13.3 — shadow SQL engine (one SELECT/WITH over materialized sources), RunSql/SqlRan, SQL window UI; tests 1a58292, feat 0400f63)
- H38 — ACCDB-Fixtures pinnen (ACE 2007/2010/2016, manifest + Hash-Gate) — done (Phase 14.1 — pristine ACE 2007/2010/2016 fixtures pinned, manifest + hash gate, provenance in FIXTURE-SOURCES; commit aef9b8f)
- H39 — ACCDB-Read-Pfad (Catalog/TDEF/TDAT über den Jet4-4096-Pfad) — done (Phase 14.2 — ACE read path through Jet4 4096; oracle match (mdbtools 1.0.1), format through all clients, precise header errors; commit d6a3a66)
- H40 — Jet4 Update/Delete-Writes (RowOp am echten Dateipfad) — done (done in `ea76d2d` (step 11.10; in-place/repack updates, 0x8000 deletes, precise Unsupported for overflow/LVAL/oversized rows, port + bus commands))
- M2 — Journaled/restringierte Write-Profile (Preview, Bestätigung, Feldfamilien) — done (done in `dcb6852` (step 11.11; preview→apply→rollback journal, atomic Jet batch, field-family profile))
- H41 — Expression-Service (Ring 1 / Roadmap M1) — done (15.1 (b6c7c73 RED, a657762 GREEN) + 15.2 (7eb3bbc RED, 6cb634d GREEN): evaluator suite, NULL/coercion contracts, defaults/rules/calculated wiring, RunExpression over the bus and grouped/aggregate report bands)
- M3 — Native Saved Queries (MSysQueries → QueryDef) — done (15.3 (78203c3 RED, d914484 GREEN): MSysQueries decoder with all nine flavors, native queries in the nav pane, read-only QBE tab, mappable-subset Run, documented rests (parameters/criteria/joins/grouping), queryTestV2000.mdb pinned (51bd3c5))
- F2 — Jet3 database passwords are plain text (M5 fact correction) — done (done in 9b05ab7)
- M4 — Complex Values/Multivalue (Attachment-Subtabellen real decodieren) — done (done in 6d96561 (RED 5480122))
- M5 — Verschlüsselung/Passwort (RC4-Decode, Passwortpfad UI/CLI/MCP) — done (done in fd30e5a (RED 3a3bb15; Jet3 password fact fix 9b05ab7))
- L3 — Jet3-Reste (Jump-Table-Zeilen >255 Bytes, verkettete TDEFs) — done (done in a8df705 (RED bd8663a))
- M6 — Doku-Genauigkeit (Kompatibilitätsaussagen ↔ verifizierter Stand) — done (verified compatibility matrix + Phase 17 docs accuracy, commit 19854e6)
- L4 — Makro-Inventur (Ring 3 / Roadmap M5) — done (read-only macro inventory, commit 8da3acb)
- L5 — VBA-Extraction + Viewer + statische Analyse (Roadmap M6) — done (VBA extraction/viewer (ccf2107) + static analysis (63ec95c), no execution)
- I1 — i18n kernel in the UI crate — done (dbc513a)
- I2 — All UI literals through t() — done (b616e9c)
- I3 — German catalog complete — done (6486786)
- I4 — App settings store (ui.json) — done (d898120)
- I5 — Language switcher in the UI — done (745f84f)
- U1 — Theme rebase: WordCraft-style neutral tokens, Teal accent — done (done in `c9ea70e` (RED `849c428`) — neutral `Tokens`/`GEOMETRY`, teal accent derivation pinned by tests, all paint sites rebased; before/after screenshots stay with the Phase 20 gate (20.5))
- U2 — Typography: Inter + type rungs — done (done in `aef71e9` (RED `e8a6c6c`) — Inter 4.1 and JetBrains Mono 2.304 embedded unmodified with SIL OFL licenses and attribution; rung scale installed in both themes; German layout budgets now measured on the shipped fonts)
- U3 — Original vector iconography (ribbon, nav pane, tabs) — done (done in `824f546` (RED `0c2ca4e`) — one name-keyed painter dispatcher in `icons.rs` with `file`/`data`/`objects` section painters; every ribbon action id and object kind is dispatched and paints; ribbon buttons, nav-pane rows and document tabs carry icons; AccessKit labels preserved)
- U4 — Dark mode: Appearance Light/Dark/System — done (done in `0866cd9` (RED `731155a`) — Appearance Light/Dark/System (default System) persisted in `ui.json`, both palettes installed per theme, tokens picked per frame, dark teal pendant #3FA799 with WCAG contrast tests for both palettes; Linux live-follow polls the XDG portal every 4 s via `gdbus` with a tested parser)
- F3 — F3 — Empty workspace never painted the canvas token — done (fixed inline in `612b114` during the Phase 20 gate — the empty workspace now paints the canvas token in both palettes)
- U13 — ui.json hardening: atomic save + full window geometry — done (done in `1462e99` (atomic save, RED `5c2be6b`) and `21044b2` (geometry + rescue, RED `aabd453`) — sibling temp file renamed over the target, legacy files still load, position/maximized round-trip, off-screen positions rescued onto the visible monitor; runtime smoke test: (9000,9000) → (820,380))
- U5 — Contextual ribbon tabs + „Views" split-button — done (done in `de70a21` (RED `904cc7c`) — contextual tabs (Table/Query/Form/Report Tools) spec-driven in core after Database Tools; `SetTabMode` and `TabSnapshot.mode`/`has_design_view` move the view mode into core state; the Home Views group renders the split-button and the tab-strip toggle is removed)
- U6 — Quick Access Toolbar — done (done in `0050cf2` (RED `5900968`) — QAT strip from core `quick_access` entries above the ribbon; Save dispatches through the shared `command_for` mapping, Undo/Redo stay disabled with a tooltip naming follow-up U14; undo/redo icons and German labels added; UI parity scanner now walks submodules)
- U7 — File backstage page — done (done in `9041874` (RED `3e12e06`) — full-page backstage with Home/Open/Save/Info/Options; open/close/new-table/sidecar flows dispatch their existing commands, the path fields left the ribbon, and the language/appearance switchers moved to Options; Recent Databases deferred to follow-up U15)
- U8 — Datasheet UX: column resize, header sort, row shading — done (done in `78e496b` (RED `165539c`) — drag-resizable columns with core hit-test/min-width/padding math, header-click and context-menu sorting through one SetQbeCell per action, alternating row shading plus the accent cursor marker; widths are per-session App state shared by the new-record row and the SQL window)
- U9 — Nav pane: category dropdown, context menu, group sort — done (done in `05dd600` (RED `6371c93`) — localized single-entry category dropdown, Open/Design/Delete context menu replacing the always-visible ✕, and name/type group sorting via the pure `nav::sort_entries`; Design opens in one command (`OpenObjectInMode`) with wire coverage)
- U10 — Tabs: middle-click close, context menu — done (done in `f2ae2a1` (RED `7e3c5f8`) — middle-click closes a tab; Close / Close Others / Close All context menu with the latter two as bus commands (CloseOtherTabs/CloseAllTabs) and JSON-RPC wire tests; closed tabs drop their presentation caches)
- U11 — Window title & status-bar conventions — done (done in `f049073` (RED `93c1760`) — `<db> : Database (<format>)` viewport title localized in both languages, sent only on change and cleared to the app name on close; status bar gains a disabled filter placeholder)
- U12 — Keyboard: shortcuts & ribbon key tips — done (done in `33f6e23` (RED `e108ae7`) — Ctrl+S/Ctrl+W/Esc dispatch/bind the same single actions as their buttons; Alt shows spec-derived key tips (Tabs phase selects, Commands phase badges the actions); derivation (`core::keys::key_tips`) and shortcut mapping are unit-tested)
- T4 — CLI-Bin-E2E über echten Subprozess (assert_cmd) — done (done in `c382d2f` (RED `ec77a8d`) — real-subprocess E2E with assert_cmd/assert_fs/predicates covers open/exec/non-interactive repl, the 0/1/2 exit-code matrix and stderr precision; binary routes failures/usage to stderr; TESTING.md registry #2 re-evaluated)
- F4 — Datasheet/design looked tables up by the wrong test id — done (fixed inline in `0e86d46` during step 23.2 — datasheet/design now resolve tables by name (case-insensitive) instead of comparing incompatible test-id schemes)
- T5 — egui/eframe headless UI-Tests (egui_kittest, Layout-Regressionen) — done (done in `0e86d46` (RED `3831560`) — datacraft-ui is a lib + thin bin; egui_kittest 0.37 drives ribbon/nav-pane/datasheet/SQL window/backstage via AccessKit, with committed insta semantic-tree snapshots (no pixel compares); TESTING.md registry #1 and taxonomy updated)
- M10 — Native-query decode errors are dropped at open — done (done in `3c7b1fb` (RED `5fb6f96`) — native-query decode errors are stored as `native_queries_error` in the snapshot instead of silently yielding an empty list; the Northwind attribute-12 error is pinned at the port and rendered at the Queries group)
- M8 — Jet AutoNumber counter maintenance on insert — done (done in `af07979` (RED `39e95ad`) — Jet4 writer assigns LongInteger AutoNumbers (Null = engine-assigned), persists the TDEF `+12` counter across reopen and rejects user values; GUID flavor stays Unsupported; mdb-export oracle test added)
- M7 — Jet table drops through the writer (DropTable on .mdb/.accdb) — done (done in `24f5033` (RED `e3c4601`) — writer/drop path deletes the catalog row, frees used-map + TDEF-chain + unreferenced usage pages into the global free map and 0xFF-fills freed pages; port tests cover reopen visibility, neighbour survival and failure atomicity)
- M9 — Design-time schema editing (ALTER TABLE) in the table design view — done (done in `6780b3c` (RED `588fe4f`) — AddField/DropField/RenameField/SetFieldProperties via one `alter_table` port method; TDEF rewrite on a scratch writer with row re-encoding (dense column ids on drop), design-grid edit controls dispatch one command each)
- H42 — ACCDB-Write-Verträge (create/insert/update/delete/drop/alter auf .accdb) — done (done in `7e7f755` (RED `3b4a4fb`) — ACE 2007/2010 write contracts green (insert/update/delete/create/drop/alter with byte preservation and map consistency, 2016 fixture as read-only reference); office-encrypted ACCDB now rejected at the writer boundary)
- H43 — ACCDB-Port-Write-Through + Bus-E2E an .accdb — done (done in `d53c054` — ACE write-through proven at the port (create/insert/update/delete/alter/drop on testV2010 copies, reopen persistence, format stays Ace), over the JSON-RPC wire (server_e2e_ace.rs) and through the CLI vocabulary; no format gate existed to open (pinned fact))
- H44 — Ship sample databases and templates (Phase 29) — done (done in `34dbba7` (create `35b2895`, templates `c09d192`, samples `ea98127`, auto-load/backstage `34dbba7`) — CreateDatabase (Jet4 + SQLite), CreateDatabaseFromTemplate over the deterministic core registry, six shipped samples with byte-compare generator check and manifest, sidecar auto-load with sidecar_error, backstage New/Samples pages; the manual Access-repair gate of the shipped samples is a documented follow-up (README matrix))
- M17 — Driver model: declarative capabilities, registry and core-computed affordances (Phase 30) — done (done in `2d73dae` (RED `a30037c`/`42296a4`; jet `14c6702`+`57d754f`, sqlite `be1a8d2`+`f080b4a`, registry `909c5ca`+`626099c`, rendering pin `c5427ab`) — DriverId + per-session DriverCapabilities/Support on StoragePort, snapshot carries driver/capabilities/availability, early dispatch rejection, registry-driven composite with with_fallback, capability-disabled UI affordances (ribbon/nav-pane/datasheet) with German reasons and kittest pins)
- M18 — CSV directory driver as the out-of-tree proof (Phase 30) — done (done in `5cee066` (RED `0a86718`; gates ① workspace member ② csv dependency released by the human 2026-11-02) — crates/datacraft-csv depends only on datacraft-core: directory = database, *.csv = tables, one atomic rewrite per write (temp + rename); DatabaseFormat::Csv with its single Display arm; composed at the UI/MCP/CLI roots via DetectingPort::with_fallback; server_e2e_csv pins the full stack over the wire)
- M12 — Native Query-Execution (MSysQueries ausführen, begrenzter Dialekt) — done (done in `3fd07a9` (RED `02b091a`; coverage `d668c7b`/`bf872eb`) — native selects execute the qualified Access dialect: equality joins (inner/left/right), WHERE expressions via the phase-15 service, Table.* projection and ORDER BY; fixture SelectQuery runs over the real port, UI Run is kittest-pinned, everything else answers a precise Unsupported)
- M14 — Encrypted Save-As + Passwortwechsel (verschlüsselte Kopien) — done (done in `8b2dccc` (RED `9614c5a`; coverage `bf872eb`) — SaveDatabaseAs writes unencrypted/protected Jet4 copies with a deterministic page key while the source stays untouched; ChangePassword rewrites in place atomically; ACE keeps its Office-encryption boundary and Jet3 stays read-only; backstage/CLI/JSON-RPC flows and capability gating are pinned)
- U14 — Command history for Undo/Redo — done (done in `a4ca640` (RED `926cbca`) — bounded per-database row-write history; Undo/Redo bus commands address rows by value image, restore_row preserves AutoNumber identities, journal/schema/file lifecycle are barriers, QAT enables from snapshot depths; Jet4 delete/restore verified across reopen, SQLite restore pinned)
- U15 — Recent databases in the backstage — done (done in `da1a0e8` (RED `2ad9d26`) — ui.json persists a bounded, de-duplicated recent list on DatabaseOpened; stale paths are pruned on load with a localized notice; the backstage Home/Open pages dispatch one OpenDatabase per click; kittest and updated Home snapshot pin the flow)
- M20 — Journal batches gate on the declared atomic_batch capability (Phase 30 follow-up) — done (done in `fc2b623` (RED `905ed72`) — preview_write and apply_write refuse multi-op batches on drivers with atomic_batch=false with the precise one-at-a-time reason; single-op batches stay available; Jet's atomic path unchanged; core and SQLite contracts pin both sides)
- H45 — Persist field properties (defaults, validation, captions, formats, input masks, index flags) — done (done in `c5abd00` (RED `c5176bd`) — the .datacraft sidecar is the property authority (ADR-1 addendum): tables are saved with the full FieldProperties sheet, overlaid on explicit load and sibling auto-load with precise stale/invalid handling, preserved across Jet/ACE schema refreshes; the Jet writer keeps NULL LvProp (Access display deferred to the LvProp roadmap item); design view renders the restored sheet (kittest))
