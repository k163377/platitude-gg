pragma Singleton
import QtQuick

/// Whether a page has stopped arriving — the one rule asked wherever a sampler waits for the whole of a page: the
/// census walk (`WindowCensus`), the badge verbs' arm (`WindowBadgeActs`) and the page verbs' baseline
/// (`AutoActDriver`). Written once, because a copy that drops a term settles a run early, and a run settled early
/// writes a census line for a window that is still arriving (rules-refs/app-ui.md). A sampler that waits for one
/// thing of its own — a tab count, a refs read — keeps its own wait; this is for "everything".
///
/// **Every read the page is still waiting on is named**, the selected commit's and the graph's second pass included:
/// a rule that stops at the working tree, the refs and the graph calls a run settled while the changed-file list is
/// on its way, and `FileRowDelegate` is then in the census of the run whose list arrived first and out of the next.
///
/// **`finishCount` counts passes, and the last one is the word.** The opening starts the log walk before
/// the first status has been read, so whether that walk carries the working-tree row is a race the status wins about
/// half the time; when it loses, the status asks for a rebuild and the row — with `WipTallyRow` on it — arrives a
/// pass later. What settles it is the rows agreeing with the status: the graph says what it holds (`GraphModel.wipRow`)
/// and the working tree says what should stand (`WorkTree.wipRowStands`), both off the one rule the walk itself is
/// built from (`platitude_core::graph::wip_row_stands`). A graph that could not be walked is waiting for nothing and
/// answers yes with whatever it holds — `graph-stopped` leaves an empty column, and a swap that failed (`stale`) leaves
/// the pass before it standing.
///
/// **A landing the page owes itself is a read still on its way** (`RepoPage.pageLanding`): where a write put the
/// reader is decided at the answer and resolved against the refreshed pair, so in between the page is holding a
/// move it has already made up its mind about. **Both ends of that move settle every other term here** — before it
/// the row the write is about to take away still stands, after it the new commit's changed files are on the right —
/// which is why the term is needed at all (`AutoActCompletion.owesStatus` says what the two pages cost). The
/// photograph is still the verb's own moment; what this holds is the walk, which says so (`WindowCensus.waited`).
///
/// **A move the view is owed is the page still arriving** (`GraphRowWalk.placing`): the landing a page opens on
/// centres its row a beat after it selects it, and the read behind that selection can answer inside the beat — so a
/// page that answered yes there moves its view afterwards, over whatever a run did to it in the meantime.
///
/// **The question is whether rows are still on their way.** A tab still at the picker
/// (`open-picker`) and one whose path was refused (`open-not-a-repo`) are both done: nothing is coming,
/// so a run on either has the whole of what its verb shows. Answering no for those would hold the run open until the
/// watchdog.
QtObject {
    id: pageSettled

    /// `page` is a `RepoPage`; null (a window with no page yet) is done too.
    function settled(page) {
        return pageSettled.missing(page).length === 0
    }

    /// The reads `settled` is still waiting on, by name — empty when it is not waiting on anything. The one rule,
    /// said term by term, so a run that stood here until its ceiling says which read never came (`AutoShotDriver`).
    function missing(page) {
        if (!page)
            return []
        const state = page.pageTab.state
        if (state === "" || state === "error")
            return []
        // An accepted path is only the beginning: the rows arrive with the reads behind it — and until it is open,
        // the parts that carry them are not asked.
        if (state !== "open")
            return ["open"]
        const graph = page.pageGraph
        const owed = []
        if (!page.pageWt.loaded)
            owed.push("worktree")
        if (!page.pageRefsLoaded)
            owed.push("refs")
        if (!(graph.finishCount > 0))
            owed.push("graph")
        if (!page.pageDetailsSettled)
            owed.push("details")
        if (!graph.failed && !graph.stale) {
            if (page.pageLanding)
                owed.push("landing")
            else if (graph.wipRow !== page.pageWt.wipRowStands)
                owed.push("wipRow")
        }
        if (page.pageGraphPane.placing)
            owed.push("placing")
        return owed
    }
}
