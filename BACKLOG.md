# BACKLOG.md — Open items (As of: 2026-11-01T14:48

> **This file contains only OPEN items.** Completed items are moved to `docs/archive/BACKLOG_ARCHIVE.md` unchanged after completion; this file keeps only a one-liner per item in the Done Index (below). Findings/fix-ideas/decisions are not deleted — moved to the archive.
> Legend: 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · 🔵 test gap
> Working order: sequential by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), each item test-first, commit per item.

---

## 🔴 CRITICAL

> No open items.

---

## 🟠 HIGH

> No open items.

### [ ] H39 — ACCDB-Read-Pfad (Catalog/TDEF/TDAT über den Jet4-4096-Pfad) — 🟠
- **Location:** `crates/datacraft-storage/src/jet/header.rs`, `reader.rs`, `catalog.rs`, `tdef.rs`, `row.rs`; `src/port_adapter.rs`; `crates/datacraft-core/src/types.rs` (`DatabaseFormat`)
- **Ansatz:** `FormatVersion::Ace`-Dateien über den bewährten Jet4-Lesepfad bedienen (4096-Byte-Pages sind bereits der Jet4-Pfad); ACE-spezifische Header-/Catalog-Feinheiten gegen die neuen Fixtures (H38) verifizieren; `DatabaseFormat::Ace` bis UI/CLI/MCP durchreichen (Format-Anzeige „(ACE)"). Oracle-Gegenprobe (mdb-tools) je Tabelle/Zeile; nicht unterstützte ACE-Varianten → präziser `Unsupported`.
- **Acceptance:** Tabellen/Zeilen/Beziehungen der ACCDB-Fixtures dekodieren vollständig; Oracle-Abgleich grün; Status-/Titelanzeige zeigt ACE-Format; präzise Fehlerfälle getestet.

### [ ] H40 — Jet4 Update/Delete-Writes (RowOp am echten Dateipfad) — 🟠
- **Location:** `crates/datacraft-storage/src/jet/writer.rs`, `writer_tables.rs` (ggf. neu: `writer_rows.rs`), `encode.rs`; `src/port_adapter.rs`; `crates/datacraft-storage/tests/`
- **Ansatz:** `RowOp::Update/Delete` am echten Dateipfad bedienen: In-Place-Update für Fixed-Width-Zeilen, Löschen über Zeilen-Flag + konsequente Free-Space-/Usage-Map-Pflege; LVAL-Ketten (Memo/OLE) beim Update sauber ersetzen/freigeben; Round-Trip decode∘encode∘decode plus mdb-export-Oracle wie im bestehenden Writer-Muster.
- **Acceptance:** Update/Delete-Tests auf Fixtures inkl. Memo/OLE-Feldern; Free-Map-Konsistenz nach Deletes; Oracle liest unsere Dateien fehlerfrei; Reopen verifiziert Persistenz der Änderungen; `Unsupported` für noch nicht qualifizierte Zeilenformen.

### [ ] H41 — Expression-Service (Ring 1 / Roadmap M1) — 🟠
- **Location:** `crates/datacraft-core/src/expr/` (neu: lexer/parser/eval), `schema/field.rs` (Default/Validation), `report.rs`/`reports.rs` (Bänder), `command.rs`
- **Ansatz:** Kleiner, totaler Ausdrucks-Evaluator: Feldreferenzen, Literale, Arithmetik/Konkatenation, Vergleichslogik, NULL-Semantik nach Access-Muster; Funktionsfamilien IIf/NZ/Format, Datums-/String-Funktionen, LIKE mit Access-Wildcards; typisiert über `FieldKind`. Anschlüsse: Default Values, Validation Rules, Calculated Fields sowie gruppierte/aggregate Report-Bänder (löst die M4-Report-Reste aus Phase 10 ein).
- **Acceptance:** Evaluator-Suite inkl. NULL-/Coercion-Fällen; Validation-/Default-/Calculated-Tests; Report-Preview mit Gruppen-/Aggregatbändern; Ausdrücke über Bus/MCP/CLI ausführbar; Expressions-Suite ist die Referenz für Ring 2/3.

---

## 🟡 MEDIUM

> No open items.

### [ ] M2 — Journaled/restringierte Write-Profile (Preview, Bestätigung, Feldfamilien) — 🟡
- **Location:** `crates/datacraft-core/src/command.rs`, `database.rs`, `event.rs`; `crates/datacraft-storage/src/port_adapter.rs`
- **Ansatz:** Schreiboperationen optional als Journal führen: Preview (Diff) → Bestätigung → Ausführung → Rollback-Info; Restriktion auf qualifizierte Feldfamilien (typsicher, Fixed-Width zuerst); Operationen außerhalb des Profils → präzise `Unsupported` statt stiller Annahmen. Baut auf H40 auf und ist die Voraussetzung für gefahrloses UI-Editing (Phase 11).
- **Acceptance:** Preview→Confirm→Apply-Tests; Rollback-/Journal-Tests; Restriktionsfehler präzise; E2E über JSON-RPC.

### [ ] M3 — Native Saved Queries (MSysQueries → QueryDef) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/` (MSysQueries-Decoder), `src/port_adapter.rs`; `crates/datacraft-core/src/qbe.rs`, `state.rs`
- **Ansatz:** `MSysQueries`-Zeilen lesen und auf `QueryDef` abbilden (Attribut-Codes in `docs/FORMAT-JET.md` dokumentieren); nicht abbildbare Konstrukte als dokumentierte Reste. Native Queries erscheinen in der Nav-Pane-Gruppe „Queries" und öffnen read-only im QBE; eigene Queries im Sidecar bleiben unberührt (kein Rückschreiben vor ACE-Writer-Entscheidung).
- **Acceptance:** Fixture mit nativen Queries; Listing + QueryDef-Öffnung über Bus/MCP; nicht unterstützte Attributtypen → präziser `Unsupported`; Nav-Pane zeigt native Queries neben Sidecar-Queries.

### [ ] M4 — Complex Values/Multivalue (Attachment-Subtabellen real decodieren) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/` (Complex-Decoder: benannte Unit-Tabellen f_*), `row.rs`, `tdef.rs`; `crates/datacraft-core/src/schema/field.rs`, `port.rs`
- **Ansatz:** Complex-Type-Spalten (0x12) real auflösen: zugehörige Value-/Attachment-/Named-Subtabellen erkennen, Werte pro Zeile gruppieren; Domain-Modell erweitern (Multivalue-/Attachment-Werte statt Binary-Platzhalter). Datasheet zeigt zusammengeführte Werte; QBE-Prädikate können auf Multivalue-Feldern arbeiten (EXISTS-Semantik dokumentiert).
- **Acceptance:** Fixture mit Multivalue-/Attachment-Feldern; Werte vollständig dekodiert; Datasheet-/QBE-Support getestet; Oracle-Gegenprobe; `FieldKind`-Mapping dokumentiert.

### [ ] M5 — Verschlüsselung/Passwort (RC4-Decode, Passwortpfad UI/CLI/MCP) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/header.rs`, `reader.rs` (Decode-Pfad), `row.rs`; `src/port_adapter.rs`; `crates/datacraft-core/src/port.rs` (Passwort-Parameter); UI-Prompt, CLI-Flag, MCP-Parameter
- **Ansatz:** RC4-Decoder für die verschlüsselte Header-/Datenregion (Schlüsselpfade je Formatgeneration in `docs/FORMAT-JET.md` dokumentieren); Passwort am `StoragePort::open`; alle Clients können Passwort übergeben (UI-Prompt, CLI `-p`, MCP-Parameter); verschlüsselte Test-Fixtures selbst erzeugt/pinnt (keine Fremd-Dateien mit echten Inhalten).
- **Acceptance:** Verschlüsselte Fixtures öffnen mit korrektem Passwort über alle vier Clients; falsches/fehlendes Passwort → präziser Fehler; Unverschlüsselte Files unchanged; Doku in FORMAT-JET.md.

### [ ] M6 — Doku-Genauigkeit (Kompatibilitätsaussagen ↔ verifizierter Stand) — 🟡
- **Location:** `README.md`, `docs/ARCHITECTURE.md` (§1 Mission, ADR-1), `docs/ROADMAP.md`, `docs/FORMAT-JET.md`
- **Ansatz:** Kompatibilitätsaussagen an den verifizierten Stand angleichen: ehrliche Read/Write-Matrix je Format (Jet3/Jet4/ACCDB/SQLite) mit Verweis auf die jeweiligen Tests/Gates; „bit-for-bit"-Claim auf die tatsächlich geprüften Pfade eingrenzen; Sidecar-vs-native-Absicherung (ADR-1) prominent halten. Reihenfolge: nach Abschluss von Phase 14 (ACCDB-Read), final mit Phase 17.
- **Acceptance:** Jede Kompatibilitätsaussage in README/ARCHITECTURE ist durch einen benannten Test/Gate gedeckt; Review-Absatz je Format; keine Übertreibung gegenüber der Fixture-Abdeckung.

---

## 🟢 LOW

> No open items.

### [ ] L3 — Jet3-Reste (Jump-Table-Zeilen >255 Bytes, verkettete TDEFs) — 🟢
- **Location:** `crates/datacraft-storage/src/jet/jet3.rs`, `tdef.rs`, `reader.rs`; `docs/FORMAT-JET.md` (Reste-Matrix)
- **Ansatz:** Jump-Table-Mechanismus für Zeilen >255 Bytes dekodieren; verkettete TDEF-Mehrfachseiten lesen; Reste-Matrix in FORMAT-JET.md entsprechend schrumpfen; präzise `Unsupported` nur für wirklich Exotisches.
- **Acceptance:** Jet3-Fixture mit großen Zeilen (Jump-Table) und verketteter TDEF; oracle-geprüft (mdb-tools); Reste-Matrix aktualisiert; bestehende Jet3-Tests bleiben grün.

### [ ] L4 — Makro-Inventur (Ring 3 / Roadmap M5) — 🟢
- **Location:** `crates/datacraft-storage/src/jet/` (Makro-Objekte lesen), `src/port_adapter.rs`; `crates/datacraft-core/src/state.rs`, `command.rs`, `ids.rs`; UI Nav-Pane
- **Ansatz:** Makro-Objekte als neue `ObjectKind`-Variante lesen (Inventur: Name, grob strukturierte Aktionsliste); Nav-Pane-Gruppe „Makros"; read-only Präsentation über Bus/MCP. DoCmd-Ausführung bleibt bewusst Ring-3-Umfang (Roadmap M5) und ist hier **nicht** Teil des Steps.
- **Acceptance:** Fixture mit Makros; Inventur über Bus/MCP abfragbar; Nav-Pane zeigt Makro-Gruppe; keine Ausführungssemantik im Scope.

### [ ] L5 — VBA-Extraction + Viewer + statische Analyse (Roadmap M6) — 🟢
- **Location:** `crates/datacraft-storage/src/` (`MSysAccessStorage` → CFBF-Reassembly), `crates/datacraft-core/src/vba/` (neu: dir-/Modul-Stream-Decoder), UI-Viewer-Modul; `command.rs`
- **Ansatz:** Embedded Compound File reassemblieren, `dir`-/Modul-Streams je `[MS-OVBA]` decodieren, Quelltext pro Modul ins Domain-Modell; Viewer (Modulbaum + Quelle + Volltextsuche) wie jedes andere DB-Objekt; headless statische Analyse (Call-Graph, eingebettete SQL-Literale, Tabellen-/Query-Referenzen, ungenutzte Module) — MCP-freundlich. Oracle: Referenz-*Outputs* als Golden-Daten, nie Code (Clean-Room-Muster wie H7). **Keine Ausführung** (ADR-2-Grenze).
- **Acceptance:** Fixture mit VBA-Projekt; Extraction deterministisch/stabil; Viewer + Analyse über Bus/MCP; ADR-2-Grenze (kein Interpreter) in Tests verankert.

---

## 🔵 TEST GAPS

> No open items.

### [ ] T4 — CLI-Bin-E2E über echten Subprozess (assert_cmd) — 🔵
- **Location:** `crates/datacraft-cli/tests/` (neu: `cli_process.rs`), `crates/datacraft-cli/Cargo.toml` (dev-deps `assert_cmd`, `assert_fs`, `predicates`); `docs/TESTING.md` §3 Registry #2
- **Problem:** Der CLI-Bin wird heute nur über injizierte Streams (`run_with_streams`) getestet — genau deshalb steht `main.rs` mit 17 Zeilen als ungedeckte Ausnahme in der Coverage-Registry (#2, „there is nothing to assert without a subprocess"). Prozessgrenze, echtes stdin/stdout, Arg-Parsing und Exit-Codes sind ungetestet, obwohl README deterministische Exit-Codes als Feature nennt.
- **Ansatz:** Bin als echten Kindprozess starten (`assert_cmd::Command::cargo_bin`), gegen `fixtures/testV2000.mdb` sowie Scratch-SQLite (`assert_fs`/`tempfile`); open/exec/repl-Abläufe über echte stdio-Rohre fahren; Exit-Code-Matrix je Fehlerklasse (fehlende Datei, kaputtes Argument, Read-only-Write) explizit assertieren; Timeouts so setzen, dass ein hängendes REPL den Lauf nicht blockiert.
- **Acceptance:** Mindestens open+exec+nicht-interaktives REPL über den echten Prozess getestet; jeder dokumentierte Exit-Code einmal getroffen und geprüft; Fehlerfälle präzise auf stderr; Registry-Ausnahme #2 in `docs/TESTING.md` neu bewertet (verkleinert oder begründet bestehen bleiben); canonical verification set grün inkl. Coverage-Gate.

### [ ] T5 — egui/eframe headless UI-Tests (egui_kittest, Layout-Regressionen) — 🔵
- **Location:** `crates/datacraft-ui/tests/` (neu: `ui_kittest.rs`), `crates/datacraft-ui/Cargo.toml` (dev-dep `egui_kittest`), `docs/TESTING.md` §3 Registry #1, `docs/ARCHITECTURE.md` (Schichtgrenze ui/core)
- **Problem:** Die UI-Schicht ist komplett aus der Coverage ausgeschlossen (Registry #1: „eframe startet keinen headless Event-Loop"). Damit sind alle Render-Pfade — Datasheet, SQL-Fenster, Nav-Pane, Ribbon, Statusleiste — ungetestet: Layout-Regressionen und kaputte Widget-Bindings fallen erst im manuellen Lauf auf. `egui_kittest` (offiziell im egui-Repo, AccessKit-basiert, Snapshot-Renderer) schließt genau diese Lücke.
- **Ansatz:** Harness mit `Harness::new_ui`/`new_state` gegen den echten `Core` mit MockPort aufsetzen; Widgets über AccessKit-Knoten (Label/Rolle) ansprechen statt über Pixelkoordinaten; Abläufe fahren (Tabelle in Nav-Pane wählen → Datasheet zeigt Zeilen, Query ausführen → Ergebnis-Grid, Ribbon-Aktion → Bus-Event); Snapshot-Vergleiche (`insta`-Backend von egui_kittest) für die Hauptfenster als Layout-Gate. **Versionskopplung vorab klären:** `egui_kittest` muss exakt zur gepinnten `eframe`/`egui`-Version (0.37) passen — falls für 0.37 nicht verfügbar, Item auf ⛔ setzen und Alternative (Upgrade-Kopplung an egui-Release) im Item notieren statt zu improvisieren.
- **Acceptance:** Harness startet headless ohne Fenster/GPU-Zwang und ohne echtes Event-Loop; je Hauptbereich (Datasheet, SQL/QBE-Fenster, Nav-Pane, Ribbon) mindestens ein Interaktions- plus ein Snapshot-Test; Assertions auf Domain-Zustand (`UiStateSnapshot`) statt auf Pixel; Registry-Ausnahme #1 in `docs/TESTING.md` neu bewertet (Verkleinerung dokumentieren oder Restbegründung); canonical verification set grün, Snapshot-Artefakte reproduzierbar (kein Flaky-Vergleich durch Font-/DPI-Unterschiede in CI).

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
