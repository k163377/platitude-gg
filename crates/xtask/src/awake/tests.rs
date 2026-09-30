//! The claims and the hooks' marks on them, and — on Windows — the holder
//! itself, run against stand-in Claude processes.

#[cfg(windows)]
use super::agent_transcript;
use super::claim::{Call, Claim, Request, State, Writer, digest};
use super::{
    CLAIM_FORMAT, HOLDER, Mark, REVISION, Target, WAKE_GRACE, Whose, apply, claim_name,
    end_session, file_word, holder_fresh, holder_path, target,
};

/// A test's own directory, taken down as the test ends, pass or fail —
/// after what ran in it: a stand-in Claude process (dropped before this,
/// being made after it) and a holder keep files in it open, so the holder
/// named there is ended first. One that cannot be taken down fails the
/// test.
struct Scratch(std::path::PathBuf);

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A second panic while one unwinds aborts the run: a failed test's
        // holder is ended without a wait that could panic, and what cannot
        // be taken down is said instead.
        let unwinding = std::thread::panicking();
        #[cfg(windows)]
        holder::end_the_holder(&self.0, unwinding);
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            let said = format!("could not take down {}: {error}", self.0.display());
            if unwinding {
                eprintln!("{said}");
            } else {
                panic!("{said}");
            }
        }
    }
}

/// The name of the shared activity claim of `whose` — a session, or
/// `<session>~<agent>` for a subagent.
fn claim_file(whose: &str) -> String {
    format!("r{CLAIM_FORMAT}.{whose}.claim")
}

fn scratch(stem: &str) -> Scratch {
    Scratch(
        crate::verify::claim_dir(&std::env::temp_dir().join("pgg-awake"), stem)
            .expect("a directory of its own"),
    )
}

/// What a hook writes at `at` seconds since the epoch.
fn writer(claude: u32, at: u64) -> Writer {
    Writer {
        claude,
        now_ms: at * 1000,
        seat: "a".into(),
        who: "-".into(),
        transcript: None,
    }
}

fn agent(at: u64) -> Writer {
    Writer {
        who: "a1".into(),
        transcript: Some(r"C:\p\s\subagents\agent-a1.jsonl".into()),
        ..writer(1, at)
    }
}

fn start<'a>(id: &'a str, name: &'a str) -> Mark<'a> {
    Mark::ToolStart {
        id,
        name,
        input: "i",
    }
}

/// Each call's id, whether it ended, and whether a person is asked about
/// it.
fn calls(claim: &Claim) -> Vec<(&str, bool, bool)> {
    claim
        .calls
        .iter()
        .map(|call| (call.id.as_str(), call.end != 0, call.asked))
        .collect()
}

#[test]
fn a_claim_reads_back_as_written_and_a_torn_one_not_at_all() {
    let claim = Claim {
        state: State::Working,
        claude: 52492,
        at: 1_790_000_000,
        seat: "a".into(),
        wake: 1_790_000_900,
        calls: vec![
            Call {
                who: "-".into(),
                id: "toolu_2".into(),
                name: "mcp__docs__read".into(),
                input: digest("{}"),
                start: 1_789_999_000_000,
                end: 0,
                asked: true,
            },
            Call {
                who: "a1".into(),
                id: "toolu_3".into(),
                name: "Bash".into(),
                input: digest(r#"{"command":"ls"}"#),
                start: 1_789_999_100_000,
                end: 1_789_999_200_000,
                asked: false,
            },
        ],
        requests: vec![request("docs", "e1"), request("docs", "-")],
        transcript: r"C:\Users\some one\.claude\projects\p\s.jsonl".into(),
    };
    assert_eq!(Claim::parse(&claim.line()), Some(claim.clone()));
    let bare = Claim {
        calls: Vec::new(),
        requests: Vec::new(),
        ..claim
    };
    assert_eq!(Claim::parse(&bare.line()), Some(bare.clone()));
    assert!(bare.line().starts_with(&format!("r{CLAIM_FORMAT} ")));
    let this = |rest: &str| format!("r{CLAIM_FORMAT} {rest}");
    assert!(Claim::parse(&this("idle 1 1 a 0 - - -")).is_some());
    assert_eq!(
        Claim::parse(&format!("r{} idle 1 1 a 0 - - -", CLAIM_FORMAT + 1)),
        None
    );
    assert_eq!(Claim::parse("idle 1 1 a 0 - - -"), None);
    assert_eq!(Claim::parse(&this("working 1 1 a 0 - -")), None);
    assert_eq!(Claim::parse(&this("waiting 1 1 a 0 - - -")), None);
    assert_eq!(
        Claim::parse(&this("idle 1 1 a 0 -:t1:Bash:i:1:0 - -")),
        None
    );
    assert_eq!(
        Claim::parse(&this("idle 1 1 a 0 -:t1:Bash:i:1:0:2 - -")),
        None
    );
    assert_eq!(Claim::parse(&this("idle 1 1 a 0 - docs -")), None);
}

fn request(server: &str, id: &str) -> Request {
    Request {
        server: server.into(),
        id: id.into(),
    }
}

#[test]
fn an_id_becomes_a_file_name_or_nothing() {
    assert_eq!(file_word("b99484b4-7f4a").as_deref(), Some("b99484b4-7f4a"));
    assert_eq!(file_word("../x").as_deref(), Some("x"));
    assert_eq!(file_word(" / "), None);
    assert_eq!(claim_name("s", None), format!("r{CLAIM_FORMAT}.s.claim"));
    assert_eq!(
        claim_name("s", Some("a1")),
        format!("r{CLAIM_FORMAT}.s~a1.claim")
    );
}

/// The same input digests the same, and another input not.
#[test]
fn an_input_digests_to_the_same_word_every_time() {
    assert_eq!(digest(r#"{"command":"ls"}"#), digest(r#"{"command":"ls"}"#));
    assert_ne!(
        digest(r#"{"command":"ls"}"#),
        digest(r#"{"command":"ls -a"}"#)
    );
    assert_eq!(digest("").len(), 16);
}

/// A subagent's turn lands on its own claim, so neither it nor its session
/// overwrites the other's; its calls land on the session's, since a
/// process outlives the subagent; a session's end takes them all.
#[test]
fn each_mark_lands_on_the_claim_it_belongs_to() {
    let claim = |whose: &str| Some(Target::Claim(claim_file(whose)));
    let agent = Whose::Agent("a1");
    let asked = Mark::Asked {
        id: None,
        name: "Bash",
        input: "i",
    };
    let elicited = Mark::Elicited {
        server: "docs",
        id: Some("e1"),
        pending: true,
    };
    assert_eq!(target("s", Whose::Session, Mark::Prompt), claim("s"));
    assert_eq!(target("s", agent, Mark::Working), claim("s~a1"));
    assert_eq!(target("s", agent, Mark::Gone), claim("s~a1"));
    for on_calls in [
        start("t", "Bash"),
        Mark::ToolEnd("t"),
        asked,
        elicited,
        Mark::CallsEnd,
    ] {
        assert_eq!(target("s", agent, on_calls), claim("s"), "{on_calls:?}");
    }
    assert_eq!(
        target("s", Whose::Session, Mark::Gone),
        Some(Target::Session)
    );
    assert_eq!(target("s", Whose::Agent("/"), Mark::Working), None);
}

/// A scheduled wake-up outlives the stop that follows it — the session
/// sleeps on purpose — and goes only when it is called off.
#[test]
fn a_scheduled_wake_stands_until_it_is_called_off() {
    let hook = writer(1, 100);
    let woke = hook.claim(Mark::WakeIn(600), None).expect("a claim");
    assert_eq!((woke.state, woke.wake), (State::Working, 700 + WAKE_GRACE));
    let stopped = hook.claim(Mark::Idle, Some(&woke)).expect("a claim");
    assert_eq!((stopped.state, stopped.wake), (State::Idle, woke.wake));
    let resumed = hook.claim(Mark::Working, Some(&stopped)).expect("a claim");
    assert_eq!(resumed.wake, woke.wake);
    let off = hook.claim(Mark::NoWake, Some(&resumed)).expect("a claim");
    assert_eq!((off.state, off.wake), (State::Working, 0));
    assert_eq!(hook.claim(Mark::Gone, Some(&off)), None);
}

/// A call's start and end touch the calls alone: a subagent's call leaves
/// its session's turn standing, and the transcript it reads. A call ends by
/// its own id. A stop — the session's or a subagent's — ends the stopping
/// one's own calls that never ended, and no one else's.
#[test]
fn a_call_mark_leaves_the_turn_alone() {
    let stopped = Writer {
        transcript: Some(r"C:\p\s.jsonl".into()),
        ..writer(1, 100)
    }
    .claim(Mark::Idle, None)
    .expect("a claim");
    let started = agent(200)
        .claim(start("toolu_b", "Bash"), Some(&stopped))
        .expect("a claim");
    assert_eq!(
        (started.state, started.at, &started.transcript),
        (stopped.state, stopped.at, &stopped.transcript)
    );
    let again = agent(210)
        .claim(start("toolu_b", "Bash"), Some(&started))
        .expect("a claim");
    assert_eq!(again.calls, started.calls);
    assert_eq!(started.calls[0].start, 200_000);
    let both = writer(1, 250)
        .claim(start("toolu_s", "Read"), Some(&started))
        .expect("a claim");
    let one_ended = writer(1, 260)
        .claim(Mark::ToolEnd("toolu_s"), Some(&both))
        .expect("a claim");
    assert_eq!(
        calls(&one_ended),
        [("toolu_b", false, false), ("toolu_s", true, false)]
    );
    let session_stops = writer(1, 300)
        .claim(start("toolu_t", "Read"), Some(&both))
        .and_then(|claim| writer(1, 400).claim(Mark::Idle, Some(&claim)))
        .expect("a claim");
    assert_eq!(
        calls(&session_stops),
        [
            ("toolu_b", false, false),
            ("toolu_s", true, false),
            ("toolu_t", true, false)
        ]
    );
    let agent_stops = agent(500)
        .claim(Mark::CallsEnd, Some(&both))
        .expect("a claim");
    assert_eq!(
        calls(&agent_stops),
        [("toolu_b", true, false), ("toolu_s", false, false)]
    );
    assert_eq!(
        (agent_stops.state, agent_stops.at, &agent_stops.transcript),
        (both.state, both.at, &both.transcript)
    );
}

/// A person is asked about one call — by its id, or by its tool and input,
/// the latest of the writer's open calls that match, or the one call by
/// that name when no input matches and there is one alone — and another
/// call's end leaves the question standing.
#[test]
fn a_person_is_asked_about_one_call_and_another_s_end_leaves_it() {
    let input = |text: &str| digest(text);
    let (ls, rm) = (input(r#"{"command":"ls"}"#), input(r#"{"command":"rm x"}"#));
    let mut claim = writer(1, 1).claim(Mark::Prompt, None).expect("a claim");
    for (at, id, name, given, hook) in [
        (2, "t_ls", "Bash", &ls, writer(1, 2)),
        (3, "t_rm", "Bash", &rm, writer(1, 3)),
        (4, "t_rm2", "Bash", &rm, writer(1, 4)),
        (5, "t_agent", "Bash", &rm, agent(5)),
        (6, "t_mcp", "mcp__docs__read", &ls, writer(1, 6)),
    ] {
        let mark = Mark::ToolStart {
            id,
            name,
            input: given,
        };
        claim = hook.claim(mark, Some(&claim)).expect("a claim");
        assert_eq!(claim.calls.last().map(|call| call.start), Some(at * 1000));
    }
    let asked_rm = Mark::Asked {
        id: None,
        name: "Bash",
        input: &rm,
    };
    let claim = writer(1, 7).claim(asked_rm, Some(&claim)).expect("a claim");
    let claim = writer(1, 8)
        .claim(
            Mark::Asked {
                id: Some("t_mcp"),
                name: "mcp__docs__read",
                input: "",
            },
            Some(&claim),
        )
        .expect("a claim");
    let unmatched = Mark::Asked {
        id: None,
        name: "Bash",
        input: "nothing like it",
    };
    let claim = writer(1, 9)
        .claim(unmatched, Some(&claim))
        .expect("a claim");
    assert_eq!(
        calls(&claim),
        [
            ("t_ls", false, false),
            ("t_rm", false, false),
            ("t_rm2", false, true),
            ("t_agent", false, false),
            ("t_mcp", false, true)
        ]
    );
    let other_ended = writer(1, 10)
        .claim(Mark::ToolEnd("t_ls"), Some(&claim))
        .expect("a claim");
    assert_eq!(
        calls(&other_ended)[2..],
        [
            ("t_rm2", false, true),
            ("t_agent", false, false),
            ("t_mcp", false, true)
        ]
    );
    let next = writer(1, 11)
        .claim(asked_rm, Some(&other_ended))
        .expect("a claim");
    assert_eq!(calls(&next)[1], ("t_rm", false, true));
    // An input that matches none — two open calls by that name were no
    // answer above; one alone is.
    let lone = writer(1, 11)
        .claim(unmatched, Some(&other_ended))
        .expect("a claim");
    assert_eq!(calls(&lone)[1], ("t_rm", false, true));
}

/// An MCP server's request for input is a record of its own: it changes no
/// call — running, or waiting on a permission prompt — and its answer takes
/// off that request alone, by its id; an answer with none takes a request
/// with none, else the server's oldest. The session's next turn starts with
/// none; a subagent's stop leaves them.
#[test]
fn a_request_for_input_is_a_record_of_its_own() {
    let claim = [
        start("t_a", "mcp__docs__read"),
        start("t_b", "mcp__docs__read"),
        start("t_c", "mcp__docs__delete"),
    ]
    .into_iter()
    .try_fold(
        writer(1, 1).claim(Mark::Prompt, None).expect("a claim"),
        |claim, mark| writer(1, 2).claim(mark, Some(&claim)),
    )
    .and_then(|claim| {
        let prompt = Mark::Asked {
            id: Some("t_c"),
            name: "mcp__docs__delete",
            input: "i",
        };
        writer(1, 3).claim(prompt, Some(&claim))
    })
    .expect("a claim");
    let elicited = |id, pending| Mark::Elicited {
        server: "docs",
        id,
        pending,
    };
    let requested = [
        elicited(Some("e1"), true),
        elicited(Some("e2"), true),
        elicited(Some("e2"), true),
        elicited(None, true),
    ]
    .into_iter()
    .try_fold(claim.clone(), |claim, mark| {
        writer(1, 4).claim(mark, Some(&claim))
    })
    .expect("a claim");
    assert_eq!(
        requested.requests,
        [
            request("docs", "e1"),
            request("docs", "e2"),
            request("docs", "-")
        ]
    );
    assert_eq!(requested.calls, claim.calls);
    let answered = writer(1, 5)
        .claim(elicited(Some("e2"), false), Some(&requested))
        .expect("a claim");
    assert_eq!(
        answered.requests,
        [request("docs", "e1"), request("docs", "-")]
    );
    assert_eq!(answered.calls, claim.calls);
    let unnamed = writer(1, 6)
        .claim(elicited(None, false), Some(&answered))
        .expect("a claim");
    assert_eq!(unnamed.requests, [request("docs", "e1")]);
    let oldest = writer(1, 6)
        .claim(elicited(None, false), Some(&unnamed))
        .expect("a claim");
    assert_eq!(oldest.requests, []);
    let unknown = writer(1, 7)
        .claim(elicited(Some("e9"), false), Some(&unnamed))
        .expect("a claim");
    assert_eq!(unknown.requests, unnamed.requests);

    let agent_stops = agent(8)
        .claim(Mark::CallsEnd, Some(&requested))
        .expect("a claim");
    assert_eq!(agent_stops.requests, requested.requests);
    let next_turn = writer(1, 9)
        .claim(Mark::Prompt, Some(&requested))
        .expect("a claim");
    assert_eq!(next_turn.requests, []);
    let stopped = writer(1, 9)
        .claim(Mark::Idle, Some(&requested))
        .expect("a claim");
    assert_eq!(stopped.requests, []);
}

/// A prompt starts a turn: a call the turn before left open — turned down,
/// interrupted — ends, a person no longer waits on it, and a subagent's
/// calls run on.
#[test]
fn a_prompt_ends_the_calls_the_turn_before_left_open() {
    let claim = writer(1, 1)
        .claim(start("t_old", "Bash"), None)
        .and_then(|claim| {
            let asked = Mark::Asked {
                id: Some("t_old"),
                name: "Bash",
                input: "i",
            };
            writer(1, 2).claim(asked, Some(&claim))
        })
        .and_then(|claim| agent(3).claim(start("t_agent", "Bash"), Some(&claim)))
        .expect("a claim");
    let prompted = writer(1, 4)
        .claim(Mark::Prompt, Some(&claim))
        .expect("a claim");
    assert_eq!(prompted.state, State::Working);
    assert_eq!(
        calls(&prompted),
        [("t_old", true, true), ("t_agent", false, false)]
    );
}

/// A subagent's hooks name its session's transcript; its own sits beside
/// it (measured).
#[cfg(windows)]
#[test]
fn a_subagent_s_transcript_sits_beside_its_session_s() {
    assert_eq!(
        agent_transcript(r"C:\Users\u\.claude\projects\p\b9-4f.jsonl", "a34f56").as_deref(),
        Some(r"C:\Users\u\.claude\projects\p\b9-4f\subagents\agent-a34f56.jsonl")
    );
    assert_eq!(agent_transcript(r"C:\p\s.jsonl", "/"), None);
}

/// A holder beating — a name written now — of this build's revision.
fn beating(dir: &std::path::Path) -> String {
    let named = "1 released".to_string();
    std::fs::write(holder_path(dir), &named).expect("a holder's name");
    named
}

/// Each revision's holder has a name of its own; another revision's
/// beating is not this one's.
#[test]
fn a_holder_s_name_is_its_revision_s() {
    let dir = scratch("holder-revision");
    assert_eq!(
        holder_path(&dir).file_name().and_then(|name| name.to_str()),
        Some(format!("{HOLDER}-r{REVISION}").as_str())
    );
    let other = dir.join(format!("{HOLDER}-r{}", REVISION + 1));
    std::fs::write(other, "1 holding").expect("another revision's holder's name");
    assert!(
        !holder_fresh(&dir),
        "another revision's holder was taken for this one's"
    );
    beating(&dir);
    assert!(holder_fresh(&dir));
}

/// With a holder beating, a claim changes without starting another.
#[test]
fn a_claim_moves_between_states_and_goes() {
    let dir = scratch("claims");
    let named = beating(&dir);
    assert!(holder_fresh(&dir));
    let read = || {
        let line = std::fs::read_to_string(dir.join(claim_file("s"))).ok()?;
        Some(line.strip_prefix(&format!("r{CLAIM_FORMAT} "))?.to_string())
    };
    apply(&dir, &claim_file("s"), &writer(1, 1), Mark::Prompt).expect("claimed");
    assert_eq!(read().as_deref(), Some("working 1 1000 a 0 - - -"));
    apply(&dir, &claim_file("s"), &writer(1, 2), start("t1", "Bash")).expect("a call");
    assert_eq!(
        read().as_deref(),
        Some("working 1 1000 a 0 -:t1:Bash:i:2000:0:0 - -")
    );
    let requested = Mark::Elicited {
        server: "docs",
        id: Some("e 1"),
        pending: true,
    };
    apply(&dir, &claim_file("s"), &writer(1, 3), requested).expect("a request");
    assert_eq!(
        read().as_deref(),
        Some("working 1 1000 a 0 -:t1:Bash:i:2000:0:0 docs:e_1 -")
    );
    apply(&dir, &claim_file("s"), &writer(1, 4), Mark::Idle).expect("idled");
    assert_eq!(
        read().as_deref(),
        Some("idle 1 4000 a 0 -:t1:Bash:i:2000:4000:0 - -")
    );
    apply(&dir, &claim_file("s"), &writer(1, 4), Mark::Gone).expect("gone");
    assert_eq!(read(), None);
    apply(&dir, &claim_file("s"), &writer(1, 5), Mark::Gone).expect("gone twice is gone");
    assert_eq!(std::fs::read_to_string(holder_path(&dir)).ok(), Some(named));
}

/// A session's end takes its subagents' claims and every revision's, and
/// nobody else's — not a session whose id only starts the same.
#[test]
fn a_session_ends_with_its_subagents_and_alone() {
    let dir = scratch("session-end");
    beating(&dir);
    for whose in ["s", "s~a1", "s~a2", "t", "t~a1", "st"] {
        apply(&dir, &claim_file(whose), &writer(1, 1), Mark::Working).expect("claimed");
    }
    let other = |whose: &str| format!("r{}.{whose}.claim", CLAIM_FORMAT + 1);
    for whose in ["s", "s~a3", "t"] {
        std::fs::write(dir.join(other(whose)), "a claim of another revision")
            .expect("another revision's claim");
    }
    end_session(&dir, "s").expect("ended");
    let mut left: Vec<String> = std::fs::read_dir(&*dir)
        .expect("the directory")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".claim"))
        .collect();
    left.sort();
    let mut kept = vec![
        claim_file("st"),
        claim_file("t"),
        claim_file("t~a1"),
        other("t"),
    ];
    kept.sort();
    assert_eq!(left, kept);
}

/// An unknown protocol's record is not silently migrated or overwritten.
#[test]
fn a_claim_of_another_protocol_keeps_its_record() {
    let dir = scratch("revisions-apart");
    beating(&dir);
    let theirs = dir.join(format!("r{}.s.claim", CLAIM_FORMAT + 1));
    let line = format!(
        "r{} working 1 1000 a 0 -:t0:Bash:i:1000:2000:0 - -",
        CLAIM_FORMAT + 1
    );
    std::fs::write(&theirs, &line).expect("another revision's claim");
    apply(&dir, &claim_file("s"), &writer(1, 3), Mark::Prompt).expect("claimed");
    apply(&dir, &claim_file("s"), &writer(1, 4), start("t1", "Bash")).expect("a call");
    apply(&dir, &claim_file("s"), &writer(1, 5), Mark::Idle).expect("stopped");
    assert_eq!(
        std::fs::read_to_string(&theirs).ok(),
        Some(line),
        "this revision's hooks rewrote another's record"
    );
    let ours = std::fs::read_to_string(dir.join(claim_file("s"))).expect("this revision's claim");
    let ours = Claim::parse(&ours).expect("a claim that reads");
    assert_eq!(ours.calls.len(), 1, "{ours:?}");
}

#[test]
fn a_missing_holder_is_not_a_fresh_one() {
    let dir = scratch("holder");
    assert!(!holder_fresh(&dir));
}

#[test]
fn an_unreadable_existing_claim_is_not_replaced() {
    let dir = scratch("unreadable-activity");
    beating(&dir);
    let path = dir.join(claim_file("s"));
    let existing = "an incompatible or incomplete activity record";
    std::fs::write(&path, existing).expect("the existing record");
    assert!(apply(&dir, &claim_file("s"), &writer(1, 3), Mark::Prompt).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), existing);
}

/// What turns a run of this test binary into a link of the hook chain: the
/// directory the claims live in, and the Claude process owning them.
const CHAIN_DIR: &str = "PGG_AWAKE_DIR";
const CHAIN_CLAUDE: &str = "PGG_AWAKE_CLAUDE";

/// Passed as a filter rather than with `--exact`: the module path is not
/// these tests' to know.
const HOOK: &str = "stands_in_for_a_hook_that_claims_working";

/// `cargo run`, which hands the hook its own standard handles by copying
/// them — the child inherits the copies and the originals.
#[test]
#[ignore = "the middle process of a_holder_lets_the_pipes_of_its_hook_close"]
fn stands_in_for_cargo_between_claude_and_a_hook() {
    if std::env::var_os(CHAIN_DIR).is_none() {
        return;
    }
    let status = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args([HOOK, "--ignored", "--nocapture"])
        .status()
        .expect("the hook runs");
    assert!(status.success(), "{status}");
}

#[test]
#[ignore = "the last process of a_holder_lets_the_pipes_of_its_hook_close"]
fn stands_in_for_a_hook_that_claims_working() {
    let (Some(dir), Ok(claude)) = (std::env::var_os(CHAIN_DIR), std::env::var(CHAIN_CLAUDE)) else {
        return;
    };
    let claude = claude.parse().expect("the stand-in's pid");
    apply(
        std::path::Path::new(&dir),
        &claim_file("s"),
        &writer(claude, super::now_secs()),
        Mark::Working,
    )
    .expect("claimed, and the holder started");
}

#[cfg(windows)]
mod holder;
