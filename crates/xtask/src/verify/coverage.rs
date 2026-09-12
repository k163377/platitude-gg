//! `cargo xtask verbs` — which verbs the gate never runs.
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
//! own first word per line. Reading the skill's prose instead undercounts
//! — its verbs live inside sentences beside every other backticked word,
//! and a count taken off it was out by an order of magnitude.
//!
//! The other direction is an invariant rather than a backlog: a line
//! whose verb no name answers is one the gate still runs, and the run
//! spends its whole ceiling doing nothing (the harness ignores a name it
//! does not know). So an unanswered line fails this command, while
//! unrecorded verbs are reported and counted.

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
    let answered = answered(&root)?;
    let recorded = recorded(&root);

    let unrecorded: Vec<&String> = answered.difference(&recorded).collect();
    let unanswered: Vec<&String> = recorded.difference(&answered).collect();

    println!("verbs the harness answers: {}", answered.len());
    println!("verbs the census records:  {}", recorded.len());
    println!("never run by the gate:     {}", unrecorded.len());
    for verb in &unrecorded {
        println!("  {verb}");
    }
    if unanswered.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the census holds {} line(s) no verb answers — the gate runs them and the run \
         waits out its whole ceiling doing nothing. Drop the line or restore the name: {}",
        unanswered.len(),
        unanswered
            .iter()
            .map(|verb| verb.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

/// Every verb name the harness compares against.
fn answered(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    for file in crate::gate::qml_files(root)? {
        if !file.starts_with(HARNESS) {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&file))
            .map_err(|e| format!("could not read {file}: {e}"))?;
        names.extend(verbs_in(&text));
    }
    Ok(names)
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
/// question: does this run answer to that name.
fn verbs_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (at, _) in text.match_indices("===") {
        let asked = text[..at]
            .trim_end()
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
            .next()
            .unwrap_or_default();
        if asked != "act" && !asked.ends_with(".autoAct") && asked != "autoAct" {
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

/// What a verb name looks like: the kebab-case word a run is started
/// with. The same comparison is made against an argument's own words
/// (`"go"`, `"older"`, a preset's name with a colon in it), and those are
/// not verbs — they are read out of an act that already matched.
fn is_verb(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && name.contains(|c: char| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{is_verb, verbs_in};

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
