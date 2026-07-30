# 08 · Teach payloads (8 types) — 3 CLEAN · 2 spec-admitted GAPS · 1 unadmitted GAP

Source: `dataraum-context/packages/cockpit/src/tools/teach.validation.ts` —
TYPE_SCHEMAS roster: `type_pattern, null_value, unit, relationship, hierarchy,
validation, cycle, metric` (all Zod-validated; a 9th direct-read type
`expected_dependency` lives outside the registry, `core/overlay.py:49-55`).

## Clean transcriptions

`relationship` teach `{action: confirm|reject|add, from_column_id, to_column_id}`:

```glossql
DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id
  CARDINALITY many_to_one BY USER analyst;
DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id
  REJECTED BY USER analyst;
```

`unit` teach `{table, column, unit}`:

```glossql
DECLARE unit(orders.amount, value := 'EUR') BY USER analyst;
```

`hierarchy` teach, `add` action:

```glossql
DECLARE HIERARCHY geo IN customers
  LEVELS (country > region > city) KIND drilldown BY USER analyst;
```

## Gap — hierarchy `reject` (unadmitted): no negative form outside RELATIONSHIP

```glossql-gap
DECLARE HIERARCHY geo IN customers REJECTED BY USER analyst;
```

## Spec-admitted gaps (§8.3) — workspace-scoped vocabulary teaches

`type_pattern` payload (an AGENT_AUTOAPPLY type — the constrained-decoding
surface §2.5 claims):

```ts
{name: 'eu_date', pattern: '^\\d{2}\\.\\d{2}\\.\\d{4}$', inferred_type: 'DATE',
 semantic_type?, detected_unit?, case_sensitive?, standardization_expr?  /* SQL */}
```

Nearest expressible form is an *undeclared* aspect application — it parses, but
no ASPECT declaration exists, the subject is the workspace (vocabulary, not a
column fact), and `standardization_expr` is a transported SQL body:

```glossql
DECLARE type_pattern(workspace, name := 'eu_date',
  pattern := '^\d{2}\.\d{2}\.\d{4}$', inferred_type := 'DATE',
  standardization := 'strptime(value, ''%d.%m.%Y'')') BY USER analyst;
```

Classification: **SEMANTICS UNDEFINED** (parses; nothing defines workspace-scoped
vocabulary aspects — §8.3 owns this). Same for `null_value`: the real teach is
workspace-scoped with a `category` axis; §3.2's `null_token(orders.amount, …)`
sketch is column-scoped.

`validation` / `cycle` / `metric` teaches are fixtures 04/05/03 with `BY USER` —
they inherit those verdicts.
