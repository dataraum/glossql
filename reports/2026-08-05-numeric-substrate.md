# The numeric substrate — matrix ops for scripts, laid out

Date: 2026-08-05. The project lead's observation: the begin-session
ports converge on SQL. The question: should we spike matrix operations
in rhai, and how do DB results share memory with scripts? This is the
options lay-out for joint refinement — a record of the choices and
their physics, not a decision.

## What the ported lanes actually need

The demand schedule from the begin-session analysis
(`2026-08-05-begin-session-analysis.md`), in port order:

- `entity`, `behavior_evidence`, the dimensions measurements,
  `derived_formulas` — **SQL suffices**. Period sums, residual
  ratios, entropy over top-K buckets, GROUP BY functional-dependency
  counts: all reachable through the script door plus scalar
  arithmetic.
- **Drivers** (operating-model era) is where columnar numerics
  arrive: grouped variance reduction per candidate dimension,
  permutation tests (hundreds of seeded shuffles over the target),
  intraclass correlation, within-entity de-meaned residuals — over
  up to millions of rows.
- **Correlation matrices** (on-demand, if wanted): pairwise dot
  products over standardized columns — the one true matrix
  multiplication in sight.
- Decompositions (SVD/eigen, regression) appear in no planned lane.

So the honest shape of the need: **1-D reductions, group aggregation,
seeded shuffles, and one matmul** — statistics, not linear algebra.

## The seam today (crates/scripts)

Memory sharing is already mostly solved, because the handles hold
Arrow directly:

- `Table(Arc<Vec<RecordBatch>>)`, `Col(ArrayRef)` (lib.rs:45,49).
  `col()` on a single-batch result is an `Arc` clone — zero-copy;
  multi-batch results concat once into a contiguous array
  (lib.rs:92-107).
- The numeric kernels (`sum`…`mad`) cast to Float64 and collect
  non-null values into a `Vec<f64>` (lib.rs:457-469, 440-447) — one
  cast copy plus one compaction copy per call, not cached on the
  handle.
- Per-element access (`value_at`, `cell`) serves **display strings**
  (lib.rs:110-127, 293) — the `parse_int()` wart the relationships
  script hit. Fine for one-row aggregate reads; a dead end for
  numerics.

The physics underneath (vendored arrow 58.4, the tree datafusion 53.1
pins): `Float64Array::values()` yields a `&ScalarBuffer<f64>`
(arrow-array `primitive_array.rs:725`) which derefs to a plain
`&[f64]` (arrow-buffer `scalar.rs:156-162`) — **a zero-copy slice
view exists whenever the column is already Float64 and
single-chunk**. Two things force a copy: nulls (compact, or carry a
mask-aware kernel) and dtype (our money is `DECIMAL(18,2)`;
Decimal→Float64 is a documented lossy cast, arrow-cast
`mod.rs:636-638`). The principle that falls out: **exact arithmetic
stays in SQL over Decimal — the numeric plane is statistics, and f64
is its currency.** One explicit materialization at the boundary,
never per-element churn.

## The architecture principle

rhai is glue and never loops over elements — interpreter dispatch
plus `Dynamic` boxing per element is orders slower than a kernel, and
the display-string accessors make it wrong as well as slow. "Matrix
operations on rhai" therefore means: **typed handles + Rust kernels**,
extending the existing `register_fn` pattern, with scripts composing
whole-column operations. Sketch:

- `col.floats(policy)` → a `NumVec` handle: one explicit
  materialization (zero-copy fast path when Float64/no-null/one
  chunk), null policy named at the call (`drop` with disclosed count,
  or `fail`).
- `t.matrix([cols], policy)` → a `NumMat` handle: column-major,
  row-complete or pairwise null policy.
- Kernels on the handles: elementwise ops, reductions, `group_agg`
  by a key column (bincount-style), `shuffle(seed)` /
  `perm_test(seed, reps)`, `dot`. Seeds are always explicit
  arguments — determinism is the caller's contract, no ambient RNG
  in the engine.
- Typed accessors alongside (fixing the `parse_int` wart) for the
  one-row reads.

## Library options

| option | verdict |
|---|---|
| hand-rolled kernels over `&[f64]` (status quo, extended) | sufficient for everything up to drivers; every statistic hand-written |
| **ndarray** | the numpy analogue: zero-copy `ArrayView` over slices, axis ops, matmul via matrixmultiply, `ndarray-stats`/`-rand` ecosystem — the natural vocabulary **inside** kernels |
| faer | best-in-class pure-Rust decompositions, zero-copy `from_column_major_slice`; nothing planned needs QR/SVD — add later on the same slice seam if regression/PCA arrive |
| nalgebra | center of gravity is fixed-size/geometry; wrong fit for column statistics |
| SQL/UDAFs only | keeps one engine but contorts shuffles and residual loops, and UDAFs are server surface — against the wipeable-library principle |
| polars | a second dataframe engine beside DataFusion — a parallel layer, ruled out by the substrate principle |
| candle / burn | GPU/autograd tensor frameworks; a scale of dependency nothing here justifies |

Recommendation: **ndarray inside kernels, never in rhai signatures** —
scripts see `NumVec`/`NumMat` and kernel names, so the library stays
swappable (faer could sit beside it later without a script changing).
Plus a seeded RNG dependency (`rand` + a small deterministic
generator) for the permutation kernels.

## The spike, if wanted

Bounded to roughly a day, wipe-or-keep by numbers:

1. `floats()`/`matrix()` materialization with null policy and the
   zero-copy fast path.
2. Grouped variance reduction (the drivers gain criterion) over a
   ~1M-row column — the representative future load.
3. A seeded permutation test (500 shuffles) on the same data.

Measured: materialization cost vs the query that produced the data,
kernel throughput, rhai-glue overhead. Success looks like the gain
computation in low hundreds of milliseconds at 1M rows and
materialization dwarfed by the query. The spike also lands the typed
accessors, which every future script wants regardless.

## Open forks (the lead's call)

1. **Timing** — spike now (de-risks the seam, fixes typed access
   early) vs at drivers time (nothing before it needs the kernels).
2. **Vocabulary** — adopt ndarray in the spike vs slices-only until
   real pressure.
3. **Null policy default** — named per call (proposed) vs one global
   rule.
4. **`NumMat` in v1** — or columns-only until the correlation matmul
   actually lands.
