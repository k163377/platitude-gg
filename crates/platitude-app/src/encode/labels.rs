//! The packed record strings QML unpacks itself: the ref chips on a
//! row, and the people a commit credits.

use platitude_core::session::{LabelKind, RefLabel};

use super::graph::avatar_code;
use super::{FIELD_SEP, RECORD_SEP};

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

/// How many flag digits stand between the kind letter and the name. The
/// QML side counts the same seat by hand (`RefChip`), so a change here is
/// a change there.
const FLAGS: usize = 5;

/// Labels → `\u{1f}`-joined chip records: a kind letter, the flag digits,
/// the name, and — only when the ref was read off a remote — the field
/// separator and the remotes it came from.
///
/// The letter is `H`ead / `L`ocal / `R`emote / `T`ag; the flags, in order,
/// are is-head, has-remote, has-PR (preview via [`fake_pr_set`] until Phase
/// 4), is-it-here and is-it-out-in-another-working-copy. The fourth is what
/// the chip writes in the name's colour: a remote branch and a tag only a
/// remote has are both somewhere else, and read the same way for it. The
/// fifth is what makes the chip say a move cannot go here — it mutes and
/// wears the WORKTREES mark (`RefChip.recHeld`), because git refuses a
/// `switch` onto a branch another working copy holds (2026-08-21 実測).
///
/// **The flags are fixed-width and the name starts after them**
/// ([`FLAGS`]), so adding one moves every reader; the test at the foot of
/// this file spells a whole record out for that reason.
///
/// Records arrive sorted HEAD → local → remote → tag, and stay that way:
/// the row's one chip shows the first of them, so a branch is what a
/// commit that is also tagged reads as.
pub fn encode_labels(labels: &[RefLabel]) -> String {
    let mut out = String::new();
    for (i, l) in labels.iter().enumerate() {
        if i > 0 {
            out.push(RECORD_SEP);
        }
        out.push(match l.kind {
            LabelKind::Head => 'H',
            LabelKind::LocalBranch => 'L',
            LabelKind::RemoteBranch => 'R',
            LabelKind::Tag => 'T',
        });
        out.push(if l.is_head { '1' } else { '0' });
        out.push(if l.has_remote { '1' } else { '0' });
        let pr =
            matches!(l.kind, LabelKind::LocalBranch) && fake_pr_set().contains(l.text.as_str());
        out.push(if pr { '1' } else { '0' });
        out.push(if l.here { '1' } else { '0' });
        out.push(if l.held_elsewhere { '1' } else { '0' });
        out.push_str(&l.text);
        if !l.remote.is_empty() {
            out.push(FIELD_SEP);
            out.push_str(&l.remote);
        }
    }
    out
}

/// The refnames in a chip record string, in order — the inverse of the
/// name half of [`encode_labels`].
pub fn label_names(encoded: &str) -> impl Iterator<Item = &str> {
    encoded.split(RECORD_SEP).filter_map(|record| {
        // Kind letter plus the flag digits, then the name, then — only
        // when the ref was read off a remote — the remotes it came from.
        let rest = record.get(FLAGS + 1..)?;
        Some(rest.split(FIELD_SEP).next().unwrap_or(rest))
    })
}

/// Whether one of the names in a chip record string is the one the
/// working tree stands on — the first flag digit, read by its seat the
/// way `RefChip.recHead` reads it.
///
/// The records are sorted with that one first (`joins::label_index`), so
/// a row that answers `true` here shows it on the chip it draws.
pub fn labels_head(encoded: &str) -> bool {
    encoded
        .split(RECORD_SEP)
        .any(|record| record.as_bytes().get(1) == Some(&b'1'))
}

/// Co-authors → `\u{1f}`-joined records of name, address and identicon
/// code, in that order, separated by [`FIELD_SEP`].
///
/// A trailer with no address still gets its two separators, so the reader
/// can index without counting.
pub fn encode_co_authors(mates: &[platitude_core::details::CoAuthor]) -> String {
    let mut out = String::new();
    for (i, m) in mates.iter().enumerate() {
        if i > 0 {
            out.push(RECORD_SEP);
        }
        out.push_str(&m.name);
        out.push(FIELD_SEP);
        out.push_str(&m.email);
        out.push(FIELD_SEP);
        out.push_str(&avatar_code(&m.name).to_string());
    }
    out
}

/// The (name, address) of each credited person in a packed record
/// string — the inverse of the first two fields of [`encode_co_authors`].
///
/// The third field is a number: a reader that searched the packed string
/// whole would answer a typed `12345` with somebody's identicon code.
pub fn co_author_pairs(encoded: &str) -> impl Iterator<Item = (&str, &str)> {
    encoded.split(RECORD_SEP).filter_map(|record| {
        let mut fields = record.split(FIELD_SEP);
        let name = fields.next()?;
        Some((name, fields.next().unwrap_or("")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_records_have_fixed_prefix() {
        let labels = [
            RefLabel {
                text: "main".into(),
                kind: LabelKind::LocalBranch,
                has_remote: true,
                is_head: true,
                here: true,
                remote: String::new(),
                held_elsewhere: false,
            },
            RefLabel {
                text: "v1.0".into(),
                kind: LabelKind::Tag,
                has_remote: false,
                is_head: false,
                here: true,
                remote: String::new(),
                held_elsewhere: false,
            },
        ];
        assert_eq!(encode_labels(&labels), "L11010main\u{1f}T00010v1.0");
        // The seat the head flag is read by, from either end of the list.
        assert!(labels_head("L11010main\u{1f}T00010v1.0"));
        assert!(labels_head("T00010v1.0\u{1f}L11010main"));
        assert!(!labels_head("L01010main\u{1f}T00010v1.0"));
        assert!(!labels_head(""));
    }

    /// The fifth flag, spelled out: the chip is what says a move cannot
    /// go here, and it reads the digit by its seat (`RefChip`).
    #[test]
    fn a_branch_another_working_copy_holds_carries_the_last_flag() {
        let labels = [RefLabel {
            text: "feature/topic-a".into(),
            kind: LabelKind::LocalBranch,
            has_remote: false,
            is_head: false,
            here: true,
            remote: String::new(),
            held_elsewhere: true,
        }];
        assert_eq!(encode_labels(&labels), "L00011feature/topic-a");
        // And the name still starts where the readers look for it.
        assert_eq!(
            label_names("L00011feature/topic-a").collect::<Vec<_>>(),
            vec!["feature/topic-a"]
        );
    }

    #[test]
    fn co_authors_pack_name_address_and_face_into_one_record_each() {
        use platitude_core::details::CoAuthor;
        let packed = encode_co_authors(&[
            CoAuthor {
                name: "Claude Opus 5".into(),
                email: "noreply@anthropic.com".into(),
            },
            CoAuthor {
                name: "Nameless".into(),
                email: String::new(),
            },
        ]);
        let records: Vec<&str> = packed.split(RECORD_SEP).collect();
        assert_eq!(records.len(), 2);

        let first: Vec<&str> = records[0].split(FIELD_SEP).collect();
        assert_eq!(first[0], "Claude Opus 5");
        assert_eq!(first[1], "noreply@anthropic.com");
        assert_eq!(
            first[2],
            avatar_code("Claude Opus 5").to_string(),
            "the face comes off the name, the way the graph rows' do"
        );

        let second: Vec<&str> = records[1].split(FIELD_SEP).collect();
        assert_eq!(second.len(), 3);
        assert_eq!(second[1], "");
    }

    #[test]
    fn no_co_authors_pack_into_nothing() {
        assert_eq!(encode_co_authors(&[]), "");
    }

    #[test]
    fn a_tag_carries_the_remote_bit_like_a_branch() {
        let labels = [RefLabel {
            text: "v1.0".into(),
            kind: LabelKind::Tag,
            has_remote: true,
            is_head: false,
            here: true,
            remote: String::new(),
            held_elsewhere: false,
        }];
        assert_eq!(encode_labels(&labels), "T01010v1.0");
    }

    #[test]
    fn a_tag_only_a_remote_has_says_it_is_not_here_and_whose_it_is() {
        let labels = [RefLabel {
            text: "v9.9".into(),
            kind: LabelKind::Tag,
            has_remote: true,
            is_head: false,
            here: false,
            remote: "origin, fork".into(),
            held_elsewhere: false,
        }];
        assert_eq!(encode_labels(&labels), "T01000v9.9\u{1e}origin, fork");
    }

    #[test]
    fn names_come_back_out_of_the_records_they_went_into() {
        let labels = [
            RefLabel {
                text: "main".into(),
                kind: LabelKind::LocalBranch,
                has_remote: true,
                is_head: true,
                here: true,
                remote: String::new(),
                held_elsewhere: false,
            },
            RefLabel {
                text: "v9.9".into(),
                kind: LabelKind::Tag,
                has_remote: true,
                is_head: false,
                here: false,
                remote: "origin, fork".into(),
                held_elsewhere: false,
            },
        ];
        let encoded = encode_labels(&labels);
        assert_eq!(
            label_names(&encoded).collect::<Vec<_>>(),
            vec!["main", "v9.9"],
            "the remotes a record was read from are not part of its name"
        );
        assert_eq!(label_names("").count(), 0);
        assert_eq!(label_names("L11").count(), 0);
    }

    #[test]
    fn a_record_with_no_remote_carries_no_field_separator() {
        let labels = [RefLabel {
            text: "v9.9".into(),
            kind: LabelKind::Tag,
            has_remote: false,
            is_head: false,
            here: true,
            remote: String::new(),
            held_elsewhere: false,
        }];
        assert!(!encode_labels(&labels).contains(FIELD_SEP));
    }
}
