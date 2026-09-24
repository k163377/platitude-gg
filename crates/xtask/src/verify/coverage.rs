//! `cargo xtask verbs` — the verbs the gate skips.
//!
//! The gate runs the argument lines the census holds, so **a verb with no
//! line in the census is one no change ever re-photographs** — whatever
//! the `must_say` table says about how it would be judged if it ran
//! (verify-ui skill: 「census に行の無い動詞は gate が一度も回さない」).
//! Nothing counted that difference, and it is not a difference a person
//! can count by eye: the verbs are spread over sixteen families and the
//! census is four hundred lines.
//!
//! **Both sides are read from the tree.** The verbs are what the harness
//! compares `PGG_AUTO_ACT` against, which is the only thing that decides
//! whether a name is answered at all; the recorded side is the census's
//! own first word per line. The skill's prose undercounts
//! — its verbs live inside sentences beside every other backticked word,
//! and a count taken off it was out by an order of magnitude.
//!
//! The other direction is an invariant: a line
//! whose verb the harness no longer names is one the gate still runs,
//! and the run spends its whole ceiling doing nothing (the harness
//! ignores a name it does not know). So that line fails this command,
//! while unrecorded verbs are reported and counted.
//!
//! **The two halves are judged on different readings, deliberately.**
//! The count is of dispatches, which is what a verb is; the failure is
//! against every kebab-case word the harness holds, because a dispatch
//! is not the only way one is reached — the file-row family asks whether
//! the act is in a list it keeps. Failing on the narrower reading would
//! turn "the harness writes this one differently" into a red gate for
//! whoever wrote it, and this runs in every gate on the machine.

use std::collections::BTreeSet;
use std::path::Path;

/// Where the harness answers a verb by name. Nothing outside this
/// directory reads `PGG_AUTO_ACT`'s value as a verb.
const HARNESS: &str = "crates/platitude-app/src/auto/";

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (verbs takes none)"));
    }
    let root = crate::tree::workspace_root();
    let harness = harness(&root)?;
    let recorded = recorded(&root);
    // The tier table is read by every gate to choose what it owes, so a
    // row that went stale is a choice made on a line that is not there —
    // or a full line whose pre-merge lean moved, which leaves its claim with
    // no witness before a merge (`gate::tiers`).
    let census = crate::gate::Census::load(&root);
    let tiers = crate::gate::Tiers::load(&root);
    // A verb whose every row is a twin of another verb's line has no line
    // on purpose: it is no backlog to record.
    let twins_only = tiers.twinned_away();

    let unrecorded: Vec<&String> = harness
        .answered
        .difference(&recorded)
        .filter(|verb| !twins_only.contains(*verb))
        .collect();
    // Judged against every name the harness holds: a verb reached
    // through a list this does not read
    // is answered all the same, and a gate that failed on one would be
    // blaming a census line for how the harness was written.
    let gone: Vec<&String> = recorded.difference(&harness.mentioned).collect();

    println!("verbs the harness answers: {}", harness.answered.len());
    println!("verbs the census records:  {}", recorded.len());
    println!("unrecorded in the census:  {}", unrecorded.len());
    for verb in &unrecorded {
        println!("  {verb}");
    }
    let (linux, full, twins) = tiers.counts();
    println!(
        "census lines before a merge: {} (host), {linux} of them on the container too; {full} \
         for the full gate only; {twins} twin(s) no gate runs",
        census
            .lines
            .keys()
            .filter(|line| tiers.owed(line, false))
            .count()
    );
    let complaints = tiers.complaints(&census);
    if !complaints.is_empty() {
        return Err(format!(
            "the tier table does not agree with the census:\n{}",
            complaints
                .iter()
                .map(|c| format!("  {c}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if gone.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the census holds {} line(s) whose verb the harness no longer names — the gate runs \
         them and the run waits out its whole ceiling saying nothing. Drop the line (the one \
         edit the census takes by hand) or restore the name: {}",
        gone.len(),
        gone.iter()
            .map(|verb| verb.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

/// What the harness says about verb names: the ones it compares an act
/// against, and every kebab-case word it holds at all.
struct Harness {
    /// Names a dispatch asks for outright.
    answered: BTreeSet<String>,
    /// Every such word in the harness, whatever it is written in — a
    /// dispatch, a list the dispatch reads (`AutoActDriver.fileRowActs`),
    /// a completion table. **What the failing half is judged on**, since
    /// a name written anywhere here is one somebody may still be
    /// answering by a road this does not read.
    mentioned: BTreeSet<String>,
}

fn harness(root: &Path) -> Result<Harness, String> {
    let mut harness = Harness {
        answered: BTreeSet::new(),
        mentioned: BTreeSet::new(),
    };
    let mut files = 0;
    for file in crate::gate::qml_files(root)? {
        if !file.starts_with(HARNESS) {
            continue;
        }
        files += 1;
        let text = std::fs::read_to_string(root.join(&file))
            .map_err(|e| format!("could not read {file}: {e}"))?;
        harness.answered.extend(verbs_in(&text));
        harness.mentioned.extend(words_in(&text));
    }
    if files == 0 {
        // Every census line would read as stale, which is a scan that
        // found nothing.
        return Err(format!(
            "no QML under {HARNESS} — the harness is not where this expects it, so nothing \
             here can be said about which verbs are answered"
        ));
    }
    Ok(harness)
}

/// The verb each census line ran — its first word, the rest being the
/// argument and the flags that line was taken with.
fn recorded(root: &Path) -> BTreeSet<String> {
    crate::gate::Census::load(root)
        .lines
        .keys()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// The names compared against in one file. A dispatch is written two
/// ways — the page's families take the act as `act`, and the window's
/// read `Harness.autoAct` where they stand — and both are the same
/// question: does this run answer to that name. Either may be reached
/// through whatever holds it (`planOpenTimer.act`), so what is asked is
/// read off the last word.
fn verbs_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (at, _) in text.match_indices("===") {
        let asked = text[..at]
            .trim_end()
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
            .next()
            .unwrap_or_default();
        let asked = asked.rsplit('.').next().unwrap_or_default();
        if asked != "act" && asked != "autoAct" {
            continue;
        }
        let Some(rest) = text[at + 3..].trim_start().strip_prefix('"') else {
            continue;
        };
        let Some(end) = rest.find('"') else {
            continue;
        };
        let name = &rest[..end];
        if is_verb(name) {
            found.insert(name.to_string());
        }
    }
    found
}

/// Every word in one file that could be a verb name, wherever it
/// stands. A dispatch is not the only place one is written: the file-row
/// family reads a list and asks whether the act is in it, and a name
/// added to such a list and nowhere else is still answered.
fn words_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else {
            break;
        };
        if is_verb(&after[..close]) {
            found.insert(after[..close].to_string());
        }
        rest = &after[close + 1..];
    }
    found
}

/// What a verb name looks like: the kebab-case word a run is started
/// with. The same comparison is made against an argument's own words
/// (`"go"`, `"older"`, a preset's name with a colon in it), which are
/// read out of an act that already matched.
fn is_verb(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && name.contains(|c: char| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{is_verb, verbs_in, words_in};

    /// An act reached through whatever holds it is the same dispatch.
    /// Read off the whole expression it would be missed, and a verb
    /// written only that way would read as a census line nobody answers.
    #[test]
    fn an_act_is_asked_for_through_its_holder_too() {
        let text = r#"
            if (planOpenTimer.act === "rebase-plan") { return }
            running: driver.autoAct === "band"
        "#;
        let found = verbs_in(text);
        assert_eq!(
            found.iter().map(String::as_str).collect::<Vec<_>>(),
            ["band", "rebase-plan"]
        );
    }

    /// The list a family reads: no dispatch names
    /// these, and the failing half must still count them as answered.
    #[test]
    fn a_name_in_a_list_is_a_name_the_harness_holds() {
        let text = r#"
            readonly property var fileRowActs: [
                "stage-many", "file-menu-staged", "open-mergetool"
            ]
        "#;
        assert!(verbs_in(text).is_empty());
        assert_eq!(
            words_in(text)
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["file-menu-staged", "open-mergetool", "stage-many"]
        );
    }

    /// Both dispatch shapes, and nothing else from the same line.
    #[test]
    fn the_two_ways_a_dispatch_is_written_are_both_read() {
        let text = r#"
            if (act === "row-card") {
                page.open()
            } else if (act === "ref-list" || act === "ref-list-card") {
                page.list()
            }
            running: Harness.autoAct === "band"
            readonly property bool mine: autoAct === "window-fill"
        "#;
        let found = verbs_in(text);
        assert_eq!(
            found.iter().map(String::as_str).collect::<Vec<_>>(),
            [
                "band",
                "ref-list",
                "ref-list-card",
                "row-card",
                "window-fill"
            ]
        );
    }

    /// A comparison that is not the act's, and an act compared with one
    /// of its own argument's words: neither names a verb.
    #[test]
    fn what_is_compared_decides_whether_it_is_a_verb() {
        let text = r#"
            if (arg === "go" || mode === "diverged") { return }
            if (act === "delete-stash-row" && Harness.autoActArg === "go") { press() }
        "#;
        let found = verbs_in(text);
        assert_eq!(
            found.iter().map(String::as_str).collect::<Vec<_>>(),
            ["delete-stash-row"]
        );
    }

    /// The shape of a name, which is what tells a verb from the rest of
    /// what an act is compared against.
    #[test]
    fn a_verb_is_a_kebab_case_word() {
        assert!(is_verb("plan-reword-ask"));
        assert!(is_verb("badges"));
        assert!(!is_verb(""));
        assert!(!is_verb("1"));
        assert!(!is_verb("tag:v0.1"));
        assert!(!is_verb("RowCard"));
    }
}
