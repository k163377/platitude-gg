//! What the holder counts as work, against a stand-in Claude process: a
//! turn's calls and the processes of its shell calls, the questions a
//! person is asked, subagents, notices, interruptions, wake-ups, and how
//! long a claim with no word counts.

use super::{
    FakeClaude, REPLY, alive, append, claim_file, claim_of, ended, holding, marked, next_reading,
    notice, planted, released, scratch, session, stamped, stop_hooks_ran, summary, told,
    transcript,
};
use crate::awake::claim::{State, Writer, digest};
use crate::awake::{Mark, apply};

/// A call's start, with its input.
fn start<'a>(id: &'a str, name: &'a str) -> Mark<'a> {
    Mark::ToolStart {
        id,
        name,
        input: "i",
    }
}

/// A person asked about the call `id`, as a question of its own is.
fn asked(id: &str) -> Mark<'_> {
    Mark::Asked {
        id: Some(id),
        name: "",
        input: "",
    }
}

/// A reply that calls `name` as `id`.
fn calling(name: &str, id: &str) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"{id}","name":"{name}","input":{{}}}}]}}}}"#
    )
}

/// The result of the call `id`, written now.
fn result(id: &str) -> String {
    result_at(id, crate::awake::now_ms())
}

/// The result of the call `id`, written at `ms` since the epoch.
fn result_at(id: &str, ms: u64) -> String {
    format!(
        r#"{{"type":"user","message":{{"role":"user","content":[{{"tool_use_id":"{id}","type":"tool_result","content":"done"}}]}},"timestamp":"{}"}}"#,
        stamped(ms)
    )
}

/// The acknowledgement a background subagent's start `id` gets as its
/// result, written at `ms`.
fn launched(id: &str, agent: &str, ms: u64) -> String {
    format!(
        r#"{{"type":"user","message":{{"role":"user","content":[{{"tool_use_id":"{id}","type":"tool_result","content":"Async agent launched"}}]}},"timestamp":"{}","toolUseResult":{{"isAsync":true,"status":"async_launched","agentId":"{agent}"}}}}"#,
        stamped(ms)
    )
}

/// The notice of the background subagent `agent`'s stop, written at `ms`.
fn stopped(agent: &str, ms: u64) -> String {
    format!(
        r#"{{"type":"user","message":{{"role":"user","content":"<task-notification>\n<task-id>{agent}</task-id>\n<status>killed</status>\n</task-notification>"}},"timestamp":"{}","origin":{{"kind":"task-notification","producer":"session-task"}}}}"#,
        stamped(ms)
    )
}

/// Where the subagent `agent` of the session whose transcript is
/// `<dir>/s.jsonl` keeps its transcript, beside its `.meta.json`, which
/// names the call it runs under.
fn subagent_transcript(dir: &std::path::Path, agent: &str, under: &str) -> std::path::PathBuf {
    let subagents = dir.join("s").join("subagents");
    std::fs::create_dir_all(&subagents).expect("the subagents' directory");
    std::fs::write(
        subagents.join(format!("agent-{agent}.meta.json")),
        format!(r#"{{"agentType":"general-purpose","toolUseId":"{under}"}}"#),
    )
    .expect("the subagent's meta");
    subagents.join(format!("agent-{agent}.jsonl"))
}

const QUOTED: &str = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_r","type":"tool_result","content":"found: \"content\":[{\"type\":\"text\",\"text\":\"[Request interrupted by user]\"}]"}]}}"#;

const TURNED_DOWN: &str = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_a","type":"tool_result","content":"The user doesn't want to proceed with this tool use.","is_error":true}]}}"#;

const INTERRUPTED: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"[Request interrupted by user for tool use]"}]}}"#;

const BOOKKEEPING: &str = r#"{"type":"queue-operation","operation":"enqueue","content":"next"}"#;

/// A transcript whose path runs past ASCII — a profile named in Japanese —
/// is read as written: the claim's line is UTF-8 whatever the system's code
/// page, so the interruption the transcript ends on lets go.
#[test]
fn a_transcript_whose_path_runs_past_ascii_is_read() {
    let dir = scratch("wide-path");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("会話.jsonl"), &[REPLY, INTERRUPTED]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the interruption behind a path past ASCII held the machine"
    );
}

/// How far behind a hook stamped long ago is: past [`WORKING_TTL`].
fn long_ago(writer: Writer) -> Writer {
    let ttl = u64::try_from(crate::awake::WORKING_TTL.as_millis()).expect("a TTL in ms");
    Writer {
        now_ms: writer.now_ms - ttl - 60_000,
        ..writer
    }
}

/// A working claim with no call open and no word for [`WORKING_TTL`] holds
/// nothing — a reply that never stopped, a Stop that never came.
#[test]
fn a_turn_silent_past_its_ttl_holds_nothing() {
    let dir = scratch("silent");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[REPLY]);
    marked(&dir, &long_ago(claude.hook_with(&path)), Mark::Prompt);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a turn silent past its TTL held the machine"
    );
}

/// Two sessions and a subagent at once hold the machine while any of them
/// works — whoever else waits on a person or has stopped — and let it go
/// once each has stopped or waits: one session's question, the other's
/// command left running in the background, the first's subagent's call.
#[test]
fn sessions_and_a_subagent_hold_while_any_works() {
    let dir = scratch("overlap");
    let asking = FakeClaude::start(&dir);
    let mut stopped_one = FakeClaude::start(&dir);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[
            &calling("AskUserQuestion", "toolu_q"),
            &calling("Agent", "toolu_ag"),
            &launched("toolu_ag", "a1", crate::awake::now_ms()),
        ],
    );
    marked(&dir, &asking.hook_with(&path), Mark::Prompt);
    for (id, name) in [("toolu_q", "AskUserQuestion"), ("toolu_ag", "Agent")] {
        marked(&dir, &asking.hook(), start(id, name));
    }
    marked(&dir, &asking.hook(), asked("toolu_q"));
    let own = transcript(
        &subagent_transcript(&dir, "a1", "toolu_ag"),
        &[&calling("mcp__docs__search", "toolu_x")],
    );
    let agent = || asking.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("the subagent");
    marked(&dir, &agent(), start("toolu_x", "mcp__docs__search"));

    let other = transcript(&dir.join("t.jsonl"), &[&calling("Bash", "toolu_b")]);
    let hook = || stopped_one.hook_with(&other);
    apply(&dir, &claim_file("t"), &hook(), Mark::Prompt).expect("the other session");
    apply(&dir, &claim_file("t"), &hook(), start("toolu_b", "Bash")).expect("its command");
    let command = stopped_one.run();
    let hook = || stopped_one.hook_with(&other);
    apply(&dir, &claim_file("t"), &hook(), Mark::ToolEnd("toolu_b")).expect("to the background");
    apply(&dir, &claim_file("t"), &hook(), Mark::Idle).expect("it stops");
    stop_hooks_ran(&other);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while a command and a subagent ran"
    );

    ended(command);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while the subagent's call ran"
    );

    append(&path, &[&stopped("a1", crate::awake::now_ms())]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the holder held on while one session waited on a person and the other had stopped"
    );
}

/// A shell call whose end only its result shows — its end hook ran under
/// another revision's build — ends at the result's time: a process started
/// after it is none of its work.
#[test]
fn a_shell_call_whose_end_only_its_result_shows_ends_there() {
    let dir = scratch("result-end");
    let mut claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Bash", "toolu_b")]);
    marked(&dir, &long_ago(claude.hook_with(&path)), Mark::Prompt);
    marked(&dir, &long_ago(claude.hook()), start("toolu_b", "Bash"));
    append(&path, &[&result("toolu_b")]);
    let later = claude.run();
    assert_eq!(
        next_reading(&dir),
        released(),
        "a process started after the call's result held the machine"
    );
    assert!(alive(later));
    assert_eq!(
        session(&dir).calls,
        [],
        "the call that ended in the transcript was not forgotten"
    );
}

/// How long the command ran before it went to the background, as the
/// call's end is stamped: long past the call's start, so a process matched
/// to the call's end — within a few seconds of it — would be missed. The
/// holder reads only the stamps, so the end is stamped ahead rather than
/// waited for.
const IN_THE_FOREGROUND_MS: u64 = 12_000;

/// A command that went to the background halfway — long after it started,
/// with nothing saying it was meant for the background — holds a stopped
/// session's machine until it ends; a process the Claude process already
/// had, a server, never does. The call is forgotten once it left nothing.
#[test]
fn a_command_sent_to_the_background_holds_until_it_ends() {
    let dir = scratch("backgrounded");
    let mut claude = FakeClaude::start(&dir);
    let server = claude.run();
    marked(&dir, &claude.hook(), Mark::Prompt);
    marked(&dir, &claude.hook(), Mark::Idle);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a server the session kept held the machine"
    );

    marked(&dir, &claude.hook(), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_long", "Bash"));
    let command = claude.run();
    let later = Writer {
        now_ms: crate::awake::now_ms() + IN_THE_FOREGROUND_MS,
        ..claude.hook()
    };
    apply(&dir, &claim_file("s"), &later, Mark::ToolEnd("toolu_long")).expect("to the background");
    marked(&dir, &later, Mark::Idle);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while the command ran on"
    );

    ended(command);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the ended command held the machine"
    );
    assert!(alive(server), "the server went with the command");
    assert_eq!(
        session(&dir).calls,
        [],
        "the call that left nothing was not forgotten"
    );
    assert!(
        told(&dir).ends_with(&["holding process:s".to_string(), "released -".to_string()]),
        "the history names the command as what the stopped session held for: {:?}",
        told(&dir)
    );
}

/// A permission prompt lets go of the machine, and the command the person
/// lets through takes it back as it starts — with no hook to say so: the
/// holder, still reading, sees the process of the call asked about.
#[test]
fn the_command_a_person_lets_through_holds_the_machine() {
    let dir = scratch("let-through");
    let mut claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Bash", "toolu_a")]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_a", "Bash"));
    let prompt = Mark::Asked {
        id: None,
        name: "Bash",
        input: "i",
    };
    marked(&dir, &claude.hook(), prompt);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a permission prompt held the machine"
    );

    let command = claude.run();
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the command let through ran unheld"
    );

    ended(command);
    marked(&dir, &claude.hook(), Mark::ToolEnd("toolu_a"));
    marked(&dir, &claude.hook(), Mark::Idle);
    stop_hooks_ran(&path);
    assert_eq!(next_reading(&dir), released());
}

/// Calls that run beside a call a person is asked about hold the machine;
/// once none runs, the question alone holds nothing — however the others
/// ended, by a hook or by a result in the transcript — and one call's
/// result answers no other's question. With every call answered, the
/// reply goes on, held.
#[test]
fn a_call_a_person_is_asked_about_holds_nothing_whatever_else_ends() {
    let dir = scratch("beside");
    let claude = FakeClaude::start(&dir);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[
            &calling("Read", "toolu_read"),
            &calling("mcp__docs__search", "toolu_mcp"),
            &calling("WebFetch", "toolu_fetch"),
            &calling("Bash", "toolu_bash"),
        ],
    );
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    for (id, name) in [
        ("toolu_read", "Read"),
        ("toolu_mcp", "mcp__docs__search"),
        ("toolu_fetch", "WebFetch"),
        ("toolu_bash", "Bash"),
    ] {
        marked(&dir, &claude.hook(), start(id, name));
    }
    marked(&dir, &claude.hook(), asked("toolu_read"));
    marked(&dir, &claude.hook(), asked("toolu_bash"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while calls ran beside the questions"
    );

    marked(&dir, &claude.hook(), Mark::ToolEnd("toolu_mcp"));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while a call still ran beside the questions"
    );
    append(&path, &[&result("toolu_fetch")]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "another call's end took the questions for work"
    );

    append(&path, &[&result("toolu_read")]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "one call's result answered another's question"
    );
    append(&path, &[&result("toolu_bash")]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the reply after the answers ran unheld"
    );
}

/// A permission prompt on a call nothing shows running — an MCP tool —
/// holds nothing: no hook says when the person lets it through.
#[test]
fn a_prompt_on_a_call_nothing_shows_holds_nothing() {
    let dir = scratch("unseen");
    let claude = FakeClaude::start(&dir);
    let input = digest(r#"{"query":"awake"}"#);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[&calling("mcp__docs__read", "toolu_m")],
    );
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    let call = Mark::ToolStart {
        id: "toolu_m",
        name: "mcp__docs__read",
        input: &input,
    };
    marked(&dir, &claude.hook(), call);
    assert_eq!(next_reading(&dir), holding(), "a running call ran unheld");

    let prompt = Mark::Asked {
        id: None,
        name: "mcp__docs__read",
        input: &input,
    };
    marked(&dir, &claude.hook(), prompt);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a prompt on a call nothing shows held the machine"
    );
}

/// Requests for input on one server hold up as many of its running calls
/// as they number, whichever — none of them shows running — so a call on
/// that server left over still runs, held. One request's answer frees that
/// request alone and touches no call's permission prompt: once the answered
/// call ends, the calls left all wait on a person, and nothing is held.
#[test]
fn requests_for_input_hold_up_as_many_calls_as_they_number() {
    let dir = scratch("elicited");
    let claude = FakeClaude::start(&dir);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[
            &calling("mcp__docs__write", "toolu_a"),
            &calling("mcp__docs__write", "toolu_b"),
            &calling("mcp__docs__delete", "toolu_c"),
        ],
    );
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    for (id, name) in [
        ("toolu_a", "mcp__docs__write"),
        ("toolu_b", "mcp__docs__write"),
        ("toolu_c", "mcp__docs__delete"),
    ] {
        marked(&dir, &claude.hook(), start(id, name));
    }
    marked(&dir, &claude.hook(), asked("toolu_c"));
    let elicited = |id, pending| Mark::Elicited {
        server: "docs",
        id: Some(id),
        pending,
    };
    marked(&dir, &claude.hook(), elicited("e1", true));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a call left over on the server ran unheld"
    );

    marked(&dir, &claude.hook(), elicited("e2", true));
    assert_eq!(
        next_reading(&dir),
        released(),
        "the requests held the machine with every call waiting on the person"
    );

    marked(&dir, &claude.hook(), elicited("e1", false));
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the call after its answer ran unheld"
    );

    marked(&dir, &claude.hook(), Mark::ToolEnd("toolu_a"));
    assert_eq!(
        next_reading(&dir),
        released(),
        "one request's answer freed the call the other holds up"
    );
    let prompted = session(&dir)
        .calls
        .into_iter()
        .find(|call| call.id == "toolu_c")
        .expect("the call on a prompt");
    assert!(prompted.asked, "an answer took a permission prompt off");
}

/// Turning a tool down at a prompt ends the turn: the call's result is in,
/// and the interruption after it takes nothing back.
#[test]
fn a_tool_turned_down_at_a_prompt_takes_nothing_back() {
    let dir = scratch("turned-down");
    let claude = FakeClaude::start(&dir);
    let path = transcript(
        &dir.join("s.jsonl"),
        &[&calling("mcp__docs__read", "toolu_a")],
    );
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_a", "mcp__docs__read"));
    marked(&dir, &claude.hook(), asked("toolu_a"));
    assert_eq!(next_reading(&dir), released());
    append(&path, &[TURNED_DOWN, INTERRUPTED, BOOKKEEPING]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a tool turned down held the machine"
    );
}

/// A turn the user interrupts runs no Stop: its working claim lets go once
/// the transcript ends on the interruption. The same words quoted inside a
/// tool's result — the last entry while the model thinks over it — do not
/// let go; a background task's notice after the interruption starts a turn
/// of its own, held.
#[test]
fn a_turn_the_user_interrupted_lets_go() {
    let dir = scratch("interrupted");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Grep", "toolu_r"), QUOTED]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder took words quoted in a tool's result for an interruption"
    );

    append(&path, &[INTERRUPTED, BOOKKEEPING]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "an interrupted turn held the machine"
    );

    append(&path, &[&notice(crate::awake::now_ms())]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the turn a notice started after the interruption ran unheld"
    );
}

/// A notice written since the session's last hook — a background task's
/// end — starts a turn no hook may say began: the claim is rewritten
/// working as of the notice, and held — past the reply's first entries,
/// text or a call whose hook has not run yet — until the turn's Stop. One
/// from before the last hook started nothing now.
#[test]
fn a_notice_starts_a_turn_held_until_its_stop() {
    assert_eq!(stamped(951_782_400_123), "2000-02-29T00:00:00.123Z");
    let dir = scratch("notice");
    let claude = FakeClaude::start(&dir);
    let before = crate::awake::now_ms() - 5_000;
    let path = transcript(&dir.join("s.jsonl"), &[&notice(before)]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a notice from before the last hook held the machine"
    );

    let written = crate::awake::now_ms();
    append(
        &path,
        &[&notice(written), REPLY, &calling("Bash", "toolu_n")],
    );
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the turn a notice started ran unheld once the reply began"
    );
    let begun = session(&dir);
    assert_eq!(
        (begun.state, begun.at),
        (State::Working, written),
        "the claim was not rewritten as of the notice"
    );

    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the turn held the machine after its Stop"
    );
}

/// A background task's notice, queued at `ms` and not taken yet.
fn queued(ms: u64) -> String {
    format!(
        r#"{{"type":"queue-operation","operation":"enqueue","timestamp":"{}","content":"<task-notification>\n<task-id>b1</task-id>\n<status>completed</status>\n</task-notification>"}}"#,
        stamped(ms)
    )
}

/// The queue's head taken at `ms` to start a turn.
fn taken(ms: u64) -> String {
    format!(
        r#"{{"type":"queue-operation","operation":"dequeue","timestamp":"{}"}}"#,
        stamped(ms)
    )
}

/// What was queued, taken into the turn running: attached to its next
/// request, after a call's result.
const ATTACHED: &str = r#"{"attachment":{"type":"queued_command","prompt":"<task-notification>\n<task-id>b1</task-id>\n</task-notification>","commandMode":"task-notification"},"type":"attachment"}"#;

/// What was queued, taken off the queue into the turn running.
const REMOVED: &str = r#"{"type":"queue-operation","operation":"remove"}"#;

/// A turn is not over at its Stop: Claude Code runs every Stop hook first,
/// records that they have run, and only then takes what was queued for the
/// next turn — a notice queued however late. The stopped claim counts until
/// that record is written after its Stop, and a notice's entry takes over
/// from there. One whose record never comes — a prompt a hook turned down,
/// an API error — counts for [`STOPPING`] at most.
#[test]
fn a_stopped_session_counts_until_its_stop_hooks_have_run() {
    let dir = scratch("stopping");
    let claude = FakeClaude::start(&dir);
    let earlier = crate::awake::now_ms() - 5_000;
    let path = transcript(&dir.join("s.jsonl"), &[REPLY, &summary(earlier)]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the turn let go before its Stop hooks had run"
    );
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the turn held the machine once its Stop hooks had run"
    );

    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    assert_eq!(next_reading(&dir), holding());
    let ran = crate::awake::now_ms();
    append(&path, &[&queued(ran), &summary(ran)]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a notice queued during the Stop hooks let go before it was taken"
    );
    append(&path, &[&taken(ran + 1), &notice(ran + 1), REPLY]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the turn a notice queued during the Stop hooks began ran unheld"
    );

    let bound = scratch("stopping-bound");
    let claude = FakeClaude::start(&bound);
    let stopping = u64::try_from(crate::awake::STOPPING.as_millis()).expect("ms");
    let stopped_at = crate::awake::now_ms() - stopping - 5_000;
    let path = transcript(
        &bound.join("s.jsonl"),
        &[REPLY, &summary(stopped_at - 10_000)],
    );
    let stopped = Writer {
        now_ms: stopped_at,
        ..claude.hook_with(&path)
    };
    marked(&bound, &stopped, Mark::Idle);
    assert_eq!(
        next_reading(&bound),
        released(),
        "a Stop whose hooks' end was never recorded held the machine past its bound"
    );
}

/// The holder reads only a transcript's end, and a turn writes past it —
/// a long run of calls, one large result — so the previous turn's record
/// is out of sight at the Stop, as it is in a first turn's transcript. The
/// stopped claim counts until its own record all the same.
#[test]
fn a_stop_counts_until_its_record_with_none_in_sight() {
    let dir = scratch("stopping-unseen");
    let claude = FakeClaude::start(&dir);
    let earlier = crate::awake::now_ms() - 5_000;
    let large = format!(
        r#"{{"type":"user","message":{{"role":"user","content":[{{"tool_use_id":"toolu_l","type":"tool_result","content":"{}"}}]}}}}"#,
        "x".repeat(1 << 20)
    );
    let path = transcript(
        &dir.join("s.jsonl"),
        &[REPLY, &summary(earlier), &large, REPLY],
    );
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a turn whose previous record was out of sight let go at its Stop"
    );
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a turn whose previous record was out of sight held the machine past its own"
    );

    let first = scratch("stopping-first");
    let claude = FakeClaude::start(&first);
    let path = transcript(&first.join("s.jsonl"), &[REPLY]);
    marked(&first, &claude.hook_with(&path), Mark::Prompt);
    marked(&first, &claude.hook_with(&path), Mark::Idle);
    assert_eq!(
        next_reading(&first),
        holding(),
        "a first turn let go before its Stop hooks had run"
    );
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&first),
        released(),
        "a first turn held the machine once its Stop hooks had run"
    );
}

/// A notice queued while the last reply is written — after the turn's last
/// request, so no request takes it in — waits for the next turn: once the
/// Stop hooks have run, Claude Code writes their record, takes the notice
/// off the queue and writes its entry, and the stopped claim counts through
/// each step until the notice's turn takes over. A reply takes nothing in.
/// One the turn did take in — attached to a request after a call's result,
/// or removed from the queue — waits on nothing past the Stop's record.
#[test]
fn a_notice_queued_during_the_last_reply_holds_until_it_is_taken() {
    let dir = scratch("queued-taken");
    let claude = FakeClaude::start(&dir);
    let earlier = crate::awake::now_ms() - 5_000;
    let path = transcript(&dir.join("s.jsonl"), &[REPLY, &summary(earlier)]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    append(&path, &[&queued(crate::awake::now_ms()), REPLY]);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    assert_eq!(next_reading(&dir), holding());
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a notice queued during the last reply let go once the Stop hooks had run"
    );
    append(&path, &[&taken(crate::awake::now_ms())]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a notice queued during the last reply let go once it was taken off the queue"
    );
    append(&path, &[&notice(crate::awake::now_ms()), REPLY]);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the turn the notice began ran unheld"
    );
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
    assert_eq!(
        next_reading(&dir),
        released(),
        "the notice's turn held the machine after its Stop"
    );

    for (taken, how) in [(ATTACHED, "attached"), (REMOVED, "removed")] {
        marked(&dir, &claude.hook_with(&path), Mark::Prompt);
        append(
            &path,
            &[
                &queued(crate::awake::now_ms()),
                &calling("Bash", "toolu_c"),
                &result("toolu_c"),
                taken,
                REPLY,
            ],
        );
        marked(&dir, &claude.hook_with(&path), Mark::Idle);
        stop_hooks_ran(&path);
        assert_eq!(
            next_reading(&dir),
            released(),
            "a notice the turn took in ({how}) held the machine past the Stop"
        );
    }
}

/// What the holder makes of a turn that wrote `during` and stopped, once
/// its Stop hooks' record and then `after` are written.
fn past_the_record(stem: &str, during: &[&str], after: &[&str]) -> Option<String> {
    let dir = scratch(stem);
    let claude = FakeClaude::start(&dir);
    let earlier = crate::awake::now_ms() - 5_000;
    let path = transcript(&dir.join("s.jsonl"), &[REPLY, &summary(earlier)]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    append(&path, during);
    marked(&dir, &claude.hook_with(&path), Mark::Idle);
    stop_hooks_ran(&path);
    append(&path, after);
    next_reading(&dir)
}

/// A call's result takes nothing off the queue: a notice queued before it
/// and attached to no request waits past the Stop's record.
#[test]
fn a_call_s_result_takes_nothing_queued() {
    let at = crate::awake::now_ms();
    assert_eq!(
        past_the_record(
            "queued-result",
            &[
                &queued(at),
                &calling("Bash", "toolu_q"),
                &result("toolu_q"),
                REPLY
            ],
            &[],
        ),
        holding(),
        "a call's result took the notice queued before it"
    );
}

/// A notice queued early in a long last reply waits on the Stop as long as
/// one queued at its end: the bound runs from the Stop.
#[test]
fn what_was_queued_long_before_the_stop_waits_on_it() {
    let stopping = u64::try_from(crate::awake::STOPPING.as_millis()).expect("ms");
    let long_ago = crate::awake::now_ms() - stopping - 5_000;
    assert_eq!(
        past_the_record("queued-early", &[&queued(long_ago), REPLY], &[]),
        holding(),
        "a notice queued past the bound before the Stop let go at its record"
    );
}

/// A `dequeue` starts a turn whether or not what it takes is in sight —
/// one of several queued, the others taken before it: the stopped claim
/// counts from it until the turn's entry.
#[test]
fn a_dequeue_with_nothing_seen_queued_counts_until_its_entry() {
    let at = crate::awake::now_ms();
    assert_eq!(
        past_the_record(
            "queued-several",
            &[&queued(at), &queued(at), &taken(at), &notice(at), REPLY],
            &[&taken(crate::awake::now_ms())],
        ),
        holding(),
        "the second of two notices let go once it was taken off the queue"
    );
}

/// A notice written while one of the session's calls is still open — on a
/// permission prompt, here — is no turn of its own: the wait on the person
/// goes on, unheld.
#[test]
fn a_notice_during_an_open_call_is_no_turn_of_its_own() {
    let dir = scratch("notice-mid-turn");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Bash", "toolu_a")]);
    marked(&dir, &claude.hook_with(&path), Mark::Prompt);
    marked(&dir, &claude.hook(), start("toolu_a", "Bash"));
    marked(&dir, &claude.hook(), asked("toolu_a"));
    assert_eq!(next_reading(&dir), released());

    append(&path, &[&notice(crate::awake::now_ms())]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a notice during the wait held the machine"
    );
}

/// A process the Claude process starts during a call that starts none — a
/// stdio MCP server started again during an MCP call — is none of that
/// call's work.
#[test]
fn a_process_started_during_another_tool_s_call_is_no_work() {
    let dir = scratch("resident");
    let mut claude = FakeClaude::start(&dir);
    marked(&dir, &claude.hook(), Mark::Prompt);
    let call = start("toolu_m", "mcp__docs__read");
    marked(&dir, &claude.hook(), call);
    let server = claude.run();
    marked(&dir, &claude.hook(), Mark::ToolEnd("toolu_m"));
    marked(&dir, &claude.hook(), Mark::Idle);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a server started during an MCP call held the machine"
    );
    assert!(alive(server));
}

/// A session that scheduled a wake-up and stopped holds the machine until
/// the wake-up is due — it cannot fire on a sleeping machine — and lets go
/// once the wake-up is called off.
#[test]
fn a_scheduled_wake_holds_a_stopped_session() {
    let dir = scratch("wake");
    let claude = FakeClaude::start(&dir);
    marked(&dir, &claude.hook(), Mark::WakeIn(600));
    marked(&dir, &claude.hook(), Mark::Idle);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go before the wake-up was due"
    );

    marked(&dir, &claude.hook(), Mark::NoWake);
    marked(&dir, &claude.hook(), Mark::Idle);
    assert_eq!(next_reading(&dir), released());
}

mod subagents;
