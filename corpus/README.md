# glossql transcription corpus

Each file pairs a **real artifact** from `../dataraum-context` (quoted, with path)
with its glossql transcription attempt per SPEC.md §3. These are test fixtures —
the §9.1 evidence base — not design docs.

Block tags, enforced by `harness/check.py`:

- ` ```glossql ` — must parse under `grammar.ebnf`. A failure is a regression.
- ` ```glossql-gap ` — invented syntax documenting a grammar gap; must **fail**
  to parse. When the grammar gains the form, the checker flags "gap closed" and
  the tag flips to ` ```glossql `.

Verdict vocabulary (§9.1 buckets, refined): TRANSCRIBES CLEANLY · GRAMMAR GAP ·
SEMANTICS UNDEFINED · INFORMATION LOST. One fixture may carry several, per field.

Sources snapshot 2026-07-30. Full analysis: `reports/2026-07-30-adversarial-review.md`.

| # | fixture | verdict (dominant) |
|---|---|---|
| 01 | concept `revenue` | TRANSCRIBES CLEANLY |
| 02 | convention `sign_natural_balance` | GRAMMAR GAP |
| 03 | metric `dso` | GRAMMAR GAP |
| 04 | validation `trial_balance` | GRAMMAR GAP |
| 05 | cycle `accounts_receivable` | GRAMMAR GAP (heaviest) |
| 06 | claim witnesses + reliabilities | RESOLVED (sprint 1, fork B) |
| 07 | grounding / sql_snippets | GRAMMAR GAP (key) |
| 08 | teach payloads (8 types) | mixed |
| 09 | answer-agent served context | SEMANTICS UNDEFINED |
