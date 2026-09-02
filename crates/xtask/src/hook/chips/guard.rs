//! What a chip is held to on its way onto the list, and what the list is
//! held to before a turn ends.

use std::collections::BTreeSet;

use super::Chip;
use super::ledger::{load, store};
use crate::hook::payload::{bool_field, string_field};

/// The directories a path is recognized by when it names no file at all:
/// `crates/platitude-app/src/ui` is a claim, `Windows / macOS` is not.
const ROOTS: [&str; 6] = [
    "crates",
    ".claude",
    "internal-docs",
    "ci",
    "packaging",
    "spike",
];

/// PreToolUse(spawn_task): the two things a chip cannot be judged on
/// alone — where it stands, and what it claims.
pub(crate) fn pre_spawn(input: &str) -> Result<(), String> {
    let asked = section(input, "\"tool_input\"");
    let title = string_field(asked, "title").unwrap_or_default();
    let Some((priority, body)) = numbered(&title) else {
        deny(UNNUMBERED);
        return Ok(());
    };
    let live = load(input);
    // A re-stack is the same chip under a new number, and it is how the
    // set is renumbered at all — nothing below applies to it. The
    // duplicate it leaves is cleared by the dismiss that follows.
    if live.iter().any(|chip| chip.body == body) {
        return Ok(());
    }
    if let Some(taken) = live.iter().find(|chip| chip.priority == priority) {
        deny(&format!(
            "Priority {priority} is already the live chip '{}'. Two chips at one \
             number leave the list with no order to read. Renumber the whole live \
             set around where this one belongs first — for each chip that moves, \
             spawn it again under its new number and then dismiss its old task_id \
             — and stack this chip once its number is free. Live now: {}.",
            taken.label(),
            roster(&live)
        ));
        return Ok(());
    }
    let claimed = targets(&words(asked));
    for chip in &live {
        let shared = shared(&claimed, &chip.targets);
        if shared.is_empty() {
            continue;
        }
        deny(&format!(
            "This chip claims {}, which the live chip '{}' claims too. Chips run in \
             parallel sessions, so two of them over one path land conflicting edits \
             — and a chip waiting on the other's outcome (a user decision included) \
             cannot run beside it at all. Make them one chip, or leave this work to \
             the first chip and have its prompt end by stacking this follow-up \
             itself, once its own answer is in.",
            shared.join(", "),
            chip.label()
        ));
        return Ok(());
    }
    Ok(())
}

/// PostToolUse(spawn_task|dismiss_task): the ledger follows what the
/// list actually holds.
pub(crate) fn post_chip(input: &str) -> Result<(), String> {
    let tool = string_field(input, "tool_name").unwrap_or_default();
    if tool.contains("dismiss_task") {
        dismissed(input);
    } else if tool.contains("spawn_task") {
        spawned(input);
    }
    Ok(())
}

/// Stop: the turn does not end on a list nobody can read by its numbers.
///
/// It is judged here rather than at each spawn because a renumbering
/// passes through states no single call can approve — the replacement is
/// stacked before the chip it replaces is dismissed, so the set is
/// briefly two of everything. What matters is where it comes to rest.
pub(crate) fn stop(input: &str) -> Result<bool, String> {
    // A block already asked once. Asking again on the answer to it is
    // how a session is wedged rather than corrected.
    if bool_field(input, "stop_hook_active") == Some(true) {
        return Ok(false);
    }
    let live = load(input);
    let problems = problems(&live);
    if problems.is_empty() {
        return Ok(false);
    }
    println!(
        "{{\"decision\":\"block\",\"reason\":\"{}\"}}",
        printable(&format!(
            "The task chips this session stacked are left out of order: {}. Live \
             now: {}. The list is read by its numbers, so before a turn ends the \
             live set reads 1..N with no gap, no number twice and no chip twice, \
             1 being the most important. Re-stack whatever moved — spawn the chip \
             again under its new number, then dismiss its old task_id — and say in \
             the reply which chips changed number.",
            problems.join("; "),
            roster(&live)
        ))
    );
    Ok(true)
}

/// Records a chip that was actually stacked, and says out loud what the
/// stacking leaves undone.
fn spawned(input: &str) {
    let asked = section(input, "\"tool_input\"");
    let title = string_field(asked, "title").unwrap_or_default();
    let Some((priority, body)) = numbered(&title) else {
        return;
    };
    let chip = Chip {
        id: id_in(section(input, "\"tool_response\"")).unwrap_or_default(),
        priority,
        body,
        targets: targets(&words(asked)),
    };
    let mut live = load(input);
    let mut notes: Vec<String> = Vec::new();
    if let Some(old) = live.iter().find(|held| held.body == chip.body) {
        notes.push(format!(
            "'{}' is now stacked twice: dismiss_task the old one ({}) before this \
             turn ends — until then the list carries both.",
            chip.body, old.id
        ));
    }
    if chip.targets.is_empty() {
        notes.push(
            "This chip names no path, so no later chip can be held off what it \
             touches. A prompt that names the files it will work on is what keeps \
             the next chip out of them."
                .to_string(),
        );
    }
    live.push(chip);
    store(input, &live);
    if !notes.is_empty() {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
             \"additionalContext\":\"{}\"}}}}",
            printable(&notes.join(" "))
        );
    }
}

/// Drops a chip the session withdrew. A dismiss the harness refused (the
/// user got there first) drops it too: either way it is no longer a chip
/// waiting to be run.
fn dismissed(input: &str) {
    let Some(id) = id_in(section(input, "\"tool_input\"")) else {
        return;
    };
    let mut live = load(input);
    let before = live.len();
    live.retain(|chip| chip.id != id);
    if live.len() != before {
        store(input, &live);
    }
}

/// What is wrong with the live set, in words. Pure so the tests can ask.
fn problems(live: &[Chip]) -> Vec<String> {
    if live.is_empty() {
        return Vec::new();
    }
    let mut problems = Vec::new();
    let mut numbers: Vec<usize> = live.iter().map(|chip| chip.priority).collect();
    numbers.sort_unstable();
    if numbers != (1..=live.len()).collect::<Vec<usize>>() {
        problems.push(format!(
            "{} chips numbered {} where they must read 1..{}",
            live.len(),
            numbers
                .iter()
                .map(usize::to_string)
                .collect::<Vec<String>>()
                .join(" "),
            live.len()
        ));
    }
    let mut bodies: Vec<&str> = live.iter().map(|chip| chip.body.as_str()).collect();
    bodies.sort_unstable();
    let twice: Vec<&str> = bodies
        .windows(2)
        .filter(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
        .collect();
    if !twice.is_empty() {
        problems.push(format!("stacked twice: {}", twice.join(", ")));
    }
    problems
}

/// The number a title leads with, and the title without it.
fn numbered(title: &str) -> Option<(usize, String)> {
    let (number, rest) = title.split_once('.')?;
    let priority: usize = number.trim().parse().ok()?;
    let body = rest.trim();
    (priority >= 1 && !body.is_empty()).then(|| (priority, body.to_string()))
}

/// The paths a chip's words name. A chip prompt is written to stand
/// alone, so the files it will work on are in it — which is the only
/// thing two chips can be compared on before either has run.
fn targets(text: &str) -> BTreeSet<String> {
    text.split(|c: char| c.is_whitespace() || "\"'`(),、。「」【】[]<>|".contains(c))
        .filter_map(one_target)
        .collect()
}

fn one_target(token: &str) -> Option<String> {
    let token = token.replace('\\', "/");
    // A `:42` line suffix is not part of the path, and a URL loses its
    // whole token to its scheme — a link is not a file to collide over.
    let path = token
        .split(':')
        .next()?
        .trim_end_matches(['.', ',', ';', '、', '。', '/']);
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 || segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    let names_a_file = segments.last().is_some_and(|last| last.contains('.'));
    (names_a_file || ROOTS.contains(&segments[0])).then(|| path.to_string())
}

/// The targets two chips both claim. A directory covers what is under
/// it: a chip over `crates/platitude-app/src/ui` and a chip over one
/// .qml inside it are the same collision.
fn shared(claimed: &BTreeSet<String>, held: &BTreeSet<String>) -> Vec<String> {
    claimed
        .iter()
        .filter(|one| {
            held.iter()
                .any(|other| covers(one, other) || covers(other, one))
        })
        .cloned()
        .collect()
}

fn covers(wide: &str, narrow: &str) -> bool {
    narrow == wide
        || narrow
            .strip_prefix(wide)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The words of a chip: its title, its one-line summary and its prompt.
fn words(asked: &str) -> String {
    ["title", "tldr", "prompt"]
        .iter()
        .filter_map(|field| string_field(asked, field))
        .collect::<Vec<String>>()
        .join(" ")
}

/// The payload from `key` on. Tool input and tool response carry fields
/// under the same names, and a chip's own prompt can quote either —
/// reading from the section that owns the field keeps a quoted word out
/// of the answer.
fn section<'a>(input: &'a str, key: &str) -> &'a str {
    input.find(key).map_or(input, |at| &input[at..])
}

/// The task id in `input`, however the harness quoted it. An MCP result
/// arrives as text with its JSON escaped inside, so the punctuation
/// around the key is not worth matching on; the id is the stable part —
/// the first run of id characters after the key.
fn id_in(input: &str) -> Option<String> {
    const KEY: &str = "task_id";
    let tail = &input[input.find(KEY)? + KEY.len()..];
    let start = tail.find(|c: char| c.is_ascii_alphanumeric())?;
    let id: String = tail[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (!id.is_empty()).then_some(id)
}

/// The live set as the user reads it.
fn roster(live: &[Chip]) -> String {
    let mut sorted: Vec<&Chip> = live.iter().collect();
    sorted.sort_by_key(|chip| chip.priority);
    sorted
        .iter()
        .map(|chip| format!("'{}'", chip.label()))
        .collect::<Vec<String>>()
        .join(", ")
}

impl Chip {
    /// The chip as its title reads.
    fn label(&self) -> String {
        format!("{}. {}", self.priority, self.body)
    }
}

fn deny(reason: &str) {
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{}\"}}}}",
        printable(reason)
    );
}

const UNNUMBERED: &str = "A task chip's title must lead with its priority: '1. …', \
     1 being the most important chip live right now. The number is the whole order \
     of the list, and a chip stacked without one is read wherever it happens to \
     land. Stack it again as '<n>. <title>', and if it outranks chips already live, \
     renumber those first — spawn each one again under its new number, then dismiss \
     its old task_id.";

/// A string sanitized for the hand-built JSON it rides in: everything
/// that could end the string or the payload early is dropped.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '"' | '\\' => '\'',
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Chip, covers, id_in, numbered, problems, shared, targets};
    use std::collections::BTreeSet;

    fn chip(priority: usize, body: &str, targets: &[&str]) -> Chip {
        Chip {
            id: format!("t{priority}"),
            priority,
            body: body.to_string(),
            targets: targets.iter().map(|target| (*target).to_string()).collect(),
        }
    }

    #[test]
    fn reads_the_number_a_title_leads_with() {
        assert_eq!(
            numbered("2. サイドバーの余白を直す"),
            Some((2, "サイドバーの余白を直す".to_string()))
        );
        assert_eq!(
            numbered("10.Fix v1.2 parsing"),
            Some((10, "Fix v1.2 parsing".to_string()))
        );
        assert_eq!(numbered("Fix the badge"), None);
        assert_eq!(numbered("0. nothing outranks 1"), None);
        assert_eq!(numbered("3. "), None);
    }

    #[test]
    fn picks_paths_out_of_a_chip_and_leaves_prose_alone() {
        let found = targets(
            "`crates/platitude-app/src/ui/Sidebar.qml:42` と \
             internal-docs/実装計画.md を直す(Windows / macOS 両方)。\
             見るのは https://example.com/a.html と session::RefJoins、\
             それと crates/platitude-core/src/refs",
        );
        let want: BTreeSet<String> = [
            "crates/platitude-app/src/ui/Sidebar.qml",
            "internal-docs/実装計画.md",
            "crates/platitude-core/src/refs",
        ]
        .iter()
        .map(|target| (*target).to_string())
        .collect();
        assert_eq!(found, want);
    }

    #[test]
    fn a_directory_covers_what_is_under_it() {
        assert!(covers("crates/x/src/ui", "crates/x/src/ui/Sidebar.qml"));
        assert!(!covers("crates/x/src/ui", "crates/x/src/uiSidebar.qml"));
        let claimed = targets("crates/x/src/ui/Sidebar.qml を直す");
        let held = targets("crates/x/src/ui を通しで見る");
        assert_eq!(shared(&claimed, &held), vec!["crates/x/src/ui/Sidebar.qml"]);
        assert!(shared(&claimed, &targets("crates/x/src/refs.rs")).is_empty());
    }

    #[test]
    fn calls_a_set_readable_only_when_it_reads_one_to_n() {
        assert!(problems(&[]).is_empty());
        assert!(problems(&[chip(1, "a", &[]), chip(2, "b", &[])]).is_empty());
        assert_eq!(problems(&[chip(2, "a", &[])]).len(), 1);
        assert_eq!(problems(&[chip(1, "a", &[]), chip(1, "b", &[])]).len(), 1);
        // The pass through a re-stack: the same chip under both numbers,
        // until the old one is dismissed.
        assert_eq!(problems(&[chip(1, "a", &[]), chip(2, "a", &[])]).len(), 1);
    }

    #[test]
    fn finds_a_task_id_however_the_result_quoted_it() {
        assert_eq!(
            id_in(r#"{"task_id":"abc-123"}"#),
            Some("abc-123".to_string())
        );
        assert_eq!(
            id_in(r#"{"content":[{"text":"{\"task_id\": \"abc_9\"}"}]}"#),
            Some("abc_9".to_string())
        );
        assert_eq!(id_in("nothing here"), None);
    }
}
