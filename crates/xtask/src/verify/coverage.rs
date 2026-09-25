//! `cargo xtask verbs` — the verbs the gate never runs (verify-ui skill
//! 「census に行の無い動詞は gate が一度も回さない」).
//!
//! Both sides are read from the tree: the names the harness compares
//! `PGG_AUTO_ACT` against, and each census line's first word — not the
//! skill's prose, which undercounts.
//!
//! Unrecorded verbs are counted. A census line whose verb the harness no
//! longer names fails: the gate still runs it, and it waits out its whole
//! ceiling. That failure is judged against every kebab-case word the
//! harness holds, not only dispatches — a verb can be reached through a
//! list (the file-row family), and the narrower reading would turn every
//! gate red over how the harness spells one.

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
    let census = crate::gate::Census::load(&root)?;
    let recorded = recorded(&census);
    // Every gate chooses what it owes off this table, so its rows are held
    // to the census below (`gate::tiers`).
    let tiers = crate::gate::Tiers::load(&root)?;
    // A verb whose every row is a twin has no line on purpose.
    let twins_only = tiers.twinned_away();

    let unrecorded: Vec<&String> = harness
        .answered
        .difference(&recorded)
        .filter(|verb| !twins_only.contains(*verb))
        .collect();
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

struct Harness {
    /// Names a dispatch asks for outright.
    answered: BTreeSet<String>,
    /// Every kebab-case word in the harness, lists
    /// (`AutoActDriver.fileRowActs`) and completion tables included.
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
        // Otherwise every census line would read as stale.
        return Err(format!(
            "no QML under {HARNESS} — the harness is not where this expects it, so nothing \
             here can be said about which verbs are answered"
        ));
    }
    Ok(harness)
}

/// Each census line's verb: its first word.
fn recorded(census: &crate::gate::Census) -> BTreeSet<String> {
    census
        .lines
        .keys()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// The names `act` (the page's families) or `autoAct` (the window's) is
/// compared against in one file, read off the last word so
/// `planOpenTimer.act` counts too.
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

/// Every quoted word in one file that could be a verb name, wherever it
/// stands.
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

/// The shape of a verb name: kebab-case with a letter in it. Argument
/// words (`"go"`) share it; only what they are compared with sets them
/// apart ([`verbs_in`]).
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

    /// Missed, a verb dispatched only this way would drop out of the count
    /// of answered verbs.
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

    /// No dispatch names these, and the failing half must still count them.
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
