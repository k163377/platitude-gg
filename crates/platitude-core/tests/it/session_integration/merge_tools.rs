//! The merge-tool candidate read, and the promise its answer keeps.
//!
//! **A screen that asks for these marks itself loading and waits.** The
//! settings pane puts a turning indicator up the instant the ask is
//! accepted (`repo_tab::list_merge_tools`) and takes it down on
//! `MergeToolsLoaded` and nothing else, so every road out of
//! `ask_merge_tools` has to end in that event. The two roads that do not
//! run the search at all — a session still opening, and one whose
//! opening failed — are the ones these hold.
//!
//! **Where the read reaches the machine, the machine is the test's**
//! (`conflict::mock_available_tools`): what is judged is the session's
//! answer. The mock of a free function is one for the whole binary, so
//! every test that sets it — or reads the real one — holds
//! `#[mry::lock(conflict::available_tools)]`. The same ask against this
//! machine's own tools is in [`periodic`].

use platitude_core::conflict;
use platitude_core::session::{RepoSession, SessionEvent};

use crate::support::session::CaptureSink;
use crate::support::{exec, repo::TestRepo};

/// The event, whatever it carries.
async fn settled_names(sink: &CaptureSink) -> Vec<String> {
    nth_settled(sink, 1).await
}

/// The `nth` answer, counted — a test that asks twice cannot read the
/// second answer with a predicate the first one already satisfies.
async fn nth_settled(sink: &CaptureSink, nth: usize) -> Vec<String> {
    sink.wait_for(&format!("MergeToolsLoaded settled #{nth}"), |evs| {
        nth_settled_in(evs, nth)
    })
    .await
}

/// The `nth` settled answer among `evs`, once it has arrived.
fn nth_settled_in(evs: &[SessionEvent], nth: usize) -> Option<Vec<String>> {
    let mut seen = 0;
    for event in evs {
        if let SessionEvent::MergeToolsLoaded {
            names,
            settled: true,
        } = event
        {
            seen += 1;
            if seen == nth {
                return Some(names.clone());
            }
        }
    }
    None
}

/// **The opening is over by the time this returns.** `settle` says the
/// completion word with nothing awaited between this event and it, and
/// this suite's runtime is single-threaded, so the task runs to its end
/// before the test is scheduled again. That is what puts an ask made
/// after this on the far side of the boundary — with, at that moment,
/// nobody subscribed to it.
async fn the_opening_gave_up(sink: &CaptureSink) {
    sink.wait_for("OpenFailed", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::OpenFailed { .. }))
            .then_some(())
    })
    .await;
}

/// **An opening that never produced a working tree still answers.** A
/// path that is not a repository leaves `workdir()` `None` for good, and
/// a read that reads that as "give up" leaves the screen's indicator
/// turning for the life of the window. Nothing to offer is an answer.
#[tokio::test]
async fn a_session_whose_open_failed_answers_the_merge_tool_ask() {
    // A directory with no repository above it either: one made inside a
    // `TestRepo` would be opened as that repository, because git walks
    // up (`support::repo`).
    let plain = tempfile::tempdir().expect("a plain directory");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        exec::isolated(),
        tokio::runtime::Handle::current(),
        plain.path().to_path_buf(),
        sink.clone(),
        None,
    );
    session.ask_merge_tools();

    assert!(
        settled_names(&sink).await.is_empty(),
        "a session with nowhere to look offered something"
    );
}

/// **An ask that arrives inside the opening gets the real answer.** This
/// is the order the settings verb runs in — the screen is up before the
/// repository has finished opening — and the read has to wait the
/// opening out rather than give up on a working tree that is about to
/// exist. What config names comes first, and a tool the machine has as
/// well is named once.
#[tokio::test]
#[mry::lock(conflict::available_tools)]
async fn an_ask_inside_the_opening_names_the_configured_tool_first() {
    conflict::mock_available_tools()
        .returns_with(|| Ok(vec!["meld".to_string(), "demo-editor".to_string()]));
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "first");
    repo.git(&["config", "mergetool.demo-editor.cmd", "true"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
        None,
    );
    // Before anything is awaited: the opening is still out, so this is
    // the ask the screen makes while `workdir()` is `None`.
    session.ask_merge_tools();

    assert_eq!(settled_names(&sink).await, ["demo-editor", "meld"]);
}

/// **An ask made after the opening has already given up is answered
/// too.** The completion word is said once, and on this road it is said
/// with nothing subscribed to hear it — so the value it leaves behind is
/// the only thing a later read can be answered from. A boundary that
/// only reaches the readers already waiting is the same stall, one step
/// further on.
///
/// **This is an order, not a race**: the opening is over before the ask
/// is made ([`the_opening_gave_up`]), and the ask is the only subscriber
/// there has ever been.
#[tokio::test]
async fn an_ask_made_after_the_opening_gave_up_is_answered() {
    let plain = tempfile::tempdir().expect("a plain directory");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        exec::isolated(),
        tokio::runtime::Handle::current(),
        plain.path().to_path_buf(),
        sink.clone(),
        None,
    );
    the_opening_gave_up(&sink).await;
    session.ask_merge_tools();

    assert!(
        settled_names(&sink).await.is_empty(),
        "a session that had already given up left the ask unanswered"
    );
}

/// **A cancelled opening settles as well, and stays settled.** Closing a
/// page before its repository finished opening is the road that says the
/// word with no event beside it, so the ask that follows has nothing to
/// wait for but the boundary itself. The second ask is the part that
/// reads the kept value: by then the opening is long over and its only
/// reader — the first ask — has gone.
#[tokio::test]
async fn a_session_closed_inside_its_opening_answers_every_ask() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "first");
    repo.git(&["config", "mergetool.demo-editor.cmd", "true"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
        None,
    );
    // Before anything is awaited, so the opening is cancelled where it
    // stands: it never reaches a working tree, and it never says why.
    session.close();

    session.ask_merge_tools();
    assert!(
        nth_settled(&sink, 1).await.is_empty(),
        "a closed session offered a tool it can no longer open a file with"
    );

    session.ask_merge_tools();
    assert!(
        nth_settled(&sink, 2).await.is_empty(),
        "the second ask was left waiting on an opening that had already settled"
    );
}

/// **What the pre-merge run leaves out**: the same ask, answered from this
/// machine's own installed tools. That read takes seconds on Windows and
/// guards a screen seldom changed, so the full gate runs it
/// (`-- --ignored ::periodic::`) rather than every change.
mod periodic {
    use super::*;

    /// **The configured tool survives the machine's own sweep** — the
    /// settled answer names it next to whatever this machine has.
    #[tokio::test]
    #[ignore = "this machine's merge tools: seconds on Windows, not worth the pre-merge run"]
    #[mry::lock(conflict::available_tools)]
    async fn an_ask_inside_the_opening_still_names_the_configured_tool() {
        // Held and told to call through, so no pre-merge test's machine can
        // stand in for this one.
        conflict::mock_available_tools().calls_real_impl();
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "a\n", "first");
        repo.git(&["config", "mergetool.demo-editor.cmd", "true"]);

        let sink = CaptureSink::new();
        let session = RepoSession::open(
            exec::isolated(),
            tokio::runtime::Handle::current(),
            repo.path.clone(),
            sink.clone(),
            None,
        );
        // Before anything is awaited: the opening is still out, so this is
        // the ask the screen makes while `workdir()` is `None`.
        session.ask_merge_tools();

        // Through the silence: this machine's read says nothing until it
        // ends, and a busy machine stretches it past the silence budget
        // with nothing wrong.
        let names = sink
            .wait_through_silence("MergeToolsLoaded settled #1", |evs| nth_settled_in(evs, 1))
            .await;
        assert!(
            names.iter().any(|n| n == "demo-editor"),
            "the configured tool is missing from {names:?}"
        );
    }
}
