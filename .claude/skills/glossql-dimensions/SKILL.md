---
name: glossql-dimensions
description: The dimensional read of a glossql dataset — score slice axes with dimension_relevance, judge hierarchy candidates from detect_hierarchies, and build grain-checked enriched views. Use after tables are glossed and relationships declared, before cross-table analysis or metric work.
---

# The dimensions deliverable

Three parts, one judging discipline: which axes slice the data
(inventory + relevance), how they nest (hierarchies), and the enriched
view that puts judged dimensions beside each fact. Run it after the
add-source flow (roles glossed) and the relationships plane (edges
declared). The workspace ships both measurements at boot.

## 1. Frame the vocabulary

The verdict aspect is yours to declare, like `entity`:

```glossql
DECLARE ASPECT dimension WITH $${
  "type": "object", "required": ["value"],
  "properties": {
    "value": {"type": "string", "enum": ["primary", "supporting"]},
    "grounds": {"type": "string"}
  }
}$$ AS FACT ON COLUMN;
DECLARE WITNESS dimension_w ON dimension BY (AGENT, HUMAN);
```

`primary` and `supporting` are absolute labels — v0.3 retired an
ordinal priority because rank 3 means nothing without knowing what it
is 3 *of*, and unranked rows tied at the floor filled curated lists
alphabetically.

## 2. Inventory and relevance

For each dimension-role column (and any categorical axis worth
considering):

```glossql
SELECT profile() FROM orders.region;
SELECT dimension_relevance() FROM orders.region;
SELECT value FROM GLOSSARY(orders.region::dimension_relevance) WHERE state = 'current';
```

The score is `coverage × evenness` (Pielou), zero free parameters, on
one scale for every axis. How to read it:

- **The number answers "is this axis usable, how much does it
  resolve" — interest is yours.** Which of an even 4-way `region` and
  an even 800-way `account_id` a reader wants first is business
  judgment; the score never overrules it. Gloss `dimension` with your
  verdict and grounds.
- **`truncated: true` means lower bound.** The profile caps at 20
  buckets; the unseen tail is scored as one bucket, the least even it
  could be. Under-claiming is deliberate — never promote an axis by
  assuming its tail is even.
- **Abstentions are gates, not defects**: near-keys (fraction ≥ 0.9 of
  filled rows — a key is not an axis), null-dominated columns
  (> 0.5), constants. A null-coded binary (`{X, NULL}`) is admitted —
  NULL is a bucket — but scores low through coverage; whether the lane
  matters is your call.

## 3. Hierarchies

```glossql
SELECT detect_hierarchies() FROM orders;
SELECT value FROM GLOSSARY(orders::hierarchy_candidates) WHERE state = 'current';
```

Candidates are within-table FD screens at high recall (g3 ≤ 0.05 —
the fraction of rows breaking `from → to`). Judge each:

- **λ < 0.5 is the vacuous-skew signature.** A ≥98%-dominant dependent
  passes the FD screen vacuously — knowing `zip` "determines" a flag
  that is almost always A predicts nothing. v0.3's pre-registered
  floor killed 48 such false positives with zero truth lost. Treat a
  low-λ candidate as noise unless the data argues otherwise.
- **A perfect 1:1 (`kind: alias`) is a relabeling or a coincidence,
  and only meaning separates them.** A code↔label bijection
  (`city_code ↔ city`) collapses to one canonical axis; an entity key
  that happens to align with a per-row timestamp must not. Unsure?
  Leave both, say so in prose — never merge silently.
- **Same-family role columns stay apart, however cleanly they align.**
  An origin and a destination, a bill-to and a pay-to — merging them
  silently corrupts every aggregation that crosses them.
- **Reduce transitively.** `zip → city` and `city → state` imply
  `zip → state`; the measurement serves all three (recall), you
  declare the chain, not the shortcut.

Record a surviving nest as a same-table relationship, finer → coarser,
and gloss the grounds on the pair:

```glossql
DECLARE RELATIONSHIP orders.zip -> orders.city;
DECLARE RELATIONSHIP orders.city -> orders.state;
GLOSS meaning ON orders.zip -> orders.city AS $${"value": "postal drill-down; g3 0, judged non-vacuous"}$$;
```

## 4. Enriched views

`CREATE VIEW` is native SQL. Before a view carries a join, run the
grain check — the cheapest verification of the most consequential
property a view has:

```glossql
SELECT count(*) FROM orders;
SELECT count(*) FROM orders o JOIN customers c ON o.customer_id = c.id;
```

Equal counts, exactly, or the join does not go in: a fan-out view
multiplies every downstream aggregate, and v0.3 failed the run rather
than ship one. In a one-hop star the probes are independent — check
each join alone. Carry the dimension columns you judged worth
carrying, not everything; a conformed dimension shared across facts
needs its concept named in prose, and alias axes collapse to one
canonical column while role pairs never do.
