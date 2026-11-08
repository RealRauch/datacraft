# UI reference audit — implemented workflows vs. the Microsoft Access documentation

> Date: 2026-11-08. First application of the UI reference discipline
> (`AGENTS.md` §6, `docs/ARCHITECTURE.md` §12, `docs/ROADMAP.md` §16).
> Scope: the workflows DataCraft **implements today** are compared against
> the structure Microsoft documents for the Access desktop UI. Deliberately
> open workflows are **listed only** (§5) — no feature proposals, no backlog
> items filed (human decision 2026-11-08).
> Discipline: structure facts are restated in our own words; nothing is
> copied — no Microsoft graphics, icons, screenshots or long text passages
> (ARCHITECTURE §12). Only functional names and structural facts are used.

## 1. Method and sources

Reference side (fetched 2026-11-08):

- [Basic tasks for an Access desktop database](https://support.microsoft.com/en-gb/access/basic-tasks-for-an-access-desktop-database)
  — the documented top-level workflow: create/open a database, add a table,
  edit it datasheet-first, import or link data, analyze, then queries/forms/
  reports as next steps.
- [Use the Navigation Pane](https://support.microsoft.com/en-us/access/use-the-navigation-pane)
  — categories, groups, context-menu commands, sorting, search, hidden
  objects.
- [Show or hide the Navigation Pane](https://support.microsoft.com/en-us/access/show-or-hide-the-navigation-pane-in-access)
  — the F11 toggle and the shutter-bar affordance.

Implementation side: the core-owned ribbon spec
(`crates/datacraft-core/src/ribbon.rs`), the backstage modules
(`crates/datacraft-ui/src/backstage.rs` and `backstage/`), the navigation
pane (`crates/datacraft-ui/src/nav_pane.rs`), key-tip derivation
(`crates/datacraft-core/src/keys.rs`), the datasheet and designer modules,
and the German catalog (`crates/datacraft-ui/src/i18n/de/`). Each verdict
names its evidence.

## 2. Conform — implemented workflows that match the documented Access structure

| # | Workflow | Our implementation | Evidence |
| --- | --- | --- | --- |
| 1 | Ribbon tab set and order: File, Home, Create, External Data, Database Tools | Identical tab set and order; File opens the backstage | `ribbon.rs:109-227` |
| 2 | Create tab group structure: Tables / Queries / Forms / Reports with the commands Table, Table Design, Query Design, Form Design, Report Design | Same groups, same command names, same order | `ribbon.rs:133-181` |
| 3 | Navigation pane as the object surface with per-kind groups and a context menu (Open, Design, Delete) plus group sorting by name/type | Same model; the category dropdown exists with the single "All Access Objects" entry (the Access "Object Type" equivalent) | `nav_pane.rs:35-180` |
| 4 | Views split-button owning the only mode switch, on Home and on every contextual tab | Same pattern; Table/Query/Form/Report Tools contextual tabs each carry the shared Views group | `ribbon.rs:83-105`, step 21.1 (U5) |
| 5 | Backstage (File) pages as the file lifecycle surface: Home/New/Open/Save/Info/Options | Same page set plus our own Samples page (deliberate, §3); Excel-style sidebar with the bottom-pinned Options entry | `backstage.rs:26-54`, step 38.2 (M41) |
| 6 | Query design grid rows (table, field, sort, show, criteria) | Same row model in the QBE grid, schema-whitelisted, sidecar-persisted | README QBE row, phase 7/15 |
| 7 | Keyboard conventions: save, close, escape; Alt key tips derived from the spec | Ctrl+S / Ctrl+W / Esc + Alt key tips with deterministic derivation | `keys.rs`, step 21.8 (U12) |
| 8 | Terminology in English and German | "Navigation Pane"/"Navigationsbereich", "Table Design"/"Tabellenentwurf" (the German Access name, step 44.4 precedent), "Formularentwurf", "Berichtsentwurf", "Abfrageentwurf", "Datenbanktools", "Externe Daten" | `i18n/de/ui_chrome.rs`, `views.rs` |

## 3. Deliberate divergences (documented decisions, not defects)

| # | Access structure | Our structure | Why |
| --- | --- | --- | --- |
| 1 | Microsoft's visual language, icons and artwork | Original token system and vector iconography | ADR-6 and ARCHITECTURE §12: nothing copied; the look is ours |
| 2 | Template gallery (online templates, commercial templates) | Own deterministic template registry on the Samples/New pages | Shipped, hash-pinned samples; no online dependency (phase 29, H44) |
| 3 | Save persists the whole application state (Access prompts to save) | Ctrl+S/Save persists the `.datacraft` design sidecar; database writes go through the port immediately (write-through) | ADR-1 write model: the file is never silently modified by editing affordances; writes are explicit commands |
| 4 | Fixed Access-key letters for key tips | Key tips derived from action ids (first character, collision-numbered) | Deterministic, spec-derived, test-shared (`keys.rs` doc comment) |
| 5 | Layout View as a third form/report view | Design view plus the runtime; no layout view | Runtime scope ring still open (ROADMAP §5 M4); not a UI defect |
| 6 | Security-warning bar ("Enable Content") for macros/VBA | No such bar; macros/VBA are read-only inventory and extraction | ADR-2: no execution, so no enable-content concept exists |
| 7 | View-shortcut icons in the status bar's right edge | View switching only through the Views split-button | Single mode-switch affordance by design (step 21.1) |

## 4. Unintended divergences (fix candidates — listed only, not filed)

Small structural gaps against the documented Access flows, found by the
audit. Per the human decision of 2026-11-08 these are **not** filed as
backlog items yet; this section is the candidate list.

1. **External Data naming.** Access documents the group as "Import & Link"
   with the "New Data Source" command per format; our group is "Import" and
   the command is "Import File" (`ribbon.rs:183-195`). The link half of the
   workflow does not exist yet (§5, item 3), which explains but does not
   justify the naming drift.
2. **F11 navigation-pane toggle.** Access toggles the navigation pane with
   F11 (plus its shutter-bar button); we offer only the Home ▸ View ▸
   Navigation Pane toggle. No F11 binding exists (`keys.rs` derives key tips
   only; the shortcut set is Ctrl+S/Ctrl+W/Esc).
3. **Record navigation lacks First/Last buttons.** Access's navigator
   carries First/Previous/Next/Last; ours has previous/next buttons and the
   Home/End keyboard path (steps 40.1/45.2) but no First/Last buttons.
4. **Datasheet-first table editing.** Access's documented "add a table"
   flow renames a column by double-clicking the header and adds fields via
   a "Click to Add" trailing column; our datasheet offers neither — renaming
   runs through the design view only (steps 40.4/24.3).
5. **Pane-top sorting.** Access carries "Sort By" at the pane's top-level
   context menu; ours puts Sort by Name/Type on the group-header context
   menu (`nav_pane.rs:159-177`). Same vocabulary, different anchor.
6. **Base shortcuts Ctrl+N / Ctrl+O.** Access binds new-database and
   open-database to Ctrl+N/Ctrl+O; we do not bind either (open/new live in
   the backstage only).
7. **Expression Builder embedding.** Access embeds the Expression Builder
   into the context it serves (control sources, criteria cells); our
   Expression window is standalone without a row context — the row-context
   follow-up is already open as BACKLOG L27, the embedding itself is the
   structural divergence this audit records.

## 5. Open workflows — listed only (no proposals)

Access workflows Microsoft documents that DataCraft does not implement.
Where a roadmap decision record already owns the topic, it is named; the
rest are named without a home, to be triaged when relevant.

- **Export** (CSV/JSON/xlsx/ODS, PDF from report preview, print) —
  ROADMAP §11 export matrix.
- **Compact & Repair, salvage reader, pre-open health check** — ROADMAP §10.
- **Linked tables and server sources** (import-or-link per source, Linked
  Table Manager, SQL Server/ODBC) — ROADMAP §9 stage ladder; SQL Server is
  explicitly gated there.
- **Migration assessment** — ROADMAP §14.
- **Excel/ODS import formats** — ROADMAP §7/§8.
- **Datasheet completeness**: multi-select, cell copy/paste, find/replace,
  column drag-reorder, null markers — ROADMAP §16 stage 1.
- **Designer parity**: control property sheet, resize handles, z-order,
  preview zoom — ROADMAP §16 stage 2.
- **Navigation pane completeness**: search bar, hidden objects, further
  categories (related views, dates, custom), View By details/icon/list —
  ROADMAP §16 stage 3.
- **Command palette, QAT customization** — ROADMAP §16 stage 4.
- **No roadmap home yet, named for triage**: Table Analyzer (Database
  Tools ▸ Analyze Table), form/report wizards (the one-click bound-object
  flows Access documents next to the design entries), Object Dependencies.
- **In flight, not audited as complete**: the Import File flow itself —
  the write chain landed (M50, step 58.1 commits) and the UI is landing;
  it is audited here only as far as the ribbon surface exists
  (`ribbon.rs:188-195`).

## 6. Vocabulary check

| DataCraft label | Access documented name | Verdict |
| --- | --- | --- |
| Navigation Pane | Navigation Pane | match |
| Views | Views | match |
| Table / Table Design | Table / Table Design | match |
| Query Design | Query Design | match |
| Form Design | Form Design | match (the one-click Form wizard has no equivalent yet — §5) |
| Report Design | Report Design | match (wizard — §5) |
| Relationships | Relationships | match |
| Database Tools | Database Tools | match |
| External Data | External Data | match |
| Import File | New Data Source (group "Import & Link") | divergent naming (§4.1) |
| SQL | SQL View (inside query design) | divergent placement/naming |
| Expression | Expression Builder | divergent naming (§4.7) |
| Samples | (no Access equivalent) | our own surface (§3.2) |

German catalog convergence to German Access holds where the audit checked:
"Navigationsbereich", "Tabellenentwurf", "Abfrageentwurf",
"Formularentwurf", "Berichtsentwurf", "Löschen", "Entwurf" — the step-44.4
precedent ("Tabellenentwurf" from German Access's create action) generalizes.

## 7. Verdict

The implemented workflow set follows the documented Access structure closely
where it exists: tab order, create-group structure, navigation-pane model,
views switcher, backstage shape and the EN/DE vocabulary all conform. The
divergences found are either deliberate and documented (§3) or small naming/
affordance gaps (§4) that can be triaged item by item; no implemented
workflow contradicts the documented Access structure in its fundamental
shape. The audit repeats whenever a ROADMAP §16 stage lands — the same
method, against the same sources, updated with the new date.
