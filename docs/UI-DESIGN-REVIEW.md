# UI design review — density, typography, spacing (L47)

> Protocol for the human's live review of the running build (phase 89,
> step 89.4, item L47). The review itself changes no code: every accepted
> finding becomes its own `BACKLOG.md` item with location and acceptance,
> and the fix lands in its own released step. Screenshots are optional but
> helpful; "before" captures stay in this log.
>
> Reference discipline: Microsoft's Access support documentation
> (`support.microsoft.com/access`) is the structural idea source — nothing
> is copied, the visual language stays the original token system (ADR-6).
> The token system itself is kept (ARCHITECTURE §12); this pass tunes
> density/typography/spacing within it. No framework or shell rewrite.

## Review conventions

- **Build under review:** current `main` at review start
  (`127bb88` + the phase 89 decision records), release build,
  default window size (1280×800) and the pinned minimum (960×600)
  both checked where layout is affected.
- **Sample data:** the shipped `bike_shop` sample (tables, form, report)
  or the reviewer's own database — whichever exercises the surface.
- **The three questions per surface** (Access density conventions):
  1. **Density** — is the information per screen right? Too airy, too
     cramped, wasted rows, misgrouped controls?
  2. **Typography** — do the type rungs (Inter scale) and weights read
     correctly? Hierarchy of labels vs values vs chrome?
  3. **Spacing** — alignment, padding, gaps: does anything sit oddly,
     overlap, or drift from the grid?
- **One finding per row:** surface, location (what exactly), which of the
  three questions, severity (annoyance / inconsistency / broken feel),
  optional screenshot. Vague impressions ("looks off") are refined on the
  spot or dropped — a finding must be actionable into a BACKLOG item.
- **Verdict per finding:** accepted → BACKLOG item with acceptance;
  rejected → stays in this log with the reason (nothing is lost).

## Surface checklist

| # | Surface | Reviewed | Findings |
|---|---------|----------|----------|
| 1 | Ribbon + Quick Access Toolbar | ⬜ | — |
| 2 | Navigation pane | ⬜ | — |
| 3 | Datasheet (table view) | ⬜ | — |
| 4 | Backstage (File tab) | ⬜ | — |
| 5 | Tool windows (SQL, Expression, Relationships, Import, Notices, VBA viewer) | ⬜ | — |
| 6 | Form designer + form runtime | ⬜ | — |
| 7 | Report designer + print preview | ⬜ | — |

## Findings log

> Appended in review order; one table row per finding. BACKLOG item ids are
> assigned when a finding is accepted.

| # | Surface / location | Question (density/typography/spacing) | Finding | Severity | Verdict | Item |
|---|--------------------|----------------------------------------|---------|----------|---------|------|
| # | Surface / location | Question (density/typography/spacing) | Finding | Severity | Verdict | Item |
|---|--------------------|----------------------------------------|---------|----------|---------|------|
| 1 | product-wide — UI (backstage Info), CLI, MCP/JSON-RPC | none of the three — missing affordance | No version display anywhere: the CLI has no `--version` (archived L34 already noted the flag "does not exist in the CLI vocabulary"), the command vocabulary has no About/Version command, and the backstage Info page shows no version. A user or agent cannot tell which build they are talking to. | inconsistency (agent-facing gap) | accepted → item | M109 |
| 2 | tool windows — SQL window (reviewed out of sequence, reviewer's screenshot) | spacing / layout bug | Running any statement grows the window animated toward full screen height — "as if the query were still producing rows" (screenshot: 4 result rows, window fills the screen). Root cause located: the shared grid's `ScrollArea::both().auto_shrink([false, false])` (`datasheet/grid.rs`, correct inside the datasheet CentralPanel) is embedded by `sql_window.rs` without a height budget, so window auto-size and the all-available-space claim feed each other while `show_rows` supplies the virtual row-count height; egui's window-size interpolation makes the growth visible. | broken feel | accepted → item | M110 |
