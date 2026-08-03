# glossql

A declarative context language over a SQL host. It describes a dataset —
sources, tables, relationships, meanings, checks — so that agents and humans
work on the same data with the same context. Context is JSON validated by
JSON Schemas; analytical logic is scripts with JSON contracts; adjudication
is witnesses and detector functions, read back as bands, never written into
data.

The full definition is **[SPEC.md](./SPEC.md)** — the single normative
document of this repository. `grammar.ebnf` is the machine source of truth
for syntax; `corpus/` proves every construct against real artifacts;
`harness/check.py` keeps both honest.

The name: a *gloss* is a marginal annotation explaining a text's meaning; a
glossary is a collection of them. `GLOSS` is the write verb — you gloss an
aspect onto a subject; `GLOSSARY()` is the read.

## The statement set

```
DECLARE SOURCE / RECIPE / DATASET          -- where data comes from
DECLARE RELATIONSHIP a.x -> b.y            -- declared structure (-> m:1, <-> 1:1)
DECLARE ASPECT ... AS MEASUREMENT|FACT|QUERY   -- the vocabulary
GLOSS aspect ON subject AS { ... }         -- the one write verb, body always JSON
DECLARE FUNCTION ... ACCEPTS ... RETURNS ...   -- scripts as functions
DECLARE WITNESS ... BY (...) DETECTOR ...  -- who may speak; who adjudicates
SELECT * FROM GLOSSARY(subject)            -- collapsed context read
SELECT * FROM ATTEST(subject.aspect)       -- adjudication read (band + score)
```

Everything else — views, deletes, function extraction — is plain SQL.

## Status

- Working draft, 2026-08-03: a radical simplification of the 2026-07 draft
  (git history holds the old track; `reports/2026-08-03-simplification.md`
  records the pivot and what was deliberately dropped).
- No implementation; language first. Tech stack and evaluation options are an
  upcoming decision.

## Relationship to dataraum-context

The sibling repository [`dataraum-context`](../dataraum-context) is the
current production system (v0.3). Its pipeline, detectors, and teach
mechanisms are the fieldwork that determined this language's vocabulary —
SPEC.md §2 maps every artifact of that system onto a construct here, and
`corpus/11-12` model its two operational flows (add source, begin session) as
statement sequences.

## Prior art

- [ggsql](https://github.com/posit-dev/ggsql) — grammar-of-graphics clauses as a SQL
  extension; the pattern for a declarative tail on SELECT.
- [Snowflake semantic views](https://docs.snowflake.com/en/sql-reference/sql/create-semantic-view) —
  declared semantics as SQL objects consumed by agents; declarations only, no
  evidence model.
- [Open Semantic Interchange / Apache Ossie](https://github.com/open-semantic-interchange/osi) —
  vendor-neutral interchange for semantic models; a possible export mapping for the
  vocabulary tier.
- [DuckLake](https://ducklake.select/) — lakehouse design: parquet data files, all
  metadata in a transactional SQL database; a persistence reference point.

What none of these carry — and this language treats as first-class — is
adjudicated context: measurements, agent assertions, and human assertions
held in separate slots per subject and aspect, with disagreement surfaced as
a band instead of silently resolved.
