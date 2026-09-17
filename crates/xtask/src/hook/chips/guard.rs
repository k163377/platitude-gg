//! What a chip is held to on its way onto the list, and what the list is
//! held to before a turn ends.

use std::collections::BTreeSet;

use super::Chip;
use super::at_hand;
use super::ledger::{load, store};
use crate::hook::payload::{deny, printable, string_field};

/// The mark a chip wears when it is a question:
/// the user decides whether the work is wanted at all, and may decide it
/// is not. It sits right after the number (`3. [任意] …`) and is spelled
/// exactly this way, because what it buys is a list read at a glance.
const OPTIONAL: &str = "[任意]";

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

/// PreToolUse(spawn_task): the three things a chip cannot be judged on
/// alone — where it stands, what weight it carries, and what it claims.
pub(crate) fn pre_spawn(input: &str) -> Result<(), String> {
    let asked = section(input, "\"tool_input\"");
    let title = string_field(asked, "title").unwrap_or_default();
    let Some((priority, body, optional)) = numbered(&title) else {
        deny(UNNUMBERED);
        return Ok(());
    };
    if misspelt_mark(&body) {
        deny(&format!("'{body}' {MISSPELT}"));
        return Ok(());
    }
    if asks(&body) && !optional {
        deny(&format!("'{body}' {UNMARKED}"));
        return Ok(());
    }
    let live = load(input);
    // A re-stack is the same chip under a new number, and it is how the
    // set is renumbered at all — nothing below applies to it. The
    // duplicate it leaves is cleared by the dismiss that follows.
    if live.iter().any(|chip| chip.body == body) {
        return Ok(());
    }
    let claimed = targets(&words(asked));
    // Asked before anything about the list is: a chip over work this
    // session is already holding is refused, whatever its number.
    if let Some(objection) = at_hand::objection(&session_cwd(input), &claimed) {
        deny(&objection);
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
    for chip in &live {
        let shared = shared(&claimed, &chip.targets);
        if shared.is_empty() {
            continue;
        }
        deny(&format!(
            "This chip claims {}, which the live chip '{}' claims too. Chips run in \
             parallel sessions, so two of them over one path land conflicting edits \
             — and a chip waiting on the other's outcome (a user decision included) \
             cannot run beside it at all. {} If this is that same chip re-weighed or \
             reworded, the title is all the guard knows it \
             by: dismiss_task the live one first, then stack this.",
            shared.join(", "),
            chip.label(),
            resolution(optional, chip.optional)
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

/// Stop: a turn ends only on a list that reads by its numbers.
///
/// It is judged here because a renumbering
/// passes through states no single call can approve — the replacement is
/// stacked before the chip it replaces is dismissed, so the set is
/// briefly two of everything. What matters is where it comes to rest.
pub(crate) fn stop(input: &str) -> Result<bool, String> {
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
             live set reads 1..N, each number once and each chip once, \
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
    let Some((priority, body, optional)) = numbered(&title) else {
        return;
    };
    let chip = Chip {
        id: id_in(section(input, "\"tool_response\"")).unwrap_or_default(),
        priority,
        body,
        optional,
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

/// The number a title leads with, the title left after it, and whether
/// it wears the weight mark.
fn numbered(title: &str) -> Option<(usize, String, bool)> {
    let (number, rest) = title.split_once('.')?;
    let priority: usize = number.trim().parse().ok()?;
    let rest = rest.trim();
    let (optional, body) = rest
        .strip_prefix(OPTIONAL)
        .map_or((false, rest), |body| (true, body.trim()));
    (priority >= 1 && !body.is_empty()).then(|| (priority, body.to_string(), optional))
}

/// What two chips over one path are told to do about it. There is no one
/// answer: it turns on what each of them weighs, because a chip asking
/// the user to decide and a chip proposing work can be neither folded
/// together nor ordered the same way.
fn resolution(claimed: bool, held: bool) -> &'static str {
    match (claimed, held) {
        (true, true) => {
            "Both of them ask the user to decide, so they are one chip: ask both \
             questions in it. Two chips put the same file in front of the user \
             twice and let the answer come back two ways."
        }
        (false, false) => {
            "Make them one chip, or leave this work to the first chip and have its \
             prompt end by stacking this follow-up itself, once its own work is done."
        }
        _ => {
            "One of the two asks the user to decide and the other proposes work on \
             the same file, so the question leads: the '[任意]' chip is the one that \
             stays live — a question is answered without touching anything — and its \
             prompt ends by stacking the work chip once the answer is in. \
             A session that works the file first has already picked \
             one of the answers, and a question folded into a work chip is not asked \
             until somebody starts that work."
        }
    }
}

/// Whether a title spelled the weight mark some other way, or reached
/// for a mark of its own — a tag nobody else writes is a column only
/// this chip has. The right spelling is off the title by now, so a `[…]`
/// still leading it is a second one; 任意 is looked for in the head
/// alone, since a chip may use the word in its own sentence further on.
fn misspelt_mark(body: &str) -> bool {
    body.starts_with(['[', '［']) || body.chars().take(8).collect::<String>().contains("任意")
}

/// The shapes a title takes when it asks and does not end on the asking.
const QUESTIONS: [&str; 4] = ["かどうか", "どちらが", "どちらを", "べきか"];

/// Whether a title asks a question. The user answers a
/// question and may answer no, which is the whole of what the mark
/// carries — a chip nobody is recommending yet.
fn asks(body: &str) -> bool {
    let body = body.trim_end_matches(['?', '？', '。', '.', ' ']);
    body.ends_with('か') || QUESTIONS.iter().any(|shape| body.contains(shape))
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

/// The targets two claims both cover — one chip's against another's, or
/// a chip's against what this session has open (`at_hand`). A directory
/// covers what is under it: a chip over `crates/platitude-app/src/ui`
/// and a chip over one .qml inside it are the same collision.
pub(super) fn shared(claimed: &BTreeSet<String>, held: &BTreeSet<String>) -> Vec<String> {
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

/// Where the session asking is standing, read from the payload's own
/// half: a chip may carry a `cwd` of its own
/// for another repository, and the first `"cwd"` in the input is the
/// answer `string_field` gives.
fn session_cwd(input: &str) -> String {
    let payload = input.split("\"tool_input\"").next().unwrap_or(input);
    string_field(payload, "cwd").unwrap_or_default()
}

/// The task id in `input`, however the harness quoted it. An MCP result
/// arrives as text with its JSON escaped inside, so the id is read as
/// the stable part — the first run of id characters after the key —
/// past whatever quoting sits around it.
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
        let mark = if self.optional {
            format!("{OPTIONAL} ")
        } else {
            String::new()
        };
        format!("{}. {mark}{}", self.priority, self.body)
    }
}

const UNNUMBERED: &str = "A task chip's title must lead with its priority: '1. …', \
     1 being the most important chip live right now. The number is the whole order \
     of the list, and a chip stacked without one is read wherever it happens to \
     land. Stack it again as '<n>. <title>' — or '<n>. [任意] <title>' when the chip \
     asks the user to decide — and if it outranks \
     chips already live, renumber those first: spawn each one again under its new \
     number, then dismiss its old task_id.";

const MISSPELT: &str = "says 任意 where the weight mark goes and does not spell it \
     '[任意]'. The mark is read at a glance, so a second spelling is invisible in \
     the list: it is exactly '[任意]', right after the number ('3. [任意] …'), and it \
     goes on a chip that asks the user to decide. Stack it again with that \
     spelling — or with no mark and the word out of the title's head, if the chip \
     is a recommendation.";

const UNMARKED: &str = "asks a question: the user answers it and may answer no, so \
     it weighs less than a chip proposing work. Stack it again as '<n>. [任意] \
     <title>', which is what the list is read by. If the chip really is \
     recommending something, reword the title as the work itself.";

#[cfg(test)]
mod tests {
    use super::{
        Chip, asks, covers, id_in, misspelt_mark, numbered, problems, resolution, shared, targets,
    };
    use std::collections::BTreeSet;

    fn chip(priority: usize, body: &str, targets: &[&str]) -> Chip {
        Chip {
            id: format!("t{priority}"),
            priority,
            body: body.to_string(),
            optional: false,
            targets: targets.iter().map(|target| (*target).to_string()).collect(),
        }
    }

    #[test]
    fn reads_the_number_a_title_leads_with() {
        assert_eq!(
            numbered("2. サイドバーの余白を直す"),
            Some((2, "サイドバーの余白を直す".to_string(), false))
        );
        assert_eq!(
            numbered("10.Fix v1.2 parsing"),
            Some((10, "Fix v1.2 parsing".to_string(), false))
        );
        assert_eq!(numbered("Fix the badge"), None);
        assert_eq!(numbered("0. nothing outranks 1"), None);
        assert_eq!(numbered("3. "), None);
    }

    #[test]
    fn takes_the_weight_mark_off_the_title_and_keeps_what_it_said() {
        assert_eq!(
            numbered("3. [任意] タグの並び順を日付にするか"),
            Some((3, "タグの並び順を日付にするか".to_string(), true))
        );
        // The body is the chip's identity, so the same work re-stacked
        // under the other weight is the same chip.
        assert_eq!(
            numbered("3. タグの並び順を日付にする").map(|(_, body, _)| body),
            numbered("4. [任意] タグの並び順を日付にする").map(|(_, body, _)| body)
        );
        assert_eq!(numbered("3. [任意]"), None);
    }

    #[test]
    fn holds_the_mark_to_one_spelling_and_asks_for_it_on_a_question() {
        assert!(misspelt_mark("(任意) 余白を詰める"));
        assert!(misspelt_mark("任意: 余白を詰める"));
        // A tag of the chip's own invention is a column only it has.
        assert!(misspelt_mark("[要判断] 余白を詰める"));
        assert!(misspelt_mark("［任意］余白を詰める"));
        // Stripped already when the spelling was right, and a chip may
        // still say the word in its own sentence further along.
        assert!(!misspelt_mark("余白を詰める"));
        assert!(!misspelt_mark("並び順の既定を任意に変えられるようにする"));
        // A parenthetical that is not reaching for a mark is left be.
        assert!(!misspelt_mark("(macOS だけ) 余白を詰める"));

        assert!(asks("タグの並び順を日付にするか"));
        assert!(asks("アバターを縮めるか？"));
        // A question that does not end on its own question word.
        assert!(asks("タグを畳むかどうかを決める"));
        assert!(asks("どちらを既定にするかを決める"));
        assert!(asks("バッジを出すべきか決める"));
        assert!(!asks("タグの並び順を日付にする"));
        assert!(!asks("Sidebar.qml の余白を詰める"));
    }

    #[test]
    fn tells_two_chips_over_one_path_what_to_do_by_what_they_weigh() {
        // A question and a proposal are ordered, and the question leads.
        assert!(resolution(true, false).contains("[任意]"));
        assert!(resolution(false, true).contains("[任意]"));
        assert_eq!(resolution(true, false), resolution(false, true));
        // Two questions are one question; two proposals are merged or
        // serialized. Neither borrows the other's answer.
        assert!(resolution(true, true).contains("one chip"));
        assert_ne!(resolution(true, true), resolution(false, false));
        assert_ne!(resolution(false, false), resolution(true, false));
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
