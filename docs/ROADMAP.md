# DataCraft — Compatibility Roadmap

> Created 2026-10-31 from the programmability/VBA scope and scripting strategy
> discussions (human-approved). Living document — revisited when a ring milestone starts
> or on strategic changes.
> Language: English (binding, see `docs/PLAYBOOK.md` §2).
> Related: `docs/ARCHITECTURE.md` (mission, principles, crate graph) ·
> `PROGRESS.md` (phase tracking) · `BACKLOG.md` (concrete work items).
> This doc is deliberately strategy-level — the subject felt too abstract for
> `BACKLOG.md`. Work items are created from it only when the owning phase starts.

## 1. Framing

"Rebuilding Access" is not one problem. Access decomposes into rings of
compatibility with very different cost and interdependence. Most logic in real
Access applications lives in the **inner rings** — bound forms, queries,
macros, expressions — with VBA as glue on top. A runtime that executes the
inner rings therefore makes the majority of typical `.mdb`/`.accdb` apps
meaningfully runnable **without any VBA execution**.

Core commitment of this roadmap: buy compatibility ring by ring,
cheapest-first, and never pay for the outermost ring (VBA execution) while
inner rings are still open.

## 2. The compatibility rings

| Ring | Capability | Depends on | Cost / boundedness | Phase mapping |
|------|------------|-----------|--------------------|---------------|
| 0 | Storage: Jet4 read → ACE read → Jet4 write → ACE write | — | high; spec + oracle driven | Phase 2, H9 (write), L1 (Jet3), Phase 25 (ACE write) |
| 1 | **Expression service** — `=Now()`, `IIf()`, `[Field] & " x"`, `Format()`, default values, validation rules, calculated fields | — | small, well-bounded language | ✅ Phase 15 (steps 15.1–15.2) |
| 2 | Query engine — Jet SQL + the algebra stored in `MSysQueries` (QBE compiles to it) | R1 | hard but bounded | Phase 7 |
| 3 | Macros — DoCmd action set (~60 documented actions, declarative) | R1 | bounded, low cost per action | ✅ Phase 17, step 17.1: read-only inventory (DoCmd execution remains out of scope) |
| 4 | Forms/Reports runtime — bound forms, events, record navigation, requery | R1, R3 | largest single item | Phase 8 (canvas/design first), runtime extends 8.3 |
| 5 | VBA | R4 **and** runtime semantics | out of scope as interpreter — see §3 | ✅ Phase 17, steps 17.2–17.3: extraction, viewer, static analysis (no execution) |

## 3. VBA strategy — decision record

**Decision (2026-10-31, human-approved):** DataCraft does **not** implement a
VBA interpreter or VBA runtime. VBA is extracted, stored, displayed and
statically analyzed. Byte-compatible VBA *execution* is explicitly out of
scope, subject to the re-evaluation gate below.

**Rationale:**

1. **Format ≠ runtime.** Extraction is spec-driven (`MSysAccessStorage` →
   embedded CFBF → `[MS-OVBA]`/`[MS-CFB]` module streams) and cheap — the
   format side is one of the *best*-documented parts of the Access ecosystem,
   with three independent prior implementations (olevba, Jackcess `VbaProject`,
   jetdb) proving the docs suffice. Execution means reimplementing runtime
   semantics — Variant coercion, late binding, `On Error Resume Next` control
   flow, implicit ByRef, event lifecycle — with **no open-source reference VM**
   to build against. Multi-year effort with a moving target.
2. **Access VBA is undefined without its host.** Typical modules reference
   `Me`, `Forms!…`, `DoCmd`, DAO, `Screen.ActiveControl`. Even a perfect
   language interpreter has nothing to bind those to; the true prerequisite is
   Ring 4. VBA execution is therefore gated behind the *most expensive* ring
   anyway.
3. **Strategic contradiction.** Microsoft distributes the free Access Runtime —
   byte-compatible execution of existing apps would compete with a free product
   on its home terrain. DataCraft's reason to exist is cross-platform +
   headless automation (ARCHITECTURE §1, Principle 5: no COM anywhere) — and
   the VBA VM is precisely a Windows-native COM component. Reimplementing it
   on the platforms we are leaving would be self-defeating.

**What we do instead (in scope):** — delivered in Phase 17, steps 17.2–17.3.

- **Extraction** (fits storage milestones; lands via existing LVAL/long-value
  decoding): read `MSysAccessStorage` (Jet3: `MSysAccessObjects` — Jet 3
  extraction is still a remainder), decode `dir` + module streams per
  `[MS-OVBA]`, keep per-module source in the domain model. Oracle:
  reference *outputs* as golden data, never code — same clean-room pattern
  as the libmdb oracle (H7).
- **Presentation:** module tree + source viewer + full-text search in the
  Phase 5+ UI; the VBA project appears like any other database object.
- **Static analysis (headless, MCP-friendly):** call graph per project, SQL
  embedded in string literals, table/query references from code, unused
  modules — high migration value, zero execution required.
- **Out of scope, stated plainly:** p-code / `_VBA_PROJECT` performance cache,
  VBA-stomping recovery, COM add-in interop, `Declare`/WinAPI execution.

**Re-evaluation gate.** Revisit execution only if **both** hold: (a) Ring 4
(forms runtime) is shipped, and (b) real user demand exists for running
*unmodified* apps. If triggered, the path is still not interpretation:
(a) transpile the per-app *used subset* (migration tooling; success metric
"95 % of real glue code migrates"), and (b) offer an own, documented extension
story for apps built *on* DataCraft — the scripting strategy in §4 is that
story. The target is never "VBA-compatible per spec"; it is "the old app's
logic comes across".

## 4. Scripting strategy — decision record

**Decision (2026-10-31, human-approved):** DataCraft ships **no own scripting
language** and no embedded script engine for now. Automation is command-bus
first; an embedded *existing* language, chosen later behind a reserved
`ScriptEngine` trait, is a post-Ring-4 decision. Never a custom DSL.

**Rationale:**

1. The language is ~30 % of the cost, the ecosystem ~70 %: editor support,
   completion, diagnostics, debugging, docs and community cannot be hand-rolled
   alongside everything else — and Access migrants would gain zero familiarity
   from a bespoke DSL anyway.
2. Agents are a first-class user (Principle 3). They write what exists in
   their training data — Python, TypeScript, SQL, Lua — so an exotic dialect
   is the weakest possible interface for DataCraft's own audience.
3. Code outside the file is no macro-malware vector — relevant for a tool
   that opens foreign `.mdb`/`.accdb` files.

**Stages:**

- **Stage 0 (now):** the command bus is the scripting layer — headless
  JSON-RPC/MCP + CLI (H8, Phase 9) covers the VBA automation use cases
  (imports, batch maintenance, mass updates) with scripts in any language.
- **Stage 1 (Ring 4 events):** declarative first — event → action mapping plus
  Ring 1 expressions, mirroring how many real Access apps are macro-only.
- **Stage 2 (gated, post-Ring-4):** embed an existing sandboxed language behind
  a `ScriptEngine` trait in core (reserved, unimplemented). Decision then, not
  now; candidates:

| Candidate | Strength | Weakness |
|---|---|---|
| Rhai (pure Rust, sandboxed) | zero friction with the Rust stack, safe | niche language, weak agent training data |
| Lua / Luau (mlua) | battle-tested, tiny, agent-friendlier | C bindings; Luau is the more modern fork |
| WASM (wasmi, pure Rust) | real sandboxing guarantees, language-agnostic | heaviest model; "code inside DB files" question remains |

**Out of scope at every stage:** interpreting VBA (see §3) and auto-executing
any code embedded in database files without explicit user consent/permissions.

## 5. Sequencing

| # | Milestone | Unlocks | Status |
|---|-----------|---------|--------|
| M0 | Storage read milestones (Jet4 → ACE) | everything | ✅ Jet4 (Phase 2), Jet3 (Phase 10), ACE 2007/2010/2016 (Phase 14, oracle-checked); SQLite sources (Phase 13); ACCDB write (Phase 25) |
| M1 | Expression service (Ring 1) | queries, macros, forms, validation | ✅ Phase 15 (steps 15.1–15.2, H41 archived; `Now`/`Date` deliberately absent — deterministic) |
| M2 | Query engine + QBE (Ring 2) | real app behavior | ✅ Phase 7 · native query execution ✅ Phase 27 (M12) |
| M3 | Write path (Ring 0 write) | editing apps, Access-compatible write goal | ✅ Jet4 writer landed in Phase 10 (create + insert); update/delete + journal profiles landed in Phase 11 (steps 11.10–11.11); ACCDB write-through landed in Phase 25 (writer contracts + port + wire/CLI, README matrix); drops, design-time alter, field indexes and relationships landed in Phases 24 and 32 |
| M4 | Forms/Reports runtime (Ring 4) | the Access UX itself | ✅ Phase 8 (forms) + Phase 10 (report designer + print preview) + Phase 15 (grouped/aggregate bands) |
| M5 | Macros (Ring 3) | unattended app logic | ✅ Phase 17, step 17.1: read-only inventory (name + coarse arguments); DoCmd execution stays out of scope |
| M6 | VBA extraction + viewer + static analysis | migration tooling, fidelity | ✅ Phase 17, steps 17.2–17.3 (extraction, viewer, `AnalyzeVba`; no execution) |
| M7 | `ScriptEngine` trait hook in core (unimplemented) + engine decision | stage-2 scripting when the §4 gate opens | ⬜ gated (Ring 4 + demand) |
| — | VBA execution | — | ⛔ out of scope (see §3 gate) |

Ordering notes:

- Planned sequence (2026-11-02; human decision — execution diverged, see the executed orders below): all feature work
  first, review and documentation before the release, release engineering
  last — Phase 27 (native query execution + encrypted save-as) → Phase 28
  (undo/redo + recent databases) → Phase 33 (journal batch integrity, M20)
  → Phase 32 (writer catalog fidelity: H45 field properties, M15 indexes,
  M16 relationships) → Phase 34 (cross-cutting code review, R1 — before
  publishing so findings feed the crate split) → Phase 31 (engine
  publishing: M21 vestige retirement as the prep step, crate split,
  standalone MCP binary, Semver policy) → Phase 35 (documentation sweep,
  R2 — after the crate structure freezes) → Phase 26 (release engineering +
  first tags, M13). Supersedes the earlier plan that placed Phase 26 right
  after Phase 30; the first tag should carry the complete write surface,
  a walked review checklist and contradiction-free entry-point docs.
  Phase numbers stay fixed.
- Executed order (2026-11-05): Phase 26 (release engineering, M13)
  delivered early — the release workflow, packaged smoke and tag policy
  shipped ahead of the first tag; Phases 27–29 landed as planned; Phases
  31–34 (publishing/crate split, schema writing, journal gating,
  cross-cutting review) and Phases 36–43 (driver edge suites, release
  readiness, UI restyle, correctness fences, datasheet UX, GUI parity,
  core correctness, coverage pinning) executed ahead of the sweep, so
  Phase 35 (documentation sweep, R2) runs last before the first tag —
  the entry-point docs are the remaining gate.
- Executed order (2026-11-02): Phase 30 (driver model — ADR-7: declarative
  per-session capabilities, registry detection, CSV directory driver as the
  out-of-tree proof, core-computed UI affordances), after Phase 25 (ACCDB
  write) and Phase 29 (sample databases and templates).
- Executed order (2026-11-02): Phase 25 (ACCDB write) → Phase 29 (sample
  databases and templates, released ahead of the rest). Still planned:
  Phase 26 (release engineering — deferred), Phase 27 (native query
  execution + encrypted save-as), Phase 28 (undo/redo + recent databases).
- Planned sequence (2026-11-02): Phase 25 (ACCDB write) → Phase 26 (release
  engineering) → Phase 27 (native query execution + encrypted save-as) →
  Phase 28 (undo/redo + recent databases). Phase numbers stay fixed.
- Planned sequence (2026-11-01): Phase 13 (SQL/SQLite) → Phase 14 (ACCDB read)
  → Phase 11 extension (update/delete + journal profiles) → Phase 15
  (expressions + saved queries) → Phase 16 (complex values, encryption, Jet3
  remainders) → Phase 17 (macros, VBA extraction, docs accuracy). Phase numbers
  stay fixed; execution follows this sequence (precedent: Phase 12 ran before
  Phase 11).
- M1 is the keystone and deliberately front-loaded — every later ring consumes
  it. It has no natural owner in the Phase 2–10 plan yet; give it a phase when
  Phase 3 planning starts.
- M5 has no hard dependency on M4 (only on M1) and can be pulled earlier if
  user feedback rewards it.
- M6 interleaves with the storage milestones and does not block anything.
- M7 is a declaration, not work: the trait hook stays unimplemented until the
  §4 stage-2 gate opens.
- This roadmap does not renumber `PROGRESS.md`; when a milestone becomes
  active, its steps are planned there and in `BACKLOG.md` as usual.

## 6. Deferred scope decisions (2026-11-02, Phase 27/28/32 releases)

Three deliberate boundaries were decided while releasing Phases 27, 28, 32
and 33. They are scope decisions, not open defects; each one names the
follow-up that would lift it.

- **Field properties: sidecar authority (H45).** DataCraft persists the
  full `FieldProperties` set in the `.datacraft` sidecar's `tables` list
  and treats that as its own authority. The Jet/ACE catalog keeps `NULL`
  `LvProp` (the file stays Access-readable, and Access does not display
  DataCraft-set defaults/captions/validation). A future item may encode the
  `MR2` property pack into `LvProp` and decode it back; that is a
  High-class storage-format change (clean-room format verification first)
  and is intentionally not part of Phase 32.
- **Undo/Redo scope: row writes only (U14).** The history covers row
  inserts/updates/deletes and applied journal batches. Schema edits
  (`CreateTable`/`DropTable`/`AddField`/`DropField`/`RenameField`/
  `SetFieldProperties`), QBE/form/report design edits and file-level
  commands are not undoable and answer a precise reason. A schema-history
  follow-up needs reverse operations that restore dropped table data; it
  is a separate design (and must not weaken the current honest boundary).
- **Jet index fidelity: metadata plus uniqueness (M15/M16).** The writer
  emits TDEF index descriptors and enforces `IndexMode::NoDuplicates` on
  writes; SQLite gets the matching `CREATE INDEX`/`FOREIGN KEY` DDL and
  relationships land as `MSysRelationships` rows. Physical Jet index
  B-tree pages (leaf/intermediate pages and their maintenance on
  insert/update/delete) are not written yet — Access may rebuild them when
  it opens the file (documented manual gate). A follow-up item covers the
  index-page format research and maintenance; it is the largest remaining
  writer-fidelity risk.

## 7. Format families beyond Access — decision record

**Decision (2026-11-06, human-approved):** DataCraft adds Excel workbook
support as a new format family, delivered as its own driver module under the
ADR-7 model (one new `DatabaseFormat` variant, sniff + registry composition,
sheets exposed as tables). **Read-only first**; a write path is a separate,
later decision. Work items are created only when the owning phase starts.

**Format matrix:**

| Format | Read | Write | Boundary |
|--------|------|-------|----------|
| .xlsx (OOXML SpreadsheetML) | ✅ first | later phase | core format |
| .xlsm | ✅ (values) | later phase | macros are never read, executed or modified (same boundary as VBA, §3) |
| .xls (BIFF8) | ✅ (values) | ⛔ out of scope | BIFF8 writing is a format family of its own; the clean-room reference (GridCraft) also imports .xls only |
| .xlsb (binary OOXML) | ✅ (values) | ⛔ for now | data import from saved values |
| .ods / .fods (ODF) | — | — | deliberately a **separate later module** (decision 2026-11-06) |

**Implementation strategy:** the reader is
[calamine](https://crates.io/crates/calamine) (MIT, pure Rust, synchronous,
empty default features) — same precedent as datacraft-sqlite using rusqlite
for a publicly standardized format. The writer candidate for the later phase
is rust_xlsxwriter (MIT OR Apache-2.0). When writes arrive they must respect
the deterministic-write rule and never silently destroy formulas, styles or
objects of foreign workbooks; a whole-file rewrite (CSV precedent) is
acceptable only for pure data sheets or workbooks DataCraft created itself.
Password-protected workbooks fail with a precise `Unsupported` error (ACE
office-encryption precedent).

**Prior art (2026-11-06):** GridCraft (`storytold/gridcraft`, clean-room Rust
spreadsheet, MIT OR Apache-2.0, alpha) was studied as a fidelity yardstick:
its native .xlsx round-trips formulas/styles/tables, while .xls is import-only
and .xlsb/.ods import worksheet data only — supporting the read-only boundary
above. Its `gridcraft-xlsx` crate is **not** usable as a dependency here
(couples to gridcraft-core/model/formula/math; conflicts with the ADR-7
rule that out-of-tree drivers depend only on `datacraft-core`).

**Downstream:** once the read driver exists it feeds M50 (Import File —
"ADR-7 driver model makes later sources additive") and a future
table-export command. Release gates per PLAYBOOK §0.5: new crate + calamine
dependency = Medium, fixtures = Medium, any CI oracle job = High.

## 8. ODF spreadsheets (OpenOffice/LibreOffice) — decision record

**Decision (2026-11-06, human-approved):** ODF spreadsheet support is planned
as its own driver module, deliberately separate from the Excel family (§7).
Same ADR-7 shape: one new `DatabaseFormat` variant, sniff + registry
composition, sheets exposed as tables. **Read-only first**; a write path is a
separate, later decision. Work items are created only when the owning phase
starts; sequencing sits behind the Excel module (§7) — no hard dependency,
but the driver pattern, the calamine pin and the release gates are shared.

**Format matrix:**

| Format | Read | Write | Boundary |
|--------|------|-------|----------|
| .ods (OpenDocument Spreadsheet) | ✅ first | later phase | the native Calc format of OpenOffice/LibreOffice — OASIS ODF, ISO/IEC 26300 |
| .fods (flat XML ODF) | later step | ⛔ for now | single-file XML variant, not covered by calamine; a clean-room flat-XML reader from the public spec is the natural path |
| .ots (ODF spreadsheet template) | unconfirmed | ⛔ | structurally an .ods with a template mimetype — verify against calamine at phase start before promising it |
| legacy OpenOffice 1.x (.sxc/.stc) | — | — | out of scope (pre-ODF format) |

**Implementation strategy:** read via calamine — it already covers .ods, so
the module adds no new reader dependency; both driver crates share one pin in
`[workspace.dependencies]`. Note: calamine loads all sheets of an .ods at open
time (no lazy loading) — acceptable for the table-level read path, a footnote
for very large workbooks. The writer candidate for the later phase is
[spreadsheet-ods](https://crates.io/crates/spreadsheet-ods) (MIT OR
Apache-2.0, 1.0.x, actively maintained); the same deterministic-write rule as
§7 applies — never silently destroy styles, formulas or objects of foreign
files. Password-protected files fail with a precise `Unsupported` error (ACE
office-encryption precedent).

**Prior art (2026-11-06):** GridCraft (`storytold/gridcraft`) imports .ods
worksheet data only (saved values instead of formulas) — supporting the
values-first read boundary. ODF is a fully public OASIS/ISO specification, so
a clean-room extension beyond library support (e.g. .fods) is
straightforward.

**Downstream:** identical to §7 — feeds M50 (Import File) and a future
table-export command once the read driver exists. Optional oracle pattern:
LibreOffice headless as `DATACRAFT_ORACLE_ODS` (env-var gated, mirrors
DATACRAFT_ORACLE_MDB). Release gates per PLAYBOOK §0.5: new crate = Medium,
fixtures = Medium, any CI oracle job = High.

## 9. Remote sources & linked tables — decision record

**Decision (2026-11-06, human-approved):** DataCraft approaches remote
databases as a staged ladder, native-protocol first. It maps to Access's
External-Data/linked-tables surface, and the ladder buys migration value
before connectivity: (0) read-only linked-table inventory in Jet/ACE files,
(1) the `SourceSpec { Path, Uri }` seam (BACKLOG L8) landing together with
the first server driver, (2) one driver family per protocol, read-only
first, (3) linked-table resolution inside an open database. Work items are
created only when the owning phase starts.

**Stage ladder:**

| Stage | Content | Basis |
|-------|---------|-------|
| 0 — Link inventory (read-only) | Decode MSysObjects link records (Type 4 ODBC-linked, Type 6 Access-file-linked: CONNECT, DATABASE, foreign name, flags) and surface them in catalog/UI metadata. Today they are silently invisible — the catalog reads only Id/Name/Type/Flags and non-TDEF rows are skipped. Pure Jet-format work; no network, nothing executed. | Phase 17 macro-inventory precedent: read and display, never execute |
| 1 — SourceSpec + first driver | L8 lands with the first server driver (PostgreSQL, read-only): additive `SourceSpec { Path, Uri }` on the open commands, snapshot gains a source display, the sidecar stays Path-only (ADR-1 rule kept), credentials via UI prompt / headless env — never persisted in the sidecar. | ADR-7 documented future path; BACKLOG L8 |
| 2 — MySQL/MariaDB (read-only) | Second driver family; same shape, same capability honesty. | — |
| 3 — Linked-table resolution | Linked tables inside an open Jet/ACE file resolve to Stage-1/2 sources; reads materialize into the query-engine path; writes stay out of scope initially. Resolution is an explicit user action — opening a database never dials out (ADR-2 analog: a file must not trigger side effects). | "SQL over all sources" materialization precedent (in-memory SQLite) |

**Client strategy (pure Rust, synchronous — the no-async rule is binding):**
PostgreSQL via the `postgres` crate (native, synchronous, pure Rust,
MIT OR Apache-2.0), MySQL/MariaDB via the `mysql` crate (synchronous,
pure Rust, MIT OR Apache-2.0). Identifiers reaching a server pass the
schema whitelist (existing hard rule); values are parameterized; TLS via
rustls features. Connection failures surface as precise, locatable errors.

**Gates and out of scope, stated plainly:**

- **SQL Server / TDS: deferred behind a gate.** Pure-Rust TDS clients are
  async-only today, conflicting with the no-async rule. Revisit when a
  synchronous pure-Rust client exists or the no-async principle is formally
  reopened (High-class principle change). Same gate pattern as M7 (§4).
- **ODBC: out of scope.** Access's own linked-table road — it needs the
  system driver manager (odbc32/winapi, unixODBC C) and breaks the
  no-winapi / pure-Rust principles. Native protocol drivers replace it.
  Re-evaluation gate: only if native coverage is proven insufficient in
  practice.
- **File-on-share stays an OS concern** (a share path works as a path
  today; local lock-file concurrency is M59's scope; multi-user
  shared-.mdb editing is not part of this record). A network transport for
  the command bus (remote MCP/JSON-RPC) is a separate future topic, not
  decided here.
