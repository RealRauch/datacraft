# LESSONS.md — Lessons learned as a review checklist (project-independent)

> Every rule originated from a **real, found bug** — the reference in brackets
> points to the backlog archive of the origin project (evidence, not prerequisite).
>
> **Usage:** before every review, before every phase completion and when creating new
> routes/endpoints/caches, work through this checklist. Goal: never make the same mistake twice.

## A. Security & tenant separation

1. **Parameterized values do not protect the keys.** Dynamic SQL (SET/ORDER-BY over object keys) needs a **column whitelist**. *(Evidence: K1)*
   - Check: every repo update that interpolates body keys into SQL → whitelist or 400.
2. **Every `:id` route checks ownership, not only the role.** Pattern: load object → compare owner → otherwise 403/404. Applies to sub-resources too. *(K2)*
   - Check: test matrix "foreign tenant → 403/404" for every `:id` route including sub-resources.
3. **Protection by registration order is not protection.** A route that is only secure because another handler was registered earlier is a latent gap — guard at the resource itself, prove independence with a test. *(H2)*
4. **Secrets fail hard, not with a warning.** In production with default secrets/passwords: process abort with a clear message; `${VAR:?…}` instead of `:-default` in compose. *(K4)*
5. **Public auth endpoints get throttled.** Login, recovery, anonymous creation: simple rate limits — otherwise large keyspaces and partial hashes are no time problem for attackers. *(H3)*

## B. Tests that find something

6. **Tests must not mirror their own implementation.** "Smooth" fixtures confirm the code instead of checking it. Always include **realistic/evil** variants: mixed spellings, foreign keys, giant payloads, foreign tenant IDs. *(K3, K1, K2 — simple tests would have found all three)*
7. **Secure contracts at the test seam, from both sides.** Where two layers share the same convention (code case, lengths, payload fields), there must be a test on **both** sides with a **shared** fixture source — otherwise one side "guesses" the other. *(K3, T3)*
8. **In-memory clients (inject/mocks) hide host and environment logic.** IP/host-based resolution looks correct in the integration test and breaks in real HTTP — always test host logic with real requests. *(Phase-4 finding: 127.0.0.1 as "subdomain")*
9. **Silent catches are diagnostic poison.** `catch(() => {})` without logging turns errors into "mysteriously does not work". Minimum: a warn log with context. *(SW registration finding)*

## C. Frontend & browser reality

10. **URL-keyed caches must not store personalized responses.** If a response contains user-specific data, a service worker must not cache it under the bare URL — extend the key with the session or split the response. *(H1)*
11. **The router does not remount on param change.** `useRef(searchParams.get(…))` freezes the first value — keep refs in sync or set remount component boundaries. *(M2)*
12. **Plan for device reality.** iOS needs `requestPermission()` inside a user gesture; headless browsers sometimes do not know service workers. Always build device features with a permission flow + fallback; E2E with a suitable browser channel. *(M4, E2E experience)*
13. **Environment-dependent values (build IDs, paths) must be deterministic OR fail hard.** Silent fallbacks ("package version") lead to old bugs that never surface. Pattern: allow an env override, but indeterminacy = build error. *(M5, phase-5 finding)*

## D. Build & process

14. **Bundler `define` values are expressions, not values.** `JSON.stringify` exactly once for strings/arrays — double stringify produces strings instead of arrays and fails silently at runtime. *(SW `addAll(String)` finding)*
15. **Security self-review before "done".** Deliberately walk through your own attack surface: "What can a *different* tenant do? An unauthenticated one? An admin with an evil body?" — the regular test suite mostly checks happy paths. *(R1 protocol)*
16. **Mechanical mass replacements with placeholders are dangerous.** Replace only with exact, unique strings; placeholders must never collide with real content; always run a diff/anomaly check afterwards. *(Evidence: archival slip 09/2026 — a `.Replace('X', …)` destroyed every "X" in three files)*
17. **Structure operations belong to the tool.** Status maintenance in the progress table, detail-block, Done-Index and archive changes always run through the tools built for it (`progress_update`, `archive_item`) — manual edits to these structures create exactly the drift the tools are meant to prevent. Only the prose that the tool deliberately does not manage (goal/acceptance texts, item content, new BACKLOG items) is edited directly in the files. *(Evidence: method-docs 09/2026 — status edits by hand in PROGRESS.md despite available tools, uncovered at the next validate.)*
18. **Measure exit codes in the host language.** PowerShell `$LASTEXITCODE`, POSIX `$?`/`$status` — never cmd `%ERRORLEVEL%` expansion after `&` (expanded at parse time, shows the value from before the run); inside cmd itself only with `!ERRORLEVEL!` under `/v:on`. *(Evidence: R9 — method-docs 09/2026, false alarm 🔴 → measurement artefact)*
