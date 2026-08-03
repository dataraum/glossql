//! The corpus is the acceptance suite — the Rust twin of `harness/check.py`:
//! every ```sql block in SPEC.md parses, every ```glossql block in corpus/*.md
//! parses, every ```glossql-gap block contains at least one failing statement.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root above server/crates/parser")
        .to_path_buf()
}

/// (tag, body) for every fenced block, mirroring check.py's FENCE_RE.
fn fences(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut tag: Option<String> = None;
    let mut body = String::new();
    for line in text.lines() {
        match (&tag, line.starts_with("```")) {
            (None, true) => {
                tag = Some(line[3..].trim().to_string());
                body.clear();
            }
            (Some(t), true) => {
                out.push((t.clone(), body.clone()));
                tag = None;
            }
            (Some(_), false) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, false) => {}
        }
    }
    out
}

struct Stats {
    blocks: usize,
    stmts: usize,
    failures: Vec<String>,
    gaps_closed: Vec<String>,
}

fn check_file(path: &Path, stats: &mut Stats, tags: &[(&str, bool)]) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    for (blockno, (tag, body)) in fences(&text).iter().enumerate() {
        let Some((_, expect_gap)) = tags.iter().find(|(t, _)| t == tag) else { continue };
        let results = parser::check_source(body);
        stats.blocks += 1;
        stats.stmts += results.len();
        let errs: Vec<_> = results.iter().filter_map(|(p, e)| e.as_ref().map(|e| (p, e))).collect();
        if *expect_gap {
            if errs.is_empty() {
                stats.gaps_closed.push(format!("{name} block {}", blockno + 1));
            }
        } else {
            for (p, e) in errs {
                stats.failures.push(format!("{name} block {} [{tag}]: {p}: {e}", blockno + 1));
            }
        }
    }
}

#[test]
fn corpus_invariant() {
    let root = repo_root();
    let mut stats = Stats { blocks: 0, stmts: 0, failures: Vec::new(), gaps_closed: Vec::new() };

    check_file(&root.join("SPEC.md"), &mut stats, &[("sql", false)]);

    let mut fixtures: Vec<_> = fs::read_dir(root.join("corpus"))
        .expect("corpus/ directory")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    fixtures.sort();
    for f in &fixtures {
        check_file(f, &mut stats, &[("glossql", false), ("glossql-gap", true)]);
    }

    println!(
        "{} blocks, {} statements checked; {} failures, {} gaps closed",
        stats.blocks,
        stats.stmts,
        stats.failures.len(),
        stats.gaps_closed.len()
    );
    assert!(stats.blocks > 0 && stats.stmts > 0, "corpus fences not found — path wiring broken");
    assert!(stats.failures.is_empty(), "corpus regressions:\n{}", stats.failures.join("\n"));
    assert!(
        stats.gaps_closed.is_empty(),
        "gaps closed — flip the tag and fold the decision into SPEC.md:\n{}",
        stats.gaps_closed.join("\n")
    );
}
