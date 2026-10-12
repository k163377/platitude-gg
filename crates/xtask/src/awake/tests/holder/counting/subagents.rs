//! What the holder counts of a session's subagents: their calls, their
//! stops — said or not — and the questions they raise, apart from their
//! session's turn.

use super::{
    FakeClaude, INTERRUPTED, TURNED_DOWN, append, asked, calling, claim_file, ended, holding,
    launched, long_ago, next_reading, planted, released, result, scratch, session, start, stopped,
    subagent_transcript, transcript,
};
use crate::awake::claim::State;
use crate::awake::{Mark, apply};

/// A subagent's call runs for as long as it runs — no word from the
/// subagent for an hour is no stop — and a subagent stopped from outside,
/// which runs no SubagentStop, has stopped once its session's transcript
/// says so after its last hook: its claim goes and its call ends. Sent on
/// again later, it claims again with its next hook, held.
#[test]
fn a_subagent_s_call_runs_until_its_stop_is_written() {
    let dir = scratch("agent-stop");
    let claude = FakeClaude::start(&dir);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[
            &calling("Agent", "toolu_ag"),
            &launched("toolu_ag", "a1", crate::awake::now_ms()),
        ],
    );
    let own = transcript(
        &subagent_transcript(&dir, "a1", "toolu_ag"),
        &[&calling("mcp__docs__search", "toolu_x")],
    );
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(&dir, &claim_file("s"), &claude.hook_with(&path), Mark::Idle).expect("the session stops");
    let agent = || long_ago(claude.agent_hook_with("a1", &own));
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    apply(
        &dir,
        &claim_file("s"),
        &agent(),
        start("toolu_x", "mcp__docs__search"),
    )
    .expect("its call");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent's call an hour old was taken for stopped"
    );

    // The notice starts a turn of the session's, which stops before the
    // holder reads.
    append(&path, &[&stopped("a1", crate::awake::now_ms())]);
    apply(&dir, &claim_file("s"), &claude.hook_with(&path), Mark::Idle).expect("the turn stops");
    assert_eq!(
        next_reading(&dir),
        released(),
        "a subagent stopped from outside held the machine"
    );
    assert!(
        !dir.join(claim_file("s~a1")).exists(),
        "the stopped subagent's claim stood"
    );
    assert_eq!(
        session(&dir).calls,
        [],
        "the stopped subagent's call was not ended"
    );

    let again = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &again(), Mark::Working).expect("sent on");
    apply(
        &dir,
        &claim_file("s"),
        &again(),
        start("toolu_y", "mcp__docs__search"),
    )
    .expect("its next call");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent sent on after its stop ran unheld"
    );
}

/// A subagent in the foreground has stopped once the call it runs under
/// has its result — as an interrupted one has, with no SubagentStop — and
/// a background one's start, whose result is only its acknowledgement, has
/// not.
#[test]
fn a_foreground_subagent_stops_with_the_result_of_its_call() {
    let dir = scratch("agent-result");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Agent", "toolu_ag")]);
    let own = transcript(
        &subagent_transcript(&dir, "a1", "toolu_ag"),
        &[&calling("mcp__docs__search", "toolu_x")],
    );
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        start("toolu_ag", "Agent"),
    )
    .expect("its start");
    let agent = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    apply(
        &dir,
        &claim_file("s"),
        &agent(),
        start("toolu_x", "mcp__docs__search"),
    )
    .expect("its call");
    apply(&dir, &claim_file("s"), &claude.hook_with(&path), Mark::Idle).expect("the session stops");
    append(
        &path,
        &[&launched("toolu_ag", "a1", crate::awake::now_ms())],
    );
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a background start's acknowledgement was taken for its stop"
    );

    append(&path, &[&result("toolu_ag")]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a subagent whose call has its result held the machine"
    );
    assert!(!dir.join(claim_file("s~a1")).exists());
}

/// Compatible builds use the same activity protocol for subagents too.
/// An idle subagent has no work for its parent's open Agent call to add,
/// including when its hook has not supplied a transcript path.
#[test]
fn an_idle_subagent_of_a_compatible_build_takes_its_call_off_this_holder() {
    let dir = scratch("agent-elsewhere");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Agent", "toolu_ag")]);
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        start("toolu_ag", "Agent"),
    )
    .expect("its start");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent starting, before any claim of it, ran unheld"
    );

    let elsewhere = dir.join(claim_file("s~a1"));
    planted(
        &dir,
        &elsewhere,
        &format!(
            "r1 idle {} {} a 0 - - -",
            claude.pid(),
            crate::awake::now_ms()
        ),
    );
    assert_eq!(
        next_reading(&dir),
        released(),
        "an idle compatible subagent without a transcript kept its parent held"
    );
}

/// A subagent's turn is its own, and a process it starts is its
/// session's: the process holds the machine after the subagent stopped,
/// while the session's question stands untouched — and once the process
/// ends, the question alone holds nothing.
#[test]
fn a_subagent_s_command_holds_the_machine_apart_from_its_session() {
    let dir = scratch("subagent");
    let mut claude = FakeClaude::start(&dir);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[&calling("AskUserQuestion", "toolu_q")],
    );
    let own = transcript(&dir.join("agent-a1.jsonl"), &[&calling("Bash", "toolu_b")]);
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        start("toolu_q", "AskUserQuestion"),
    )
    .expect("a call");
    apply(&dir, &claim_file("s"), &claude.hook(), asked("toolu_q")).expect("asked");
    let agent = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    apply(&dir, &claim_file("s"), &agent(), start("toolu_b", "Bash")).expect("its command");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while the subagent's call ran"
    );

    let command = claude.run();
    // Each hook stamps its own time: the call runs from its start's to its
    // end's.
    let later = claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s"), &later, Mark::ToolEnd("toolu_b")).expect("to the background");
    apply(&dir, &claim_file("s"), &later, Mark::CallsEnd).expect("the subagent stops");
    apply(&dir, &claim_file("s~a1"), &later, Mark::Gone).expect("its claim goes");
    let standing = session(&dir);
    assert_eq!(
        (standing.state, standing.calls[0].asked),
        (State::Working, true),
        "the subagent's calls touched its session's question"
    );
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while the subagent's command ran"
    );

    ended(command);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the session's question held the machine once the command ended"
    );
}

/// A permission prompt a subagent raised: the subagent's only call waits
/// on the person, and its session's only call is the subagent's own — so
/// nothing runs, and the machine goes; the call's result, in the
/// subagent's own transcript, lets the subagent go on, held.
#[test]
fn a_prompt_a_subagent_raised_holds_nothing_until_its_answer() {
    let dir = scratch("agent-asked");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Agent", "toolu_ag")]);
    let own = transcript(&dir.join("agent-a1.jsonl"), &[&calling("Bash", "toolu_b")]);
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        start("toolu_ag", "Agent"),
    )
    .expect("the subagent");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent starting, before its first hook, ran unheld"
    );
    let agent = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("its claim");
    apply(&dir, &claim_file("s"), &agent(), start("toolu_b", "Bash")).expect("its call");
    assert_eq!(next_reading(&dir), holding());

    let prompt = Mark::Asked {
        id: None,
        name: "Bash",
        input: "i",
    };
    apply(&dir, &claim_file("s"), &agent(), prompt).expect("asked");
    assert_eq!(
        next_reading(&dir),
        released(),
        "the machine was held while the subagent's prompt waited"
    );

    append(&own, &[&result("toolu_b")]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the subagent's reply after the answer ran unheld"
    );
}

/// A subagent's transcript is not read for an interruption: what a
/// subagent does after one is not known, and it may reply on.
#[test]
fn a_subagent_s_transcript_is_not_read_for_an_interruption() {
    let dir = scratch("agent-interrupted");
    let claude = FakeClaude::start(&dir);
    let own = transcript(&dir.join("agent-a1.jsonl"), &[TURNED_DOWN, INTERRUPTED]);
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("the session stopped");
    let agent = claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent, Mark::Working).expect("a subagent");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "an interruption in a subagent's transcript let go of its turn"
    );
}
