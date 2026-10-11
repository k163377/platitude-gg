//! The chip guard's git side and its ledger: what a seat already has open
//! is read from the repository, and a chip over it is refused; a chip
//! holds its number until the session withdraws it.

use std::path::Path;

use crate::support::Sandbox;

/// The session every payload here comes from.
const SESSION: &str = "gate-chips";

/// How the app answers a proposal that went through, JSON-escaped.
const PROPOSED: &str = ",\"tool_response\":[{\"type\":\"text\",\"text\":\"Proposed \\\"1. 直す\\\" \
     (task_id: task_a18c448e). The user sees a card and nothing runs until they start it.\"}]";

fn forward(dir: &Path) -> String {
    dir.display().to_string().replace('\\', "/")
}

/// The start_session call `event` reports, proposing `title` with
/// `prompt` from `dir`; `extra` is the payload's tail after the input.
fn proposal(
    dir: &Path,
    event: &str,
    initiation: &str,
    title: &str,
    prompt: &str,
    extra: &str,
) -> String {
    format!(
        "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"hook_event_name\":\"{event}\",\
         \"tool_name\":\"mcp__ccd_session__start_session\",\
         \"tool_input\":{{\"initiation\":\"{initiation}\",\"title\":\"{title}\",\
         \"context\":\"fresh\",\"prompt\":\"{prompt}\"}}{extra}}}",
        forward(dir)
    )
}

/// What the guard says to a chip whose prompt names `path`, asked from
/// `dir`; empty is no objection.
fn pre_chip(sb: &Sandbox, dir: &Path, path: &str) -> String {
    sb.hook(
        "pre-tool",
        &proposal(
            dir,
            "PreToolUse",
            "own_initiative",
            "1. 直す",
            &format!("{path} を直す"),
            "",
        ),
    )
}

#[test]
fn a_chip_is_refused_over_the_work_the_seat_is_already_holding() {
    let sb = Sandbox::new("chips");
    let core = "crates/platitude-core/src/refs.rs";

    assert_eq!(pre_chip(&sb, &sb.seat, core), "");

    // An unstaged edit: the first ` M` line is where a reading that counts
    // columns goes wrong — git's answer arrives trimmed.
    sb.write_refs(&sb.seat, 1);
    let refused = pre_chip(&sb, &sb.seat, core);
    assert!(refused.contains("already has open"), "{refused}");

    // Committed: the branch carries it until it lands, so the chip's
    // session would still start from a main without it.
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    let carried = pre_chip(&sb, &sb.seat, core);
    assert!(carried.contains("already has open"), "{carried}");

    // The primary checkout, on main, holds nothing.
    assert_eq!(
        pre_chip(&sb, &sb.seat, "crates/platitude-core/src/stash.rs"),
        ""
    );
    assert_eq!(pre_chip(&sb, &sb.repo, core), "");
}

/// A proposal that went through holds its number until the session
/// withdraws it; one that failed is no chip, and neither is an offer or a
/// session the user asked for.
#[test]
fn a_chip_holds_its_number_until_it_is_withdrawn() {
    let sb = Sandbox::new("chips-ledger");
    let (fix, more) = ("internal-docs/P3.md を直す", "internal-docs/P5.md を足す");
    let second = proposal(
        &sb.repo,
        "PreToolUse",
        "own_initiative",
        "1. 別の直し",
        more,
        "",
    );
    let first = |event: &str, extra: &str| {
        proposal(&sb.repo, event, "own_initiative", "1. 直す", fix, extra)
    };
    assert_eq!(sb.hook("pre-tool", &first("PreToolUse", "")), "");
    assert_eq!(
        sb.hook("post-tool", &first("PostToolUseFailure", PROPOSED)),
        ""
    );
    assert_eq!(
        sb.hook("pre-tool", &second),
        "",
        "a failed proposal is no chip"
    );

    assert_eq!(sb.hook("post-tool", &first("PostToolUse", PROPOSED)), "");
    let taken = sb.hook("pre-tool", &second);
    assert!(taken.contains("already the live chip '1. 直す'"), "{taken}");
    for initiation in ["offer", "user_asked"] {
        let not_a_chip = proposal(&sb.repo, "PreToolUse", initiation, "1. 別の直し", more, "");
        assert_eq!(sb.hook("pre-tool", &not_a_chip), "", "{initiation}");
    }

    let withdrawn = format!(
        "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"hook_event_name\":\"PostToolUse\",\
         \"tool_name\":\"mcp__ccd_session__dismiss_task\",\
         \"tool_input\":{{\"task_id\":\"task_a18c448e\"}},\
         \"tool_response\":[{{\"type\":\"text\",\"text\":\"Task task_a18c448e withdrawn.\"}}]}}",
        forward(&sb.repo)
    );
    assert_eq!(sb.hook("post-tool", &withdrawn), "");
    assert_eq!(sb.hook("pre-tool", &second), "");
}
