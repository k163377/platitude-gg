//! What the holder counts of a session's subagents: their calls, their
//! stops — said or not — and the questions they raise, apart from their
//! session's turn.

use super::{
    FakeClaude, INTERRUPTED, REPLY, TURNED_DOWN, append, asked, calling, claim_file, claim_of,
    ended, holding, launched, long_ago, marked, next_reading, planted, released, result, scratch,
    session, start, stop_hooks_ran, stopped, subagent_transcript, transcript,
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
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
    let agent = || long_ago(claude.agent_hook_with("a1", &own));
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    marked(&dir, &agent(), start("toolu_x", "mcp__docs__search"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent's call an hour old was taken for stopped"
    );

    // The notice starts a turn of the session's, which stops before the
    // holder reads.
    append(&path, &[&stopped("a1", crate::awake::now_ms())]);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
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
    marked(&dir, &again(), start("toolu_y", "mcp__docs__search"));
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
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_ag", "Agent"));
    let agent = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    marked(&dir, &agent(), start("toolu_x", "mcp__docs__search"));
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
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

/// A subagent that has written neither a transcript nor a `.meta.json` of
/// its own by its first call is none the Agent tool started — those write
/// both before their first call — but one of Claude Code's own, run between
/// turns, that runs no stop of any kind (measured): nothing will ever say
/// it ended. At its call it has stopped, as one stopped from outside has:
/// its claim goes and its call ends — and it stays stopped when an end hook
/// of its call writes its claim again. Before its first call it
/// counts as any subagent starting does, and one with its `.meta.json` and
/// no transcript yet is the Agent tool's.
#[test]
fn a_subagent_that_writes_nothing_of_its_own_stops_at_its_call() {
    let dir = scratch("agent-unwritten");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[REPLY]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
    let unwritten = dir.join("s").join("subagents").join("agent-a9.jsonl");
    let agent = || claude.agent_hook_with("a9", &unwritten);
    apply(&dir, &claim_file("s~a9"), &agent(), Mark::Working).expect("its start");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent starting, before anything of its own is written, ran unheld"
    );
    marked(&dir, &agent(), start("toolu_x", "ToolSearch"));
    assert_eq!(
        next_reading(&dir),
        released(),
        "a subagent that writes nothing of its own held the machine"
    );
    assert!(!dir.join(claim_file("s~a9")).exists(), "its claim stood");
    assert_eq!(session(&dir).calls, [], "its call was not ended");
    // Its call's end hook writes its claim again, working, with no call.
    apply(&dir, &claim_file("s~a9"), &agent(), Mark::Working).expect("its end hook");
    marked(&dir, &agent(), Mark::ToolEnd("toolu_x"));
    assert_eq!(
        next_reading(&dir),
        released(),
        "a stopped subagent that writes nothing held the machine again"
    );
    assert!(
        !dir.join(claim_file("s~a9")).exists(),
        "its claim written again stood"
    );

    let unread = subagent_transcript(&dir, "a1", "toolu_ag");
    let agent = || claude.agent_hook_with("a1", &unread);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("the Agent tool's");
    marked(&dir, &agent(), start("toolu_y", "mcp__docs__search"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent with its .meta.json and no transcript yet was taken for one that writes nothing"
    );
}

/// A session that keeps no transcript keeps none for its subagents either:
/// a subagent of its writes nothing of its own and is the Agent tool's all
/// the same, so its call runs until its end.
#[test]
fn a_subagent_of_a_session_that_keeps_no_transcript_runs_its_call() {
    let dir = scratch("agent-unkept");
    let claude = FakeClaude::start(&dir);
    let unkept = dir.join("s.jsonl");
    marked(&dir, &claude.hook_with(&unkept), Mark::Prompt);
    marked(&dir, &claude.hook_with(&unkept), Mark::Idle);
    let agent = || {
        claude.agent_hook_with(
            "a1",
            &dir.join("s").join("subagents").join("agent-a1.jsonl"),
        )
    };
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    marked(&dir, &agent(), start("toolu_x", "mcp__docs__search"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent of a session that keeps no transcript was taken for one that writes nothing"
    );
}

/// Compatible builds use the same activity protocol for subagents too.
/// An idle subagent has no work for its parent's open Agent call to add,
/// including when its hook has not supplied a transcript path.
#[test]
fn an_idle_subagent_of_a_compatible_build_takes_its_call_off_this_holder() {
    let dir = scratch("agent-elsewhere");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Agent", "toolu_ag")]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_ag", "Agent"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent starting, before any claim of it, ran unheld"
    );

    let elsewhere = dir.join(claim_file("s~a1"));
    planted(&dir, &elsewhere, &claim_of(State::Idle, claude.pid()));
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
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_q", "AskUserQuestion"));
    marked(&dir, &claude.hook(), asked("toolu_q"));
    let agent = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("a subagent");
    marked(&dir, &agent(), start("toolu_b", "Bash"));
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
    marked(&dir, &later, Mark::CallsEnd);
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
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_ag", "Agent"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a subagent starting, before its first hook, ran unheld"
    );
    let agent = || claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("its claim");
    marked(&dir, &agent(), start("toolu_b", "Bash"));
    assert_eq!(next_reading(&dir), holding());

    let prompt = Mark::Asked {
        id: None,
        name: "Bash",
        input: "i",
    };
    marked(&dir, &agent(), prompt);
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
    marked(&dir, &claude.hook(), Mark::Idle);
    let agent = claude.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent, Mark::Working).expect("a subagent");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "an interruption in a subagent's transcript let go of its turn"
    );
}
