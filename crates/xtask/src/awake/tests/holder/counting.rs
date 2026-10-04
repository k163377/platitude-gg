//! What the holder counts as work, against a stand-in Claude process: a
//! turn's calls and the processes of its shell calls, the questions a
//! person is asked, subagents, notices, interruptions, wake-ups, and how
//! long a claim with no word counts.

use super::{
    FakeClaude, REPLY, alive, append, claim_file, ended, holding, next_reading, notice, planted,
    released, scratch, session, stamped, told, transcript,
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
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
    apply(
        &dir,
        &claim_file("s"),
        &long_ago(claude.hook_with(&path)),
        Mark::Prompt,
    )
    .expect("claimed");
    assert_eq!(
        next_reading(&dir),
        released(),
        "a turn silent past its TTL held the machine"
    );
}

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
    apply(
        &dir,
        &claim_file("s"),
        &asking.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    for (id, name) in [("toolu_q", "AskUserQuestion"), ("toolu_ag", "Agent")] {
        apply(&dir, &claim_file("s"), &asking.hook(), start(id, name)).expect("a call");
    }
    apply(&dir, &claim_file("s"), &asking.hook(), asked("toolu_q")).expect("the question");
    let own = transcript(
        &subagent_transcript(&dir, "a1", "toolu_ag"),
        &[&calling("mcp__docs__search", "toolu_x")],
    );
    let agent = || asking.agent_hook_with("a1", &own);
    apply(&dir, &claim_file("s~a1"), &agent(), Mark::Working).expect("the subagent");
    apply(
        &dir,
        &claim_file("s"),
        &agent(),
        start("toolu_x", "mcp__docs__search"),
    )
    .expect("its call");

    let other = transcript(&dir.join("t.jsonl"), &[&calling("Bash", "toolu_b")]);
    let hook = || stopped_one.hook_with(&other);
    apply(&dir, &claim_file("t"), &hook(), Mark::Prompt).expect("the other session");
    apply(&dir, &claim_file("t"), &hook(), start("toolu_b", "Bash")).expect("its command");
    let command = stopped_one.run();
    let hook = || stopped_one.hook_with(&other);
    apply(&dir, &claim_file("t"), &hook(), Mark::ToolEnd("toolu_b")).expect("to the background");
    apply(&dir, &claim_file("t"), &hook(), Mark::Idle).expect("it stops");
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

/// A shell call whose end only its result shows — its end hook ran under
/// another revision's build — ends at the result's time: a process started
/// after it is none of its work.
#[test]
fn a_shell_call_whose_end_only_its_result_shows_ends_there() {
    let dir = scratch("result-end");
    let mut claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[&calling("Bash", "toolu_b")]);
    apply(
        &dir,
        &claim_file("s"),
        &long_ago(claude.hook_with(&path)),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(
        &dir,
        &claim_file("s"),
        &long_ago(claude.hook()),
        start("toolu_b", "Bash"),
    )
    .expect("its call");
    append(&path, &[&result("toolu_b")]);
    let later = claude.run();
    assert_eq!(
        next_reading(&dir),
        released(),
        "a process started after the call's result held the machine"
    );
    assert!(super::alive(later));
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
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("claimed");
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("stopped");
    assert_eq!(
        next_reading(&dir),
        released(),
        "a server the session kept held the machine"
    );

    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("a prompt");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        start("toolu_long", "Bash"),
    )
    .expect("a command");
    let command = claude.run();
    let later = Writer {
        now_ms: crate::awake::now_ms() + IN_THE_FOREGROUND_MS,
        ..claude.hook()
    };
    apply(&dir, &claim_file("s"), &later, Mark::ToolEnd("toolu_long")).expect("to the background");
    apply(&dir, &claim_file("s"), &later, Mark::Idle).expect("the turn stops");
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
        start("toolu_a", "Bash"),
    )
    .expect("the call");
    let prompt = Mark::Asked {
        id: None,
        name: "Bash",
        input: "i",
    };
    apply(&dir, &claim_file("s"), &claude.hook(), prompt).expect("asked");
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolEnd("toolu_a"),
    )
    .expect("its end");
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("stopped");
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    for (id, name) in [
        ("toolu_read", "Read"),
        ("toolu_mcp", "mcp__docs__search"),
        ("toolu_fetch", "WebFetch"),
        ("toolu_bash", "Bash"),
    ] {
        apply(&dir, &claim_file("s"), &claude.hook(), start(id, name)).expect("a call");
    }
    apply(&dir, &claim_file("s"), &claude.hook(), asked("toolu_read")).expect("asked");
    apply(&dir, &claim_file("s"), &claude.hook(), asked("toolu_bash")).expect("asked");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go while calls ran beside the questions"
    );

    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolEnd("toolu_mcp"),
    )
    .expect("its end");
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    let call = Mark::ToolStart {
        id: "toolu_m",
        name: "mcp__docs__read",
        input: &input,
    };
    apply(&dir, &claim_file("s"), &claude.hook(), call).expect("the call");
    assert_eq!(next_reading(&dir), holding(), "a running call ran unheld");

    let prompt = Mark::Asked {
        id: None,
        name: "mcp__docs__read",
        input: &input,
    };
    apply(&dir, &claim_file("s"), &claude.hook(), prompt).expect("asked");
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    for (id, name) in [
        ("toolu_a", "mcp__docs__write"),
        ("toolu_b", "mcp__docs__write"),
        ("toolu_c", "mcp__docs__delete"),
    ] {
        apply(&dir, &claim_file("s"), &claude.hook(), start(id, name)).expect("a call");
    }
    apply(&dir, &claim_file("s"), &claude.hook(), asked("toolu_c")).expect("a prompt");
    let elicited = |id, pending| Mark::Elicited {
        server: "docs",
        id: Some(id),
        pending,
    };
    apply(&dir, &claim_file("s"), &claude.hook(), elicited("e1", true)).expect("a request");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "a call left over on the server ran unheld"
    );

    apply(&dir, &claim_file("s"), &claude.hook(), elicited("e2", true)).expect("a request");
    assert_eq!(
        next_reading(&dir),
        released(),
        "the requests held the machine with every call waiting on the person"
    );

    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        elicited("e1", false),
    )
    .expect("an answer");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the call after its answer ran unheld"
    );

    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolEnd("toolu_a"),
    )
    .expect("its end");
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
        start("toolu_a", "mcp__docs__read"),
    )
    .expect("a call");
    apply(&dir, &claim_file("s"), &claude.hook(), asked("toolu_a")).expect("asked");
    assert_eq!(next_reading(&dir), released());
    append(&path, &[TURNED_DOWN, INTERRUPTED, BOOKKEEPING]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a tool turned down held the machine"
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
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
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed");
    apply(&dir, &claim_file("s"), &claude.hook_with(&path), Mark::Idle).expect("stopped");
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

    apply(&dir, &claim_file("s"), &claude.hook_with(&path), Mark::Idle).expect("the turn stops");
    assert_eq!(
        next_reading(&dir),
        released(),
        "the turn held the machine after its Stop"
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
        start("toolu_a", "Bash"),
    )
    .expect("a call");
    apply(&dir, &claim_file("s"), &claude.hook(), asked("toolu_a")).expect("a prompt");
    assert_eq!(next_reading(&dir), released());

    append(&path, &[&notice(crate::awake::now_ms())]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a notice during the wait held the machine"
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

/// A process the Claude process starts during a call that starts none — a
/// stdio MCP server started again during an MCP call — is none of that
/// call's work.
#[test]
fn a_process_started_during_another_tool_s_call_is_no_work() {
    let dir = scratch("resident");
    let mut claude = FakeClaude::start(&dir);
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("claimed");
    let call = start("toolu_m", "mcp__docs__read");
    apply(&dir, &claim_file("s"), &claude.hook(), call).expect("a call");
    let server = claude.run();
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolEnd("toolu_m"),
    )
    .expect("its end");
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("stopped");
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
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::WakeIn(600)).expect("scheduled");
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("stopped");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder let go before the wake-up was due"
    );

    apply(&dir, &claim_file("s"), &claude.hook(), Mark::NoWake).expect("called off");
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("stopped again");
    assert_eq!(next_reading(&dir), released());
}
