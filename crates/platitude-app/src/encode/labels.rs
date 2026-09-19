//! The packed record strings QML unpacks itself: the ref chips on a
//! row, and the people a commit credits.

use platitude_core::session::{LabelKind, RefLabel};

use super::graph::avatar_code;
use super::{FIELD_SEP, RECORD_SEP};

/// Branch names wearing the PR badge. Real PR data joins in Phase 4; until
/// then the only thing that ever fills this is the harness, so the design
/// can be reviewed (`harness::Knobs::fake_pr`) — and a build without the
/// harness has an empty set here and no way to be handed a full one.
pub(crate) fn pr_set() -> &'static std::collections::HashSet<String> {
    &crate::harness::knobs().fake_pr
}

/// How many flag digits stand between the kind letter and the name. The
/// QML side counts the same seat by hand (`RefChip`), so a change here is
/// a change there.
const FLAGS: usize = 5;

/// Labels → `\u{1f}`-joined chip records: a kind letter, the flag digits,
/// the name, and — only when the ref was read off a remote — the field
/// separator and the remotes it came from.
///
/// The letter is `H`ead / `L`ocal / `R`emote / `W`orktree / `T`ag; the
/// flags, in order, are is-head, has-remote, has-PR (`pr` names the
/// branches wearing it — the callers pass [`pr_set`], the preview until
/// Phase 4), is-it-here and is-it-out-in-another-working-copy. The
/// fourth is what the chip writes in the name's colour: a remote branch
/// and a tag only a remote has are both somewhere else, and read the same
/// way for it. The fifth is what makes the chip say a move cannot go here
/// — it mutes and wears the WORKTREES mark (`RefChip.recHeld`), because
/// git refuses a `switch` onto a branch another working copy holds
/// (measured); the `W` record says the same thing about a copy that has
/// no branch to carry the flag, and is drawn the same way for it.
///
/// **The flags are fixed-width and the name starts after them**
/// ([`FLAGS`]), so adding one moves every reader; the test at the foot of
/// this file spells a whole record out for that reason.
///
/// Records arrive sorted HEAD → local → remote → tag, and stay that way:
/// the row's one chip shows the first of them, so a branch is what a
/// commit that is also tagged reads as.
pub fn encode_labels(labels: &[RefLabel], pr: &std::collections::HashSet<String>) -> String {
    let mut out = String::new();
    for (i, l) in labels.iter().enumerate() {
        if i > 0 {
            out.push(RECORD_SEP);
        }
        out.push(match l.kind {
            LabelKind::Head => 'H',
            LabelKind::LocalBranch => 'L',
            LabelKind::RemoteBranch => 'R',
            LabelKind::Worktree => 'W',
            LabelKind::Tag => 'T',
        });
        out.push(if l.is_head { '1' } else { '0' });
        out.push(if l.has_remote { '1' } else { '0' });
        let has_pr = matches!(l.kind, LabelKind::LocalBranch) && pr.contains(l.text.as_str());
        out.push(if has_pr { '1' } else { '0' });
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

/// The name on one chip record — [`label_names`] for the single record a
/// press or a right-click hands back.
pub fn label_name_of(record: &str) -> &str {
    label_names(record).next().unwrap_or("")
}

/// The ref kind a chip record's letter names, in the word the menus
/// branch on. The HEAD marker, the worktree marker (and anything
/// unrecognised) answer `""`: they name no ref, so there is nothing to
/// act on — a working copy is opened from its own row.
pub fn label_kind_word(record: &str) -> &'static str {
    match record.as_bytes().first() {
        Some(b'L') => "branch",
        Some(b'R') => "remote",
        Some(b'T') => "tag",
        _ => "",
    }
}

/// A chip's identity inside a gone set: its kind letter and the name on
/// it. The flag digits between the two say how the chip is drawn, and a
/// tag may share a name with a branch, so the letter stays part of the
/// key.
pub fn label_key(record: &str) -> String {
    let mut key = String::with_capacity(1 + record.len().saturating_sub(FLAGS + 1));
    key.push_str(record.get(..1).unwrap_or(""));
    key.push_str(label_name_of(record));
    key
}

/// The gone set itself: one key per kind, empty halves left out — a
/// delete touches at most one of each ([`label_key`] is the shape).
pub fn gone_keys(branch: &str, remote: &str, tag: &str) -> String {
    let mut out = String::new();
    for (letter, name) in [('L', branch), ('R', remote), ('T', tag)] {
        if name.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(RECORD_SEP);
        }
        out.push(letter);
        out.push_str(name);
    }
    out
}

/// The records still standing once the gone set has spoken: `packed` as
/// [`encode_labels`] wrote it, less the chips `gone` names. The window
/// says a deleted ref's chip is gone before the walk that follows the
/// delete lands (デザイン規約 §消す操作は先に画面から消す), and this is
/// where that word is applied.
pub fn labels_shown(packed: &str, gone: &str) -> String {
    if packed.is_empty() || gone.is_empty() {
        return packed.to_string();
    }
    let dropped: Vec<&str> = gone.split(RECORD_SEP).collect();
    packed
        .split(RECORD_SEP)
        .filter(|record| !dropped.contains(&label_key(record).as_str()))
        .collect::<Vec<_>>()
        .join(&RECORD_SEP.to_string())
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

    /// [`encode_labels`] with no branch wearing the PR badge — what every
    /// test here means when the set is not its subject.
    fn encode_no_pr(labels: &[RefLabel]) -> String {
        encode_labels(labels, &Default::default())
    }

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
                locked: false,
            },
            RefLabel {
                text: "v1.0".into(),
                kind: LabelKind::Tag,
                has_remote: false,
                is_head: false,
                here: true,
                remote: String::new(),
                held_elsewhere: false,
                locked: false,
            },
        ];
        assert_eq!(encode_no_pr(&labels), "L11010main\u{1f}T00010v1.0");
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
            locked: false,
        }];
        assert_eq!(encode_no_pr(&labels), "L00011feature/topic-a");
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
            locked: false,
        }];
        assert_eq!(encode_no_pr(&labels), "T01010v1.0");
    }

    /// The PR digit answers the set the caller passed — only a local
    /// branch wears it, and only when named. The set is an argument, so
    /// this test is a fact about its inputs alone.
    #[test]
    fn the_pr_digit_answers_the_named_branches() {
        let branch = |text: &str, kind| RefLabel {
            text: text.into(),
            kind,
            has_remote: false,
            is_head: false,
            here: true,
            remote: String::new(),
            held_elsewhere: false,
            locked: false,
        };
        let labels = [
            branch("topic", LabelKind::LocalBranch),
            branch("topic", LabelKind::Tag),
        ];
        let pr = std::collections::HashSet::from(["topic".to_string()]);
        assert_eq!(
            encode_labels(&labels, &pr),
            "L00110topic\u{1f}T00010topic",
            "the branch wears the digit; the tag sharing its name does not"
        );
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
            locked: false,
        }];
        assert_eq!(encode_no_pr(&labels), "T01000v9.9\u{1e}origin, fork");
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
                locked: false,
            },
            RefLabel {
                text: "v9.9".into(),
                kind: LabelKind::Tag,
                has_remote: true,
                is_head: false,
                here: false,
                remote: "origin, fork".into(),
                held_elsewhere: false,
                locked: false,
            },
        ];
        let encoded = encode_no_pr(&labels);
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
            locked: false,
        }];
        assert!(!encode_no_pr(&labels).contains(FIELD_SEP));
    }

    #[test]
    fn a_record_answers_its_kind_and_its_name() {
        assert_eq!(label_kind_word("L11010main"), "branch");
        assert_eq!(label_kind_word("R01000origin/main\u{1e}origin"), "remote");
        assert_eq!(label_kind_word("T00010v1.0"), "tag");
        assert_eq!(label_kind_word("H10010HEAD"), "", "nothing to act on");
        assert_eq!(label_kind_word("W00010spike"), "", "nor is a working copy");
        assert_eq!(label_kind_word(""), "");
        assert_eq!(
            label_name_of("R01000origin/main\u{1e}origin"),
            "origin/main"
        );
        assert_eq!(label_name_of("L11010main"), "main");
        assert_eq!(label_name_of("L11"), "", "flags cut short name nothing");
    }

    #[test]
    fn the_gone_set_takes_chips_out_by_kind_and_name() {
        let packed = "L11010main\u{1f}T00010main\u{1f}R01000origin/main\u{1e}origin";
        // A tag sharing the branch's name stays: the letter is part of
        // the identity.
        assert_eq!(
            labels_shown(packed, &gone_keys("main", "", "")),
            "T00010main\u{1f}R01000origin/main\u{1e}origin"
        );
        assert_eq!(
            labels_shown(packed, &gone_keys("main", "origin/main", "main")),
            ""
        );
        assert_eq!(labels_shown(packed, ""), packed);
        assert_eq!(labels_shown("", "Lmain"), "");
        assert_eq!(gone_keys("", "", ""), "");
        assert_eq!(gone_keys("main", "", "v1"), "Lmain\u{1f}Tv1");
    }
}
