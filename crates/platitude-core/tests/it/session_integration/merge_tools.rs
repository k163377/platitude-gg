//! The merge-tool candidate read, and the promise its answer keeps: the
//! settings pane shows a loading indicator from the accepted ask
//! (`repo_tab::list_merge_tools`) until `MergeToolsLoaded`, so every road
//! out of `ask_merge_tools` has to end in that event — including the two
//! that never search (a session still opening, one whose opening failed).
//!
//! The machine's tools are mocked (`conflict::mock_available_tools`). The
//! mock of a free function is one for the whole binary, so every test that
//! sets it — or reads the real one — holds
//! `#[mry::lock(conflict::available_tools)]`. The real read is in [`periodic`].

use platitude_core::conflict;
use platitude_core::session::{RepoSession, SessionEvent};

use crate::support::session::CaptureSink;
use crate::support::{exec, repo::TestRepo};

/// The first settled answer, whatever it names.
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

/// The opening is over by the time this returns: `settle` says the
/// completion word with nothing awaited after this event, and the runtime
/// is single-threaded, so the task ends before the test runs again.
async fn the_opening_gave_up(sink: &CaptureSink) {
    sink.wait_for("OpenFailed", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::OpenFailed { .. }))
            .then_some(())
    })
    .await;
}

/// A path that is not a repository leaves `workdir()` `None` for good, and
/// nothing to offer is an answer.
#[tokio::test]
async fn a_session_whose_open_failed_answers_the_merge_tool_ask() {
    // Not inside a `TestRepo`: git walks up and would open that repository.
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

/// An ask inside the opening (the settings verb's order) waits the opening
/// out rather than giving up on a working tree about to exist. What config
/// names comes first, and a tool the machine also has is named once.
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

/// The completion word is said once, here with nobody subscribed, so the
/// value it leaves behind is all a later ask can be answered from — a
/// boundary that reaches only waiting readers stalls this ask.
///
/// An order, not a race: the opening is over before the ask is made
/// ([`the_opening_gave_up`]).
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

/// A cancelled opening settles too, with no event beside the word, so the
/// ask has only the boundary to wait for. The second ask reads the kept
/// value, after the opening's only reader (the first ask) has gone.
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
    // Before anything is awaited: cancelled short of a working tree, and
    // silent about it.
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

/// Left out of the pre-merge run: the same ask against this machine's own
/// tools, a read on the order of seconds guarding a screen seldom changed.
mod periodic {
    use super::*;

    /// The settled answer names the configured tool next to whatever this
    /// machine has.
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
        // Inside the opening, as above.
        session.ask_merge_tools();

        // The machine's read says nothing until it ends, and load can
        // stretch it past the silence budget.
        let names = sink
            .wait_through_silence("MergeToolsLoaded settled #1", |evs| nth_settled_in(evs, 1))
            .await;
        assert!(
            names.iter().any(|n| n == "demo-editor"),
            "the configured tool is missing from {names:?}"
        );
    }
}
