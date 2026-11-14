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
| M4 | Forms/Reports runtime (Ring 4) | the Access UX itself | ⬜ designer + preview shipped (Phases 8, 10, 15 — forms designer, report designer + print preview, grouped/aggregate bands); the events/requery runtime (§2 Ring 4) stays open |
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

## 10. Maintenance & salvage — decision record

**Decision (2026-11-06, human-approved):** DataCraft ships a maintenance
track: Compact & Repair as a first-class command, a read-only salvage reader
for damaged files, and a pre-open health check. Today none of these exist —
no command touches maintenance.

**Rationale:** compact is cheap here — a verified Jet4/ACE writer already
exists, so compact = read everything, write a fresh file, swap atomically
(the sidecar follows the path, not the inode). Salvage is the clean-room
differentiator: a reader that recovers every readable row, never writes, and
reports damage precisely per page/row is the honest version of the "Access
repair" emergency that Access's own repair handles silently at best.

| Capability | Read/write | Boundary |
|-----------|------------|----------|
| Compact & Repair (command + backstage) | write, always a new verified file | in-place rewrite is out of scope; repair = compact + per-object damage report, never silent fixes |
| Salvage reader | read-only | recovers readable rows into a new file or a report; per-page errors are locatable (existing error rule) |
| Pre-open health check | read-only | header/region/page sanity before presenting the file; feeds the M59 lock-file notice |

Order: health check → compact (writer reuse) → salvage (largest; needs a
damage-model fixture set from clean-room sources — Medium gate each).

## 11. Export matrix — decision record

**Decision (2026-11-06, human-approved):** External data is bidirectional —
Access's External-Data story is import AND export, while DataCraft so far
records only read-side families (§7/§8/§9). Export becomes its own record,
same capability-honesty style.

| Target | Order | Basis / boundary |
|--------|-------|------------------|
| CSV | first | the `csv` crate already writes (the datacraft-csv driver uses it for its atomic rewrite); the export command is a new core command, distinct from the CSV *driver* (a directory of tables) |
| JSON (tables + schema) | early | agent-first surface for MCP consumers; pure data, no styling |
| xlsx / .ods | after the §7/§8 read drivers | writer candidates are already named there (rust_xlsxwriter, spreadsheet-ods); same deterministic-write rule |
| PDF from report preview | later | the print preview already paginates in core; PDF is a renderer question, not a core one |
| Print | later | OS print dialog is UI-owned; core stays headless |

Exports are explicit user actions producing new files; the source database
is never modified. Multi-table (whole-database) export follows single-table.

## 12. Text semantics & collation — decision record

**Decision (2026-11-06, human-approved):** text comparison semantics become
an explicit, documented boundary instead of an accident of the code path.
Today `Like`/QBE use Access semantics (case-insensitive, `* ? #` wildcards —
Option Compare Database), while the SQL engine uses SQLite semantics
(`%` wildcards, SQLite text ordering), and FORMAT-JET.md documents byte-order
text equality as a divergence from Access. The two wildcard languages mirror
Access's own ANSI-89 (QBE, `*`) vs ANSI-92 (SQL, `%`) split — DataCraft
keeps that split, but as a *declared* rule per engine, not an undocumented
accident.

Staged follow-ups (work items at phase start): (a) document the semantics
map per engine in TESTING/ARCHITECTURE; (b) read the database collation from
the Jet header and honor it for text ordering in queries, replacing raw byte
order — deviations stay listed in the compatibility matrix; (c) per-column
collation on the create path (already flagged as a real-writer convention in
FORMAT-JET). Out of scope: locale-aware collation tables beyond what the
file declares — an honest support matrix instead.

## 13. Referential integrity enforcement — decision record

**Decision (2026-11-06, human-approved):** the backends must agree on write
semantics. Today SQLite enforces foreign keys (`PRAGMA foreign_keys=ON`,
orphans refused) while the Jet path writes `MSysRelationships` metadata
only — no orphan rejection, no cascade behavior. That breaks the parity
principle: identical state transitions across entry points *and* backends.

Ladder: (a) minimum parity — the Jet write path rejects orphan
inserts/updates against declared relationships with a precise, locatable
error; (b) cascade DELETE/UPDATE honoring the `MSysRelationships` flags as a
separate later step — it touches the physical Jet index pages, so the §6
M15/M16 deferred index work is a prerequisite; (c) cascade options in the
relationships editor ship with (b). SQLite semantics stay as they are; the
acceptance bar is "same input → same visible outcome on both backends".

## 14. Migration assessment — decision record

**Decision (2026-11-06, human-approved):** generalize `AnalyzeVba` into an
app-level readiness report — the "what breaks if I move this app" command.
`AnalyzeVba` stays as-is; a new assessment command aggregates per-object
evidence: ring coverage per form/report/query (bound? decodable? runs in
Ring 1/2?), VBA modules and their transpilable-subset estimate, the
linked-table inventory (§9 Stage 0 output feeds exactly here), an
unsupported-feature list, and an effort summary. Headless and MCP-friendly
like `AnalyzeVba`; read-only by construction. Work items are created when
the owning phase starts.

## 15. Small boundaries & record hygiene

- **Format floor:** Jet 1.x/2.0 (`.mdb` below Jet 3) are out of scope; the
  verified floor is Jet 3 (Phase 10). Stated once, here.
- **WASM/browser:** a browser build of the UI is a non-goal for now — the
  headless clients (CLI/MCP) already cover remote automation. Deliberate
  statement, not a gap.
- **Distribution:** installers/signing/auto-update are a post-tag topic;
  the release workflow (M13) governs tags, the CHANGELOG stays
  human-reviewed. v0.1.0 shipped 2026-11-08: all six
  crates are on crates.io and the tag `v0.1.0` exists; the GitHub release
  was published on 2026-11-10 after the human draft review, with notes
  composed from CHANGELOG's [0.1.0] section — the workflow's
  `--generate-notes` draft step fails without a checkout (M72). v0.1.1 followed on
  2026-11-10: six crates on crates.io, tag and published GitHub release, with
  the draft-step fix landed (phases 66/67). v0.1.2 was published on
  2026-11-12 (release run 43262808856 fully green; the draft step composed
  its notes from CHANGELOG's [0.1.2] section and attached the three
  archives plus the checksums — M72 is closed by this run). v0.1.3 followed
  the same day: tag, fully green release run 43310995863, published draft
  with notes from CHANGELOG's [0.1.3] section (SQLite depth stage 0 and the
  native-object inventory). Both versions are also on crates.io: all six
  crates were published in checklist order for 0.1.2 and 0.1.3 on
  2026-11-12 (0.1.2 from the tag worktree, 0.1.3 from `main`), and
  `cargo install datacraft-mcp` resolves the published 0.1.3.
- **Hygiene (2026-11-06):** §5 M4 corrected from ✅ to ⬜ — the
  designer/preview phases landed, the events/requery runtime of §2 Ring 4
  did not; §3's VBA re-evaluation gate keys on the runtime, not the designer.

## 16. GUI interaction strategy — decision record

**Decision (2026-11-07, human-approved):** the GUI's problem class is not
missing surface but interaction honesty — the Walkthrough-Fix arc (Phases
44–48) shows the same failure classes recurring: affordances that do nothing
visible (M54 "unsortable … wie in Excel", M55 dead criteria editor, M58
ribbon not following, M60 "arrows appear dead"), content painted where the
user cannot see it (H52, H53, M61), and dishonest affordances (M49's planned
Import). The strategy is a ladder: fix the feedback class first, then close
the missing interaction vocabulary, then designer parity, then
discoverability, then the power-user surface. Work items are created when
the owning phase starts; Phase 48 (M60–M62) has since closed under this
ladder's Stage-0 sweep.

**Stage ladder:**

| Stage | Content | Basis |
|-------|---------|-------|
| 0 — Feedback & honesty sweep | Ctrl+Z/Ctrl+Y (the undo/redo commands exist — only the keys are missing), tooltips for every ribbon/backstage control (17 hover sites today), a busy/progress affordance, a notice/error history (today: one transient status-bar line), localized failure reasons (I6 — delivered 2026-11-08), a shortcuts help page, and the deferred M42 subjective polish pass on the token system | ✅ delivered 2026-11-11 (phase 68, steps 68.1–68.5/68.7): keys, registry tooltips, bounded history, F1 page, deferred busy overlay; the M42 polish step stays open until the human's live review (M78) |
| 1 — Datasheet completeness | filter-by-value dialog (a value list, not the raw criteria string — M62 documents that dialect today), multi-select + cell copy/paste, find/replace, column drag-reorder, a visual Null marker, and windowed row reads in core (today: full-scan cache per tab) | M54: the user's stated bar is "wie z. B. in Excel" |
| 2 — Designer parity | control property sheet (today only table-design fields have one), resize handles (move-only today), z-order/tab order, preview zoom (hard-coded 0.9), drag-drop field binding (click-to-add today) | ADR-1's WYSIWYG contract (M57 was its first breach) |
| 3 — Discoverability & onboarding | nav-pane search/filter, the category model (the "All Access Objects" menu is a stub), object counts per group, nav-pane drag-drop, teaching empty states ("what can I do next"), QAT customization | the M52–M58 walkthroughs: users must find affordances unaided today |
| 4 — Power users & agents | command palette (Ctrl+K) projecting the core ribbon spec — the spec already lives in core, so the palette is a UI projection of the same list; full Alt key-tip phases (Legacy/Backstage currently out of scope); ribbon collapse | the command-bus principle: every UI action is already a named Command |

### Control capabilities as of v0.1.0 (2026-11-08)

The honest state of the form/report toolbox — what each control does on
every surface, and where the gaps are tracked. Placement: a toolbox click
adds the control at the next free slot (snap-to-grid, auto-named,
kind-specific defaults); **Add Existing Fields** click-adds a control
bound to a source field (Boolean → checkbox, everything else → text box;
binary fields are not bindable). Controls move by drag; **no resize
handles** (Stage 2), no z-order/tab order, no style/format property sheet.
The selected control's one content property is edited in the designer
panel (label text, button caption, bound field or subform name) and
persists in the `.datacraft` sidecar — never in the database file.

| Control | Form runtime | Report (print preview) |
|---|---|---|
| Label | static bold text | **literal** text — a `=` in a label is not evaluated |
| Text box | the current record's field value, **read-only** (M68) | the value, or a `=expression` |
| Check box | shows the bound Boolean; **clicks do nothing** | the value as text |
| Combo box | the value as text — **no dropdown** | the value as text |
| Command button | caption rendered; **clicks do nothing** (M69) | the caption as text |
| Subform | a `[subform: name]` placeholder — **not embedded** | **renders nothing** |

Report specifics: the page sequence with `group_by` is group header → one
detail page per record → group footer; footer text boxes evaluate
top-level `Sum/Count/Min/Max/Avg` over the group's rows (`Count(*)` counts
rows, the others skip Nulls), detail pages evaluate per record, and a
record-less source renders expressions empty. Form runtime record
navigation (◀/▶) works over the source rows. The two deliberate gaps are
tracked as M68 (editable bound fields — the update path exists, only the
wiring is missing) and M69 (a button action model — decision needed first;
macros/VBA stay out per ADR-2).

**Phase 64 (Access-aligned, delivered 2026-11-09).** The v0.1.0 matrix above
stays the historical release record. Phase 64 closed M68, M69, M70 and M71
under ADR-8 (§17 of `docs/ARCHITECTURE.md`), verified against the Microsoft
Access documentation: bound fields edit through a per-record buffer (`Esc`
once = field, twice = record); empty entries follow `Required`/
`AllowZeroLength`; checkboxes are two-state on Boolean (Null shows as No);
combos show every row source row including duplicates and accept free entry;
buttons carry an action (open form/report, run query, apply filter), stay
enabled without one and report precisely on click; filters are form-scoped
WHERE predicates, separate from the datasheet.

**Principles binding every stage:**

- **No affordance without feedback.** A new control ships with visible state
  feedback, a tooltip, and a test pin; where geometry matters, a
  layout_probe rect pin at both pinned window sizes (the T10 lesson:
  semantic-tree pins are not layout truth).
- **Core owns decisions.** When touched, the known UI-side decision points
  move to core: view-order mapping (`datasheet.rs`), FormN/ReportN
  auto-naming (`ribbon.rs`), multi-command flow composition (`flows.rs`).
- **Microsoft reference discipline.** UI work checks Microsoft's Access
  support documentation (`support.microsoft.com/access`) for how the
  corresponding surface is structured (flows, grouping, affordance order) —
  as an idea source only; nothing is copied (graphics, icons, screenshots,
  other assets), the visual language stays the original token system (ADR-6).
- **i18n parity for every string.** The catalog grows with each stage; the
  one known violation (hardcoded "Delete Record" confirmation text,
  `datasheet.rs`) is an immediate inline fix, not roadmap work.
- **Splits are part of the work.** Six UI files sit 15–42 lines under the
  500-line gate; datasheet/nav work will force module splits (precedent:
  `header.rs`) — planned, not discovered.
- **Windowing before scale.** Lazy row reads (core usage-map discovery)
  precede any datasheet feature that assumes large tables render fast.
- **Deliberate omissions stay deliberate.** Print/Export surface arrives
  via §11; the browser build stays a §15 non-goal.

**Hygiene notes:** eframe 0.37 is pinned per-crate, not in
`[workspace.dependencies]` — moved there on the next `Cargo.toml` touch
(Medium gate). The M48 grouping-picker bookkeeping (delivered per
PROGRESS 41.4) is reconciled: the picker, both-direction kittest pins
and the catalog entries exist since 41.4 — the item closed 2026-11-08 at
step 57.1.

**Reference-audit adaptation (2026-11-08):** the Microsoft-reference audit
(`docs/UI-REFERENCE-AUDIT.md`) compared every implemented workflow against
the Access documentation. Its §4 fix candidates — the unintended
divergences: import naming ("Import" vs Access's "Import & Link"/"New Data
Source"), the missing F11 navigation-pane toggle, First/Last
record-navigation buttons, datasheet-first editing (double-click rename,
Click to Add), pane-top sorting, Ctrl+N/O base shortcuts and the Expression
Builder embedding — are **slated for later adaptation**, not shelved: they
fold into the owning stage of this ladder when that stage starts (work
items are created at stage start, per this section's rule), rather than as
standalone items today.

## 17. Crate-split policy — decision record

**Decision (2026-11-07, human-approved):** the workspace keeps its current
eight-crate cut. Further crate splits are deliberately limited to at most two,
in this order: (1) `datacraft-expr` — the expression engine (lexer, parser,
evaluator, function library, format, like, coerce; ~3,100 lines in
`core/src/expr`) becomes its own crate, with the schema-type boundary
(`NumericKind`, field types) resolved first via a shared `datacraft-types`
crate; (2) `datacraft-query` — the SQL dialect engine, QBE evaluation and
native query execution (`core/src/native_exec`) become their own crate,
making the existing `sql-engine` feature structural. All other crates stay
as they are. Work items are created only when a released step touches the
crate in question (piggyback rule) — no dedicated split phases.

**Rationale:**

1. **Splits cost disproportionately here.** Every `Cargo.toml` change is a
   Medium release gate per occurrence; every new crate passes the dependency
   direction review (AGENTS §1) and updates ARCHITECTURE §4, AGENTS, README,
   TESTING and the crates.io publishing order (31.4). At the current size
   (~344 `.rs` files; core ≈ 18,200 lines in 78 files) the compile-time
   payoff is modest, and the 500-line gate pressure is cheaper to solve with
   module splits inside existing crates (L30) — a finer crate layering is a
   consequence of scale, not a quality trait by itself.
2. **expr is the one clean win.** Pure computation, no I/O, no backends, and
   among the most-tested domains (~17 % of core's lines). As its own crate
   it becomes a small, independently publishable library with its own semver
   story, and core shrinks accordingly. The blocker is the type boundary:
   expr consumes schema types, so the split ships with `datacraft-types`
   first — `core → expr → types → (nothing)` stays acyclic and every edge
   passes the direction review.
3. **query is the optional second split.** The SQL dialect and native
   execution already sit behind the `sql-engine` feature with optional
   rusqlite; a crate boundary makes that optionality structural and keeps
   default core dependency-free. The cut is larger (QBE, saved-query
   execution, expression integration), hence optional and second.
4. **Explicitly not split:** the command bus and dispatch (the single-bus
   backbone of AGENTS §6), the snapshot/wire contracts (one ABI, one crate),
   `datacraft-storage` (deliberately thin), `datacraft-jet` (cohesive format
   knowledge — jet3/jet4 subcrates would risk cycles over shared page
   primitives for no build win) and `datacraft-ui` (rendering glue by
   definition; module splits cover its size pressure).

**Downstream:** new crate = Medium release gate per occurrence; direction
review binding; AGENTS §1 graph updated after each split.

**Status (2026-11-14, phase 76):** both sanctioned splits are delivered.
`datacraft-types` carries the value/schema vocabulary and the shared pure
helpers, `datacraft-expr` the expression engine, `datacraft-query` the QBE
model, criteria parsing, evaluation, native execution and the shadow SQL
engine; the graph is `core → query → expr → types` (acyclic, boundary guard
green), and the `sql-engine` boundary is structural — core's feature forwards
to the query crate and is off by default, so a default-feature core never
pulls rusqlite. Core re-exports every moved public path, so no client
changed. The split cap of two (three crates including the type boundary) is
thereby reached; no further splits are planned under this record.

## 18. Performance benchmarking vs. Access — decision record

**Decision (2026-11-08, human-approved):** DataCraft measures itself against
Access with a two-family benchmark methodology, evidence-first like the
compatibility matrix. **Family A — engine-level (reproducible):** DataCraft
headless (bus/CLI) against `DAO.DBEngine.120` (the engine under Access) via
COM automation — the `tools/dao-oracle` precedent — on the same machine
against the same generated Jet4 `.mdb`. **Family B — UX/task-level:**
app-start-to-first-row, datasheet scrolling and report-preview rendering are
measured as task times with a documented methodology, never as
millisecond-precision claims. Results live in a `docs/BENCHMARKS.md` created
when the owning step starts. A mixed scoreboard is the expected, honest
outcome — the benchmark is an honesty and target-metric instrument first,
never a marketing surface.

**Measurement points (family A, with the honest expectation today):**

| Measurement | Method | Honest expectation today |
|---|---|---|
| Cold open (file → catalog/first row) | `OpenDatabase` + `inspect()` vs DAO `OpenDatabase` | DataCraft likely ahead |
| Full scan (1M rows) | `read_rows` vs a DAO `MoveNext` loop | open — warm runs are disk-I/O bound |
| Filtered query (indexed field, few hits of many) | native exec/QBE vs DAO `OpenRecordset` | **Access wins** — the full-scan cache (§16 stage 1) and the M15 index leaves are open |
| Join query | native exec vs DAO | Access wins (dialect coverage) |
| Insert throughput (single rows + journal batch) | Jet writer vs DAO `AddNew`/`Execute` | undecided — that is what the measurement is for |
| RAM footprint (large database open) | host process memory | likely DataCraft |
| Datasheet scroll at 1M rows | our frame time vs the 60 FPS design target; the Access side as task time | measured against our own design target, not a direct duel |
| Compact | DAO `CompactDatabase` vs — | honest gap: §10 compact does not exist yet |

**Method controls.** The Access build number, Windows build, hardware and
power plan travel with every number; Name AutoCorrect is off; the
Defender/indexing handling for the scratch directory is documented; the warm
file cache is the documented default (cold is not cleanly enforceable on
Windows); N ≥ 10 runs, median + spread; one identical Jet4 `.mdb` per run
(our verified format); size classes 10k/100k/1M rows × narrow/wide.

**Implementation strategy.** Fixtures are generated deterministically at
bench time (the dao-oracle `new-db`/`exec` pattern and the SQLite
scratch-path pattern) — never committed, so no fixture gates apply; only the
generator recipe is committed. The Access side extends `tools/dao-oracle/`
with a timing harness; the DataCraft side times bus/CLI calls with
`std::time::Instant` — **no criterion dependency** (a new dependency is a
Medium gate and buys nothing here). Benchmarks never run in CI (runner
variance); CI at most smoke-checks the generator. Clean-room discipline is
untouched: timing is own black-box observation, the same rule the
dao-oracle README already states.

**Claims hygiene.** Every number carries its evidence (command, hardware,
date, versions) in the compatibility-matrix style — no "x times faster"
headlines. The README "Fast" bullet (60 FPS with million-row scrolling as a
design target) becomes a measured claim only when the harness supports it.
The measurement points also serve as the target-metric source for §16
stage-1 windowed row reads and the §6 M15 index-page work — they define what
those items have to deliver, without re-ordering them.

**Gates:** work items are created only when the owning phase starts
(`BACKLOG.md`, L-series) and implementation starts only after the human
release; this docs-level decision itself is Low.

## 19. SQLite depth — decision record

**Decision (2026-11-12, human-approved):** SQLite is a first-class source,
not only a test driver. The depth work proceeds in stages, each gated by its
own items:

| Stage | Content | Notes |
|---|---|---|
| 0 — Visibility (delivered 2026-11-12, phase 73) | Views as read-only objects; index/trigger inventory; file facts (`sqlite_version`, `page_size`, `journal_mode`, `encoding`, `user_version`, `busy_timeout`) and a bounded `quick_check` at open; robust open (`busy_timeout`, WAL fact) and `WITHOUT ROWID`/`STRICT` honesty. | M90–M92 delivered; a damaged file opens with a precise `SourceHealthWarning`. |
| 1 — SQL pushdown | The SQL window runs read-only SQL directly against a SQLite source (full dialect, index use, views, `EXPLAIN QUERY PLAN`); the shadow materialization stays for Jet/CSV. | Writes stay on the command bus. |
| 2 — Write parity | SQLite `alter_table` (ADD/RENAME natively, DROP via rebuild), `atomic_batch` via one transaction, index management (list/create/drop). | The capability flags flip to an honest `Yes`. |
| 3 — Conversion & templates | Cross-format conversion Jet4/ACE ↔ SQLite (schema, data, relationships; the sidecar follows the name), SQLite templates alongside the Jet4 ones. | The Jet writer is the conversion target and matures through real use. |

**Out of scope, stated plainly:** SQLCipher/encryption (a new C dependency,
contrary to the pure-Rust stance); trigger creation or any trigger logic
(inventory only — writing into a foreign database fires its triggers, which
is database semantics and belongs in the inventory notice, not a DataCraft
execution path); `ATTACH`/multi-database work until a real need exists.

**Cross-format effects (Jet4/ACE):** Stage 3 benefits the Jet writer directly
(it becomes the migration target and is exercised by real conversions);
Stages 0–2 contribute the shared port vocabulary (schema inventory, health
facts) and the same capability/contract suites over both backends, which
hardens the Jet contracts indirectly. The SQLite-specific surfaces (views,
PRAGMAs, WAL, SQL pushdown) do not change the Jet read/write path. Shared
parts belong on the `StoragePort`/snapshot, never as SQLite special cases in
the UI.

**Gates:** work items are created when the owning stage starts (stage 0 is
filed as M90–M92); new dependencies (e.g. an FTS/JSON extension decision) are
Medium gates; nothing executes SQL provided by a database file without an
explicit user action.

**View designs (2026-11-12, human-approved):** a view is a named `SELECT`
stored as a sidecar design object (`views`), independent of the format.
Execution follows the sink/source rule: on SQLite the view is applied
natively (`CREATE VIEW`) and executed by SQLite; on Jet/ACE/CSV DataCraft
executes the stored `SELECT` through the shadow SQL engine. The UI presents
views as their own nav-pane group with a read-only datasheet; the sidecar
stays the design authority, the file the execution authority where the
format has one. Native SQLite views are additionally visible read-only
(M90).

**Trigger model (deferred, decided 2026-11-12):** a trigger is an
event-driven DML program; executing it inside DataCraft means owning its
semantics. The model therefore fixes a strict subset before any code:
`AFTER INSERT/UPDATE/DELETE` on DataCraft writes, `OLD`/`NEW` row images,
`WHEN` through the Ring-1 expression engine, one DML statement per trigger
(no DDL, no recursion), explicit deterministic ordering, atomicity inside
the same journal/batch unit, and no non-deterministic statements. Mapping:
SQLite compiles the subset to native `CREATE TRIGGER`; Jet/ACE/CSV are
interpreted by DataCraft — with the documented island caveat that sidecar
triggers fire only on DataCraft writes. Not started; its own phase once the
view work has landed.

## 20. Access form/report migration — Route B and the Route A fallback

**Route B (decided 2026-11-14, ADR-9 in `docs/ARCHITECTURE.md` §18;
delivered in Phase 78):** a separate, optional tool crate
(`datacraft-import`) links `jetdb` 0.4 (MIT OR Apache-2.0, pure Rust, no
COM/FFI) and maps native Jet4/ACE form/report designs into `.datacraft`
sidecar designs. It is not part of the UI/CLI/MCP composition, never writes
the database, and carries the third-party knowledge openly
(`THIRD-PARTY-NOTICES.md`, README). The engine stays clean-room.
Feasibility evidence: 77.1 — jetdb 0.4.0 reproduced the committed fixture's
object/control inventory exactly, with 61/75 control properties exact and
the 14 omissions all reconstructible Access defaults. Phase 78 ships the
tool with the six-kind/defaults fixture (`fixtures/formDesignsV2016.accdb`),
the exact expected-sidecar contract and the CLI/merge suite. Known
first-cut limitations: combo row sources, controls whose geometry Access
stored only as a form default (reported, not guessed), and Access 97
(Jet3) designs.

**Route A — own decoder (fallback, fully specified).** Replace the jetdb
adapter with a DataCraft-owned decoder so the migration path itself carries
no third-party reverse-engineering knowledge. This is the fallback if jetdb
is abandoned, changes license, produces a security finding, or if the
clean-room claim must extend to the migration path.

- **Evidence base.** The M89 container/blob findings (MSysAccessStorage
  directory, `Blob`/`TypeInfo`/`PropData` streams, UTF-16 name framing, the
  Access 2000 `0x13` and 2002+ `0x14/0x15` blob versions) plus the step-77.1
  comparison; the committed SaveAsText exports remain the ground truth.
- **Fixture generation.** Extend `tools/fixture-build` with a VBA generator
  (the Access COM recipe of step 37.3) that creates a fresh database with one
  form/report per supported control kind, every supported property set to a
  distinctive value, and a "defaults" object whose controls are freshly
  placed and untouched. Pin the new fixture (`fixtures/formDesignsV2016.accdb`
  or the version the human Access writes) with a manifest hash and provenance
  in `docs/FIXTURE-SOURCES.md`; the committed programmability fixture stays
  untouched.
- **Table derivation.** Match every property ID to the SaveAsText key that
  holds its value (the jetdb technique, but applied to our own generated
  fixtures), derive the control-type codes and the per-control-type default
  table from the defaults object, and record the tables in `docs/FORMAT-JET.md`.
  Unknown IDs become precise unsupported entries, never guesses.
- **Decoder.** A strict reader for the blob tree (trailer 253/254/255,
  sections and controls, property records with type tags and lengths) and for
  the Access 2000 tree, with checked ranges and precise errors on hostile
  input (the H46 pattern). Reuse `datacraft-types` only for value
  vocabulary if needed; no new dependencies. The same reader can consume the
  form's **default-control nodes** — which jetdb's public API drops — and
  thereby reconstruct controls whose geometry Access omitted, the one known
  limitation of Route B.
- **Parity.** Put both backends behind one mapping trait so Route A and
  Route B produce identical sidecars and unsupported reports; the expected
  sidecar pair from 77.3 and the generated fixtures are the contract tests.
  jetdb stays a test-time CLI oracle only (`DATACRAFT_ORACLE_JETDB`, the
  libmdb/L31 pattern) — never linked into the product.
- **Effort and prerequisites.** Realistically one phase of 5–6 steps; needs
  Access on the fixture-generation machine (the existing COM builder
  precedent), no new dependencies. The property surface for the six supported
  controls is small (Name, Left/Top/Width/Height, Caption, ControlSource,
  RowSource, SourceObject, section heights), so the derivation is bounded.
- **Trigger.** Do not start Route A speculatively. Start it when the fallback
  trigger fires or when a phase is explicitly released for it.
- **Out of scope in both routes.** Access 97 form/report designs, event/VBA
  binding, combo row-source/display-field fidelity beyond the documented
  first-cut limitation, and byte-identical Access round-trips (the sidecar is
  an approximate redesign, ADR-1).

Cross-reference: the backlog item is M88; the feasibility evidence is
`docs/FORMAT-JET.md` "M88 feasibility — jetdb against the committed exports".
