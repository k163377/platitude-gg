//! Pure encoding helpers: core DTOs → compact strings the QML layer decodes
//! mechanically (draw tokens / chip records). All *semantic* work already
//! happened in platitude-core; QML only draws what these strings say.

use base64::Engine as _;
use platitude_core::details::DiffTarget;
use platitude_core::graph::{Segment, SegmentKind};
use platitude_core::parse::diff::{DiffLineKind, FilePatch};
use platitude_core::patch::HunkSelect;
use platitude_core::session::{LabelKind, RefLabel};

/// Record separator for label chips (cannot occur in refnames).
pub const LABEL_SEP: char = '\u{1f}';

/// Segments → `;`-joined draw tokens: `t<lane>.<color>` (through),
/// `i<lane>.<color>` (into node), `o<lane>.<color>` (out of node).
/// Uppercase letters mark dashed segments (the WIP edge).
pub fn encode_geometry(segments: &[Segment]) -> String {
    let mut out = String::with_capacity(segments.len() * 6);
    for (i, s) in segments.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        let ch = match s.kind {
            SegmentKind::Through => 't',
            SegmentKind::IntoNode => 'i',
            SegmentKind::OutOfNode => 'o',
        };
        out.push(if s.dashed {
            ch.to_ascii_uppercase()
        } else {
            ch
        });
        out.push_str(&s.lane.to_string());
        out.push('.');
        out.push_str(&s.color.to_string());
    }
    out
}

/// A comma-separated env var as a name set (empty when unset).
fn env_name_set(var: &str) -> std::collections::HashSet<String> {
    std::env::var(var)
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_default()
}

/// Branch names previewing the PR badge (`PG_FAKE_PR=a,b`). Real PR data
/// joins in Phase 4; this hook exists so the design can be reviewed.
pub(crate) fn fake_pr_set() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| env_name_set("PG_FAKE_PR"))
}

/// Tag names previewing the remote badge (`PG_FAKE_REMOTE_TAGS=a,b`).
/// Unlike a branch, whether a tag is also on a remote cannot be read off
/// local refs — it takes `ls-remote --tags`, which joins in Phase 4 with
/// the fetch cycle; this hook exists so the design can be reviewed.
pub(crate) fn fake_remote_tag_set() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| env_name_set("PG_FAKE_REMOTE_TAGS"))
}

/// Labels → `\u{1f}`-joined chip records `KHRP` + text:
/// K = `H`ead / `L`ocal / `R`emote / `T`ag, H = head?, R = has-remote?
/// (local branches from core; tags preview via [`fake_remote_tag_set`]
/// until Phase 4), P = has-PR? (preview via [`fake_pr_set`], same).
///
/// Records arrive sorted HEAD → local → remote → tag, and stay that way:
/// the row's one chip shows the first of them, so a branch is what a
/// commit that is also tagged reads as.
pub fn encode_labels(labels: &[RefLabel]) -> String {
    let mut out = String::new();
    for (i, l) in labels.iter().enumerate() {
        if i > 0 {
            out.push(LABEL_SEP);
        }
        out.push(match l.kind {
            LabelKind::Head => 'H',
            LabelKind::LocalBranch => 'L',
            LabelKind::RemoteBranch => 'R',
            LabelKind::Tag => 'T',
        });
        out.push(if l.is_head { '1' } else { '0' });
        let remote = l.has_remote
            || (matches!(l.kind, LabelKind::Tag) && fake_remote_tag_set().contains(&l.text));
        out.push(if remote { '1' } else { '0' });
        let pr = matches!(l.kind, LabelKind::LocalBranch) && fake_pr_set().contains(&l.text);
        out.push(if pr { '1' } else { '0' });
        out.push_str(&l.text);
    }
    out
}

/// Deterministic identicon code for an author (GitHub-style 5x5 pattern,
/// generated locally — fetching real avatars would violate the
/// no-network-except-git constraint).
///
/// Layout: bits 0..15 = left 3 columns of a 5x5 grid (row-major, mirrored
/// to the right by the renderer), bits 15..18 = palette index (0..8).
pub fn avatar_code(author: &str) -> i32 {
    // FNV-1a 32-bit.
    let mut hash: u32 = 0x811c_9dc5;
    for b in author.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    let mut pattern = hash & 0x7fff;
    if pattern == 0 {
        pattern = 0b00000_00100_00000; // center dot fallback
    }
    let color = (hash >> 15) % 8;
    (pattern | (color << 15)) as i32
}

/// Lanes that touch the bottom edge of a row (from its geometry tokens):
/// `t` segments plus `o` targets. Used by the truncation footer to draw
/// the lanes running off the end of the window. Output: `lane.color;...`.
pub fn tail_lanes(geometry: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    for token in geometry.split(';').filter(|t| t.len() > 1) {
        let kind = token.as_bytes()[0].to_ascii_lowercase();
        if kind != b't' && kind != b'o' {
            continue;
        }
        let rest = &token[1..];
        let Some((lane, _color)) = rest.split_once('.') else {
            continue;
        };
        if seen.insert(lane.to_string()) {
            if !out.is_empty() {
                out.push(';');
            }
            out.push_str(rest);
        }
    }
    out
}

/// One hunk or one line of it, as the diff pane addresses them.
///
/// The indices come straight off the row the user clicked, so nothing is
/// parsed and nothing can drift: a wrong index would stage a different
/// line than the one under the cursor. A negative line means the whole
/// hunk.
pub fn hunk_selection(hunk: i32, line: i32) -> Vec<HunkSelect> {
    let Ok(hunk) = usize::try_from(hunk) else {
        return Vec::new();
    };
    match usize::try_from(line) {
        Ok(line) => vec![HunkSelect::lines(hunk, [line])],
        Err(_) => vec![HunkSelect::whole(hunk)],
    }
}

/// Rebuilds the diff target a working-tree selection refers to.
/// `kind` is the prefix [`diff_key`] uses (`staged` / `unstaged` /
/// `untracked`); a committed diff is not stageable and yields `None`.
pub fn worktree_target(kind: &str, path: &str, orig_path: &str) -> Option<DiffTarget> {
    let orig = (!orig_path.is_empty()).then(|| orig_path.to_string());
    match kind {
        "staged" => Some(DiffTarget::Staged {
            path: path.to_string(),
            orig_path: orig,
        }),
        "unstaged" => Some(DiffTarget::Unstaged {
            path: path.to_string(),
        }),
        "untracked" => Some(DiffTarget::Untracked {
            path: path.to_string(),
        }),
        _ => None,
    }
}

pub fn diff_key(target: &DiffTarget) -> String {
    match target {
        DiffTarget::Commit { oid, path, .. } => format!("commit:{}:{path}", oid.to_hex()),
        DiffTarget::Staged { path, .. } => format!("staged:{path}"),
        DiffTarget::Unstaged { path } => format!("unstaged:{path}"),
        DiffTarget::Untracked { path } => format!("untracked:{path}"),
    }
}

/// `data:` URL a QML `Image` loads directly — no temp files, no image
/// providers, and blob content works the same as working-tree content.
pub fn image_data_url(mime: &str, bytes: &[u8]) -> String {
    let mut out = format!("data:{mime};base64,");
    base64::engine::general_purpose::STANDARD.encode_string(bytes, &mut out);
    out
}

/// Human-readable byte size ("67 B", "1.5 KB", "234 KB", "1.2 MB").
/// 1024-based; one decimal below ten so small differences stay visible.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for u in UNITS {
        value /= 1024.0;
        unit = u;
        if value < 1024.0 {
            break;
        }
    }
    if value < 10.0 {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.0} {unit}")
    }
}

/// One flattened row of the diff pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    /// `hunk` / `ctx` / `add` / `del` / `meta`.
    pub kind: &'static str,
    /// -1 when the side has no line number.
    pub old_no: i32,
    pub new_no: i32,
    pub text: String,
    /// Which hunk of the file this row belongs to, and which line of that
    /// hunk it is (-1 on the hunk header). These are the same indices
    /// [`platitude_core::patch::HunkSelect`] addresses, so a row can be
    /// staged straight from what the pane is showing.
    pub hunk: i32,
    pub line: i32,
}

/// Flattens parsed patches into displayable rows (hunk headers inline).
/// `binary_note` inserts the "(binary file)" meta row; the caller turns it
/// off when a preview (image / size summary) already covers that file.
pub fn flatten_patches(patches: &[FilePatch], binary_note: bool) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    for patch in patches {
        if patch.is_binary {
            if binary_note {
                rows.push(DiffRow {
                    kind: "meta",
                    old_no: -1,
                    new_no: -1,
                    text: String::from("(binary file)"),
                    hunk: -1,
                    line: -1,
                });
            }
            continue;
        }
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            let heading = if hunk.heading.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.heading)
            };
            let hunk_index = i32::try_from(hunk_index).unwrap_or(-1);
            rows.push(DiffRow {
                kind: "hunk",
                old_no: -1,
                new_no: -1,
                text: format!(
                    "@@ -{},{} +{},{} @@{heading}",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                ),
                hunk: hunk_index,
                line: -1,
            });
            for (line_index, line) in hunk.lines.iter().enumerate() {
                let kind = match line.kind {
                    DiffLineKind::Context => "ctx",
                    DiffLineKind::Addition => "add",
                    DiffLineKind::Deletion => "del",
                    DiffLineKind::NoNewline => "meta",
                };
                rows.push(DiffRow {
                    kind,
                    old_no: line.old_no.map_or(-1, |n| n as i32),
                    new_no: line.new_no.map_or(-1, |n| n as i32),
                    text: line.text.clone(),
                    hunk: hunk_index,
                    line: i32::try_from(line_index).unwrap_or(-1),
                });
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::parse::diff::parse_patch;

    #[test]
    fn geometry_tokens_round_trip_by_eye() {
        let segs = [
            Segment {
                kind: SegmentKind::Through,
                lane: 0,
                color: 3,
                dashed: false,
            },
            Segment {
                kind: SegmentKind::IntoNode,
                lane: 2,
                color: 11,
                dashed: false,
            },
            Segment {
                kind: SegmentKind::OutOfNode,
                lane: 1,
                color: 0,
                dashed: true,
            },
        ];
        assert_eq!(
            encode_geometry(&segs),
            "t0.3;i2.11;O1.0",
            "dashed segments encode uppercase"
        );
        assert_eq!(encode_geometry(&[]), "");
    }

    #[test]
    fn tail_lanes_picks_bottom_touching_segments() {
        assert_eq!(tail_lanes("t0.3;i2.11;o1.0"), "0.3;1.0");
        assert_eq!(tail_lanes("i2.5"), "", "into-node stops at the node");
        assert_eq!(tail_lanes("t0.1;o0.2"), "0.1", "deduped by lane");
        assert_eq!(tail_lanes("T0.4;O1.5"), "0.4;1.5", "dashed still counts");
        assert_eq!(tail_lanes(""), "");
    }

    #[test]
    fn avatar_codes_are_deterministic_and_bounded() {
        let a = avatar_code("Alice <a@example.com>");
        assert_eq!(a, avatar_code("Alice <a@example.com>"), "stable");
        assert_ne!(a, avatar_code("Bob <b@example.com>"));
        assert!(a >= 0);
        let color = (a >> 15) & 0x7;
        assert!((0..8).contains(&color));
        assert_ne!(a & 0x7fff, 0, "pattern is never empty");
        assert_ne!(avatar_code("") & 0x7fff, 0, "empty author still draws");
    }

    #[test]
    fn label_records_have_fixed_prefix() {
        let labels = [
            RefLabel {
                text: "main".into(),
                kind: LabelKind::LocalBranch,
                has_remote: true,
                is_head: true,
            },
            RefLabel {
                text: "v1.0".into(),
                kind: LabelKind::Tag,
                has_remote: false,
                is_head: false,
            },
        ];
        assert_eq!(encode_labels(&labels), "L110main\u{1f}T000v1.0");
    }

    #[test]
    fn a_tag_carries_the_remote_bit_like_a_branch() {
        // What the fetch cycle will set in Phase 4; until then the same
        // bit comes from PG_FAKE_REMOTE_TAGS.
        let labels = [RefLabel {
            text: "v1.0".into(),
            kind: LabelKind::Tag,
            has_remote: true,
            is_head: false,
        }];
        assert_eq!(encode_labels(&labels), "T010v1.0");
    }

    #[test]
    fn flatten_produces_hunk_headers_and_numbers() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@ heading
 same
-old
+new
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()), true);
        assert_eq!(rows[0].kind, "hunk");
        assert!(rows[0].text.contains("@@ -1,2 +1,2 @@ heading"));
        assert_eq!(rows[1].kind, "ctx");
        assert_eq!((rows[1].old_no, rows[1].new_no), (1, 1));
        assert_eq!(rows[2].kind, "del");
        assert_eq!((rows[2].old_no, rows[2].new_no), (2, -1));
        assert_eq!(rows[3].kind, "add");
        assert_eq!((rows[3].old_no, rows[3].new_no), (-1, 2));
    }

    #[test]
    fn a_negative_line_selects_the_whole_hunk() {
        assert_eq!(hunk_selection(3, -1), vec![HunkSelect::whole(3)]);
        assert_eq!(hunk_selection(3, 4), vec![HunkSelect::lines(3, [4])]);
    }

    #[test]
    fn a_row_with_no_hunk_selects_nothing() {
        // Rows outside any hunk (the binary-file note) carry -1, and
        // staging one of those must not fall back to hunk zero.
        assert!(hunk_selection(-1, -1).is_empty());
        assert!(hunk_selection(-1, 2).is_empty());
    }

    #[test]
    fn flattened_rows_carry_the_indices_that_address_them() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,3 @@
 keep
+added
@@ -10,2 +11,1 @@
-removed
 tail
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()), true);
        // The header carries its hunk but no line; the lines that follow
        // are numbered from zero within that hunk — exactly what
        // `HunkSelect` addresses.
        assert_eq!((rows[0].kind, rows[0].hunk, rows[0].line), ("hunk", 0, -1));
        assert_eq!((rows[1].kind, rows[1].hunk, rows[1].line), ("ctx", 0, 0));
        assert_eq!((rows[2].kind, rows[2].hunk, rows[2].line), ("add", 0, 1));
        assert_eq!((rows[3].kind, rows[3].hunk, rows[3].line), ("hunk", 1, -1));
        assert_eq!((rows[4].kind, rows[4].hunk, rows[4].line), ("del", 1, 0));
        assert_eq!((rows[5].kind, rows[5].hunk, rows[5].line), ("ctx", 1, 1));
    }

    #[test]
    fn worktree_targets_exclude_committed_diffs() {
        assert_eq!(
            worktree_target("unstaged", "f.txt", ""),
            Some(DiffTarget::Unstaged {
                path: "f.txt".into()
            })
        );
        assert_eq!(
            worktree_target("staged", "new.txt", "old.txt"),
            Some(DiffTarget::Staged {
                path: "new.txt".into(),
                orig_path: Some("old.txt".into())
            })
        );
        assert_eq!(worktree_target("commit", "f.txt", ""), None);
    }

    #[test]
    fn binary_patch_flattens_to_meta_row() {
        let patch = "\
diff --git a/x.png b/x.png
Binary files a/x.png and b/x.png differ
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()), true);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "meta");
        // With a preview covering the file, the note is dropped entirely.
        assert!(flatten_patches(&parse_patch(patch.as_bytes()), false).is_empty());
    }

    #[test]
    fn image_data_urls_are_base64_with_the_mime_up_front() {
        assert_eq!(
            image_data_url("image/png", b"abc"),
            "data:image/png;base64,YWJj"
        );
        assert_eq!(image_data_url("image/gif", b""), "data:image/gif;base64,");
    }

    #[test]
    fn human_sizes_step_through_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(239_616), "234 KB");
        assert_eq!(human_size(1_258_291), "1.2 MB");
        assert_eq!(human_size(17 * 1024 * 1024 * 1024), "17 GB");
    }
}
