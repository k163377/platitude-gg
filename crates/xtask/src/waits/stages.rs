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
//!
//! What an operation's code does — the state it keeps, the inputs it puts
//! in, wherever the code stands — is the reach rule's (`super::reach`).

use super::Finding;
use crate::lex::{Lang, tokens};

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
pub(super) const MACHINERY: [&str; 7] = [
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
                if next(1) == Some(".")
                    && next(2) == Some("complete")
                    && next(3) == Some("(")
                    && !COMPLETERS.contains(&name) =>
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
    out
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
        assert!(rules("src/auto/CompletionRoute.qml", text).is_empty());
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
}
