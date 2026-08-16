//! The header lines: hunk ranges, and the paths on either side.

use super::DiffHunk;

/// `@@ -a,b +c,d @@ heading`, or the combined `@@@ -a,b -e,f +c,d @@@` with
/// one `@` per parent plus one and one `-` range each (counts default to 1
/// when omitted).
pub(super) fn parse_hunk_header(line: &str) -> Option<DiffHunk> {
    let ats = line.bytes().take_while(|b| *b == b'@').count();
    if ats < 2 {
        return None;
    }
    let olds = ats - 1;
    let rest = line.get(ats..)?;
    // The first closing run is the real one: a heading that contains `@@`
    // sits after it, and the ranges between never do.
    let close = "@".repeat(ats);
    let (ranges, heading) = match rest.find(&close) {
        Some(at) => (
            rest.get(..at)?.trim(),
            rest.get(at + close.len()..)?.trim().to_string(),
        ),
        None => (rest.trim(), String::new()),
    };
    let parse_range = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let mut parts = ranges.split_whitespace();
    let mut old = Vec::with_capacity(olds);
    for _ in 0..olds {
        old.push(parse_range(parts.next()?.strip_prefix('-')?)?);
    }
    let (new_start, new_count) = parse_range(parts.next()?.strip_prefix('+')?)?;
    let (old_start, old_count) = *old.first()?;
    Some(DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        extra_old: old[1..].to_vec(),
        heading,
        lines: Vec::new(),
    })
}

/// Path after `---` / `+++`: `/dev/null`, or `<prefix>path` with an
/// optional trailing tab git adds for paths containing spaces.
pub(super) fn parse_side_path(rest: &str, prefix: &str) -> Option<String> {
    let cleaned = rest.trim_end_matches('\t');
    if cleaned == "/dev/null" {
        return None;
    }
    let path = cleaned.strip_prefix(prefix).unwrap_or(cleaned);
    Some(path.to_string())
}

pub(super) fn strip_prefix_a(p: &str) -> Option<String> {
    parse_side_path(p, "a/")
}

pub(super) fn strip_prefix_b(p: &str) -> Option<String> {
    parse_side_path(p, "b/")
}

/// Best-effort split of `a/old b/new` from the `diff --git` line. Paths
/// with spaces make this ambiguous; `---`/`+++` lines are authoritative.
pub(super) fn split_git_header_paths(rest: &str) -> Option<(String, String)> {
    let idx = rest.find(" b/")?;
    let a = rest[..idx].to_string();
    let b = rest[idx + 1..].to_string();
    Some((a, b))
}
