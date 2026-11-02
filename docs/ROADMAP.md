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
| 0 | Storage: Jet4 read → ACE read → Jet4 write → ACE write | — | high; spec + oracle driven | Phase 2, H9 (write), L1 (Jet3) |
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
| M0 | Storage read milestones (Jet4 → ACE) | everything | ✅ Jet4 (Phase 2), Jet3 (Phase 10), ACE 2007/2010/2016 (Phase 14, oracle-checked); SQLite sources (Phase 13); ACCDB write → Phase 25 (planned) |
| M1 | Expression service (Ring 1) | queries, macros, forms, validation | ✅ Phase 15 (steps 15.1–15.2, H41 archived; `Now`/`Date` deliberately absent — deterministic) |
| M2 | Query engine + QBE (Ring 2) | real app behavior | ✅ Phase 7 · native query execution → Phase 27 (planned, M12) |
| M3 | Write path (Ring 0 write) | editing apps, Access-compatible write goal | ✅ Jet4 writer landed in Phase 10 (create + insert); update/delete + journal profiles landed in Phase 11 (steps 11.10–11.11); verified claim set in the README matrix |
| M4 | Forms/Reports runtime (Ring 4) | the Access UX itself | ✅ Phase 8 (forms) + Phase 10 (report designer + print preview) + Phase 15 (grouped/aggregate bands) |
| M5 | Macros (Ring 3) | unattended app logic | ✅ Phase 17, step 17.1: read-only inventory (name + coarse arguments); DoCmd execution stays out of scope |
| M6 | VBA extraction + viewer + static analysis | migration tooling, fidelity | ✅ Phase 17, steps 17.2–17.3 (extraction, viewer, `AnalyzeVba`; no execution) |
| M7 | `ScriptEngine` trait hook in core (unimplemented) + engine decision | stage-2 scripting when the §4 gate opens | ⬜ gated (Ring 4 + demand) |
| — | VBA execution | — | ⛔ out of scope (see §3 gate) |

Ordering notes:

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
