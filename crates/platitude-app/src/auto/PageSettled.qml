pragma Singleton
import QtQuick

/// Whether a page has stopped arriving — the one rule every sampler that waits for the whole of a page asks (the
/// census walk, the badge verbs' arm, the page verbs' baseline). A copy that drops a term settles a run early, and
/// that run writes a census line for a window still arriving (rules-refs/app-ui.md「ページ全体を待つ正本は
/// `PageSettled.settled()` 1 本」). A sampler waiting for one thing of its own (a tab count, a refs read) keeps its
/// own wait.
///
/// Every read the page still waits on is a term: stopping at the working tree, the refs and the graph calls a run
/// settled while the changed-file list is on its way, and `FileRowDelegate` flips in and out of the census.
///
/// `finishCount > 0` is not the word: the opening log walk races the first status for the working-tree row, and
/// when the status loses it asks for a rebuild that brings the row a pass later. So the rows must agree with the
/// status (`GraphModel.wipRow` against `WorkTreeModel.wipRowStands`, both off `platitude_core::graph::wip_row_stands`).
/// A graph that could not be walked (`failed` / `stale`) waits for nothing and answers yes with what it holds.
///
/// A landing the page owes itself is a read on its way (`RepoPage.pageLanding`): both ends of that move settle every
/// other term, so without it a run reads settled before and after the move (`AutoActCompletion.owesStatus`). What
/// this holds is the census walk, not the verb's photograph (`WindowCensus.waited`).
///
/// A move the view is owed is the page still arriving (rules-refs/app-ui.md「`PageSettled` は表示の置き直しも待つ」).
///
/// A tab still at the picker, or one whose path was refused, is done: nothing is coming, and answering no would hold
/// the run until the watchdog.
QtObject {
    id: pageSettled

    /// `page` is a `RepoPage`; null (a window with no page yet) is done too.
    function settled(page) {
        return pageSettled.missing(page).length === 0
    }

    /// The reads `settled` still waits on, by name, so a run that stood here until its ceiling says which read never
    /// came (`AutoShotDriver`).
    function missing(page) {
        if (!page)
            return []
        const state = page.pageTab.state
        if (state === "" || state === "error")
            return []
        // Until it is open, the parts that carry the rows are not asked.
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
