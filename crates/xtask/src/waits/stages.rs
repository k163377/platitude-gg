//! The stages rule of the app's harness: a run's stages move only
//! through its operations (`auto/StageTimer.qml` — 反映前テストの機械化
//! §検証時点), and a run completes and reports only through them. Read as
//! allowed structure, not as exceptions to a grammar: what a harness file
//! may hold is named, and anything else is a finding no marker lets
//! stand.
//!
//! - `sampler`: every sampler is an operation — a `PageStage` or a
//!   `WindowStage` — outside the files that keep the run's own machinery
//!   ([`SAMPLER_KEEPERS`]): the completion's hold, the ceiling, the
//!   measurement drivers.
//! - `owner`: an operation is the window's in a `Window*` file and the
//!   page's anywhere else; `StageTimer` itself is no operation's type.
//! - `report`: only the machinery says a line ([`REPORTERS`]); a verb's
//!   lines are read by its operation — at a transition, or by the
//!   completion's route once the completion's hold has passed.
//! - `completion`: only the routes end a run ([`COMPLETERS`]).
//! - `awaited`: a verb reads `Awaited` for what it observes and claims
//!   ([`AWAITED_FOR_VERBS`]); its stages, records and routes are the
//!   machinery's.
//! - `stage`: inside an operation's body, no state of the harness's own —
//!   a property of the file, or of `driver` / `acts` — is given a
//!   constant, stepped or added to: that is a stage, and stages are the
//!   operation's (`at` / `advance` / `input` / `complete`). A value read
//!   off the product (`was = driver.commandsWrongSeen`) is a reading, and
//!   stands; so does a local.

use super::Finding;
use crate::lex::{Lang, Token, tokens};

/// The files that keep plain samplers of their own: the operation's type,
/// the completion's hold, the ceiling, and the measurement drivers, whose
/// beats are no run's stages.
const SAMPLER_KEEPERS: [&str; 5] = [
    "StageTimer.qml",
    "CompletionRoute.qml",
    "AutoShotDriver.qml",
    "PagePerfDriver.qml",
    "WindowPerfDriver.qml",
];

/// The files that say lines: the machinery of stages and completion, the
/// pictures' and the census's lines, the page's driver (what it dispatched,
/// a broken write), the product's edges as they come (`PageReports`), and
/// the measurement drivers.
const REPORTERS: [&str; 8] = [
    "Awaited.qml",
    "CompletionRoute.qml",
    "AutoShotDriver.qml",
    "WindowCensus.qml",
    "AutoActDriver.qml",
    "PageReports.qml",
    "PagePerfDriver.qml",
    "WindowPerfDriver.qml",
];

/// The files that end a run: the completion's route, and the page's
/// driver, which ends a run its write contract broke.
const COMPLETERS: [&str; 2] = ["WindowHarness.qml", "AutoActDriver.qml"];

/// The files of the machinery itself, which read all of `Awaited`.
const MACHINERY: [&str; 7] = [
    "Awaited.qml",
    "StageTimer.qml",
    "CompletionRoute.qml",
    "WindowHarness.qml",
    "AutoShotDriver.qml",
    "WindowCensus.qml",
    "AutoActDriver.qml",
];

/// What a verb reads off `Awaited`: an observation, a diagnostic line, a
/// list built, a claim on the pictures, the run's id.
const AWAITED_FOR_VERBS: [&str; 5] = ["all", "note", "built", "claim", "runId"];

/// The findings of the stages rule in one harness file.
pub(super) fn findings(file: &str, text: &str) -> Vec<Finding> {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let tokens = tokens(text, Lang::Qml);
    let raw: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut found = |line: usize, rule: &'static str| {
        out.push(Finding {
            file: file.to_string(),
            line,
            rule,
            excerpt: raw
                .get(line.saturating_sub(1))
                .map(|l| l.trim().to_string())
                .unwrap_or_default(),
        });
    };
    let ids = ids(&tokens);
    for (at, token) in tokens.iter().enumerate() {
        let next = |n: usize| tokens.get(at + n).map(|t| t.text.as_str());
        let before = at.checked_sub(1).map(|b| tokens[b].text.as_str());
        let object = next(1) == Some("{") && before != Some(".");
        match token.text.as_str() {
            "SampleTimer" | "Timer" if object && !SAMPLER_KEEPERS.contains(&name) => {
                found(token.line, "sampler");
            }
            "StageTimer" if object && name != "PageStage.qml" && name != "WindowStage.qml" => {
                found(token.line, "owner");
            }
            "PageStage" if object && name.starts_with("Window") => found(token.line, "owner"),
            "WindowStage" if object && !name.starts_with("Window") => found(token.line, "owner"),
            "Harness"
                if next(1) == Some(".")
                    && next(2) == Some("report")
                    && !REPORTERS.contains(&name) =>
            {
                found(token.line, "report");
            }
            "finishAutoAct" if before == Some(".") && !COMPLETERS.contains(&name) => {
                found(token.line, "completion");
            }
            // The page driver's own completion is its operations' (`dispatchFinished`, the barriers).
            "driver"
                if next(1) == Some(".") && next(2) == Some("complete") && next(3) == Some("(") =>
            {
                found(token.line, "completion");
            }
            "Awaited"
                if next(1) == Some(".")
                    && before != Some(".")
                    && !MACHINERY.contains(&name)
                    && next(2).is_some_and(|member| !AWAITED_FOR_VERBS.contains(&member)) =>
            {
                found(token.line, "awaited");
            }
            _ => {}
        }
    }
    for (open, close) in operations(&tokens) {
        for line in stage_writes(&tokens[open..close], &ids) {
            found(line, "stage");
        }
    }
    out
}

/// The `id:`s the file declares, and the two names a verb family reaches
/// the drivers' own state by.
fn ids(tokens: &[Token]) -> Vec<String> {
    let mut out: Vec<String> = ["driver", "acts"].map(String::from).to_vec();
    for pair in tokens.windows(3) {
        if pair[0].text == "id" && pair[1].text == ":" {
            out.push(pair[2].text.clone());
        }
    }
    out
}

/// Each operation's body (`PageStage {` / `WindowStage {` to its matching
/// brace), as token indices.
fn operations(tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        let is_operation = matches!(token.text.as_str(), "PageStage" | "WindowStage")
            && tokens.get(at + 1).is_some_and(|t| t.text == "{");
        if !is_operation {
            continue;
        }
        if let Some(close) = crate::lex::matching_close(tokens, at + 1, "{", "}") {
            out.push((at + 2, close));
        }
    }
    out
}

/// Whether `token` is a literal a stage would be set to: a boolean, a
/// number, a string, `null`.
fn constant(token: &str) -> bool {
    matches!(token, "true" | "false" | "null")
        || token.starts_with('"')
        || token.starts_with('\'')
        || token.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// The lines in `body` that give harness state a constant or step it: the
/// target a bare name or `<id>.<name>` with `<id>` one of `ids`, and not a
/// local the body declares. A write to a product's property (`item.x`,
/// `graphPane.y`) is an input, and stands.
fn stage_writes(body: &[Token], ids: &[String]) -> Vec<usize> {
    let locals: Vec<&str> = body
        .windows(2)
        .filter(|w| matches!(w[0].text.as_str(), "let" | "const" | "var"))
        .map(|w| w[1].text.as_str())
        .collect();
    // The target ending at `end` (inclusive): `name` or `id . name`, owned by the harness.
    let harness_target = |end: usize| -> bool {
        let name = &body[end].text;
        if !name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        {
            return false;
        }
        let dotted = end >= 2 && body[end - 1].text == ".";
        if dotted {
            let head = &body[end - 2].text;
            let deeper = end >= 3 && body[end - 3].text == ".";
            !deeper && ids.iter().any(|id| id == head)
        } else {
            let after_dot = end >= 1 && body[end - 1].text == ".";
            !after_dot
                && !locals.contains(&name.as_str())
                && !matches!(
                    name.as_str(),
                    "property" | "readonly" | "let" | "const" | "var"
                )
        }
    };
    // The value at `start` alone on its statement: a constant, or a negative one.
    let constant_at = |start: usize| -> bool {
        let value = if body.get(start).is_some_and(|t| t.text == "-") {
            start + 1
        } else {
            start
        };
        body.get(value).is_some_and(|t| constant(&t.text))
            && body.get(value + 1).is_none_or(|t| {
                t.line != body[value].line || matches!(t.text.as_str(), ";" | "}" | ")")
            })
    };
    let mut out = Vec::new();
    for (at, token) in body.iter().enumerate() {
        let hit = match token.text.as_str() {
            "=" => at >= 1 && harness_target(at - 1) && constant_at(at + 1),
            // On the line of what it steps: past a line's end it is the next statement's.
            "++" | "--" => {
                let postfix = at >= 1 && body[at - 1].line == token.line && harness_target(at - 1);
                let prefix = body.get(at + 1).is_some_and(|t| t.line == token.line)
                    && path_end(body, at + 1).is_some_and(harness_target);
                postfix || prefix
            }
            "+=" | "-=" => at >= 1 && harness_target(at - 1),
            _ => false,
        };
        if hit && !out.contains(&token.line) {
            out.push(token.line);
        }
    }
    out
}

/// The last token of the `.`-joined path that starts at `start`, if a
/// name stands there.
fn path_end(body: &[Token], start: usize) -> Option<usize> {
    let name = |at: usize| {
        body.get(at).is_some_and(|t| {
            t.text
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
        })
    };
    if !name(start) {
        return None;
    }
    let mut end = start;
    while body.get(end + 1).is_some_and(|t| t.text == ".") && name(end + 2) {
        end += 2;
    }
    Some(end)
}

#[cfg(test)]
mod tests {
    use super::findings;

    fn rules(file: &str, text: &str) -> Vec<(usize, &'static str)> {
        findings(file, text)
            .iter()
            .map(|f| (f.line, f.rule))
            .collect()
    }

    #[test]
    fn a_sampler_of_a_verbs_own_is_named_and_the_machinerys_are_not() {
        let text = "Item {\n    SampleTimer {\n        onTriggered: x()\n    }\n}\n";
        assert_eq!(
            rules("src/auto/AutoActFooVerbs.qml", text),
            [(2, "sampler")]
        );
        assert!(rules("src/auto/WindowHarness.qml", text).is_empty());
        let staged = "Item {\n    PageStage {\n        name: \"x\"\n    }\n}\n";
        assert!(rules("src/auto/AutoActFooVerbs.qml", staged).is_empty());
    }

    #[test]
    fn an_operation_belongs_to_the_side_its_file_is_on() {
        let page = "Item {\n    PageStage {\n        name: \"x\"\n    }\n}\n";
        let window = page.replace("PageStage", "WindowStage");
        assert_eq!(rules("src/auto/WindowTabActs.qml", page), [(2, "owner")]);
        assert_eq!(
            rules("src/auto/AutoActFooVerbs.qml", &window),
            [(2, "owner")]
        );
        let bare = page.replace("PageStage", "StageTimer");
        assert_eq!(rules("src/auto/AutoActFooVerbs.qml", &bare), [(2, "owner")]);
    }

    #[test]
    fn a_verb_reports_and_completes_only_through_its_operation() {
        let text = "Item {\n    function f() {\n        Harness.report(\"x\")\n        window.finishAutoAct()\n        \
                    Awaited.transition(op, \"a\")\n        Awaited.all(\"x\", {})\n        Awaited.claim(\"c\", () => true)\n        \
                    driver.complete()\n        op.complete(() => \"line\")\n    }\n}\n";
        assert_eq!(
            rules("src/auto/AutoActFooVerbs.qml", text),
            [
                (3, "report"),
                (4, "completion"),
                (5, "awaited"),
                (8, "completion")
            ]
        );
        assert!(rules("src/auto/AutoActDriver.qml", text).is_empty());
    }

    /// The stage flags and counters a sampler kept for itself are named;
    /// a reading off the product, a local and an input to the product
    /// stand.
    #[test]
    fn harness_state_given_a_constant_or_stepped_inside_an_operation_is_a_stage() {
        let text = "Item {
    id: acts
    property int step: 0
    PageStage {
        id: op
        name: \"x\"
        property bool pressed: false
        property bool was: false
        onTriggered: {
            let tries = 0
            tries++
            op.was = driver.commandsWrongSeen
            item.pointerRowX = 4
            graphPane.view.contentY = 0
            op.pressed = true
            acts.step++
            step = 2
            ++acts.step
            driver.writeExpected = false
            acts.step += 1
        }
    }
}
";
        assert_eq!(
            rules("src/auto/AutoActFooVerbs.qml", text),
            [
                (15, "stage"),
                (16, "stage"),
                (17, "stage"),
                (18, "stage"),
                (19, "stage"),
                (20, "stage")
            ]
        );
    }

    /// The dispatch sets an operation up before it runs: that is outside
    /// its body, and stands.
    #[test]
    fn the_dispatch_configures_an_operation_and_is_no_stage() {
        let text = "Item {
    id: acts
    function run(act, arg) {
        op.fits = false
        op.start()
    }
    PageStage {
        id: op
        name: \"x\"
        property bool fits: false
    }
}
";
        assert!(rules("src/auto/AutoActFooVerbs.qml", text).is_empty());
    }
}
