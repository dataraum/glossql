# The review's fixes — what landed, and what was left alone

Date: 2026-08-06, the same day as the adversarial review
(`2026-08-06-adversarial-review.md`). Every fix below was written
against the reproduction that found the defect, and every one of those
reproductions was re-run against the fixed build on a throwaway
workspace. Workspace suite: 120 tests, green. Clippy: clean.

## The verdict key — ruled, then landed

**The ruling (project lead, 2026-08-06): the witness joins the cache
key.** A verdict stays a cached function value — one relation, one
forcing verb (`DELETE FROM cache`) — but the row now names the seat it
was computed for. `witness` is NULL for a function's own output, which
is legitimately keyed by subject alone; it carries the witness name for
a detector's verdict, which depends on that witness's aspect, threshold
and slots.

Landed as: a `witness` column on `cache` (added to existing stores by
ALTER, with the un-attributable old verdicts deleted — they recompute at
the next read, which is what detector-at-read already promises), the
column threaded through `cache_get`/`cache_put`/`latest_cache`, function
voices reading `witness IS NULL` so a verdict never enters the slots,
and `ensure_verdicts` computing per witness. The `cache` relation
discloses the column, so an agent can see which seat a row answers for.
SPEC §6 and §7.2 carry the clause.

Before, on the live server: `alpha` (contested) and `beta` (one slot),
both witnessed by the same detector, both reported red/1.0, and `beta`'s
undisputed value was withheld as contested. After: `alpha` red/1.0 and
contested, `beta` green/0.0 and current with its value served. The
regression test is `witnesses_sharing_a_detector_hold_their_own_verdicts`
in `crates/session/tests/flows.rs`, with a fake detector that reads its
context rather than returning a constant — so it fails if the key ever
narrows again.

## The doors

**Forwarded deletes.** `store_delete` now renders from the AST with
dollar-quoted literals normalized to single quotes — the value is
identical, and single quotes escape and tokenize the same way in both
sqlparser and SQLite, which is what makes the round trip safe. At the
point of execution, `forward_delete` refuses any text carrying a `;` or
`$` outside a quoted literal. The payload that created a table inside
the store and promoted an agent's gloss to human rank now runs as what
it always claimed to be: a string nobody's body matches. A legitimate
dollar-quoted body still deletes.

**Probe and recipe bodies** run under `SQLOptions` with DDL, DML and
statements refused, as does every `db.query()` a script issues. The
`COPY … TO` that wrote a parquet file outside the workspace now answers
"DML not supported: COPY" and writes nothing.

**`SELECT … INTO`** is refused by the allowlist — it parsed as a Query
and planned as a `CREATE MEMORY TABLE`. The dead `Explain` arm went with
it: `DFParser` intercepts EXPLAIN before the allowlist ever sees it, so
the arm was unreachable (and, had it been reachable, `EXPLAIN ANALYZE`
executes its inner statement).

**The source-root fence** now resolves symlinks: `..` was not the only
way out.

## Ordering, keys and guards

- **Re-land runs the new recipe first.** The old landing is dropped only
  once the replacement has produced batches; a recipe that errors leaves
  the table it was replacing intact. Tested with a typo'd column.
- **The recipe row lands after the landing does** (`recipe_admission`
  decides, `put_recipe` records). A failed materialization no longer
  leaves a row that answers `unchanged` over an empty table.
- **`Scope::predicate` escapes LIKE metacharacters.** `order_items` no
  longer sweeps `orderxitems`'s cached evidence, and a subject carrying
  `%` no longer wipes a dataset.
- **A function may not ACCEPT the aspect it RETURNS** — the self-edge
  invalidated its own row inside `cache_put`, and the read that followed
  panicked. Refused at declaration; the read path returns an error
  instead of an `expect`.
- **A table may not take a store relation's name** (`sources`,
  `imports`, …), which would shadow the relation under the bare name.
- **Aspect re-declaration counts cached function values**, not only
  glosses — and a MEASUREMENT aspect can hold nothing else, so its
  schema could previously change under values validated against the old
  one.
- **A NaN abstains in `classify_series`** instead of slipping past every
  gate (`NaN > x` is false) and panicking the median sort.

## Performance

- **WAL and a busy timeout** on the store: sqlx sets neither, so every
  gloss was a journal-mode commit that blocked readers.
- **Two indexes** — `glossary (dataset, subject, aspect, actor_kind,
  id)` and `cache (dataset, subject, function, witness, id)` — under the
  `NOT EXISTS` supersession predicate that reads every collapse. They are
  created *after* the column migrations, not with them: an index over a
  column an older store has not been widened with yet fails the whole
  migration, which is how the ordering was found — by opening a copy of
  the finance workspace's store rather than only fresh ones.
- **The execute path streams to the cap.** `Session::substrate` and
  `run_probe` stop one row past the door's cap instead of collecting
  everything and trimming at render. The cap is pushed down from the
  door through the plane, so a probe without a LIMIT no longer pulls a
  whole source into memory to show 200 rows of it.

## The migration, on a real store

Verified against a copy of `~/glossql-ws-fin/glossary.sqlite`: the
`witness` column is added, the 58 un-attributable detector verdicts go
(153 cache rows to 95, none of them a detector's), the 225 glosses are
untouched, WAL is on and both indexes exist. The next read recomputes
what was dropped. A running serverd on an older binary keeps its old
schema until it restarts.

## Left alone, deliberately

**A declared source can name any location the process can read.** That
is what a source *is* — the finance and RelBench workspaces read from
outside the workspace by design — so fencing locations to the workspace
would break the actual flow. Reading files at a declared location is not
an escape; writing anywhere was, and that is closed. Who may declare a
source is governance, which is held open.

**No authentication.** Held open by design; every door finding is
reachable by anything that can open the socket, and `/query` speaks as
the configured human actor, so door choice is rank choice. Worth
restating whenever the PoC moves off loopback.

**No request timeouts, no session eviction.** One query with a cartesian
join still pins a core, and the actor plane still grows one session per
distinct client name. Both are bounded in practice at PoC scale and
neither is a correctness defect.
