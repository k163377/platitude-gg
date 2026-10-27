pragma Singleton
import QtQuick

/// Whether a page has stopped arriving — the one rule every sampler that waits for the whole of a page asks (the
/// census walk, the badge verbs' arm, the page verbs' baseline). A copy that drops a term settles a run early, and
/// that run writes a census line for a window still arriving (rules-refs/app-ui.md「ページ全体を待つ正本は
/// `PageSettled.settled()` 1 本」). A sampler waiting for one thing of its own (a tab count, a refs read) keeps its
/// own wait.
///
/// Every read the page still waits on is a term: stopping at the worktree, the refs and the graph calls a run
/// settled while the changed-file list is on its way, and `FileRowDelegate` flips in and out of the census.
///
/// `finishCount > 0` is not the word: the opening log walk races the first status for the worktree row, and
/// when the status loses it asks for a rebuild that brings the row a pass later. So the rows must agree with the
/// status (`GraphModel.wipRow` against `WorktreeModel.wipRowStands`, both off
/// `platitude_core::graph::wip_row_stands`).
/// `finishCount` is cumulative, and Started applies a stream's generation before its Finished reaches the UI:
/// A healthy stream's `loading` must be down too, even after the core's reads ended and generations already agree.
/// A graph that could not be walked (`failed` / `stale`) owes no new stream or readings; the other page terms
/// (including an initial finish) still stand, and it answers with what it holds.
///
/// A landing the page owes itself is a read on its way (`RepoPage.pageLanding`): both ends of that move settle every
/// other term, so without it a run reads settled before and after the move (`AutoActCompletion.owesStatus`). What
/// this holds is a verb's completion (`complete … page=`, `WindowHarness.finish`), never the moment a picture is
/// taken.
///
/// A move the view is owed is the page still arriving (rules-refs/app-ui.md「`PageSettled` は表示の置き直しも待つ」).
///
/// So are the rows of a list on screen that the view has not built yet (`Awaited.built`): the model landed, the
/// delegates come a polish later, and a run that completes between the two photographs rows its completion never
/// saw. Every list the page shows is asked (`lists`); one hidden, or with nothing in it, owes nothing.
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
        if (!page.pageWorktree.loaded)
            owed.push("worktree")
        if (!page.pageRefsLoaded)
            owed.push("refs")
        if (!(graph.finishCount > 0) || (!graph.failed && !graph.stale && graph.loading))
            owed.push("graph")
        if (!page.pageDetailsSettled)
            owed.push("details")
        if (!page.pageCarriedSettled)
            owed.push("carried")
        if (page.diffShown && page.pageDiff && !page.pageDiff.diffSettled())
            owed.push("diff")
        if (!graph.failed && !graph.stale) {
            if (!graph.readingsSettled())
                owed.push("readings")
            if (page.pageLanding)
                owed.push("landing")
            else if (graph.wipRow !== page.pageWorktree.wipRowStands)
                owed.push("wipRow")
        }
        if (page.pageGraphPane.placing)
            owed.push("placing")
        for (const [name, view] of pageSettled.lists(page))
            if (!Awaited.built(view))
                owed.push("rows:" + name)
        return owed
    }

    /// Whether every list the page shows has built its rows (the `rows:` terms alone) — for a verb that photographs a
    /// page with reads still on their way by design (`Awaited.arriving`), whose picture still has to hold the rows
    /// its completion saw.
    function rowsBuilt(page) {
        if (!page)
            return true
        return pageSettled.lists(page).every(([, view]) => Awaited.built(view))
    }

    /// The window's page settled, or no page yet: the term a window's act waits on before its report, which reads
    /// the page under the window — the completion's hold (`WindowHarness.finish`) comes after the report.
    function windowSettled(window) {
        return !window || !window.curPage || pageSettled.settled(window.curPage)
    }

    /// The lists the page shows, by name, for the rows term: every `ListView` on screen under the page, found by
    /// walking its items, so a pane's new list is asked without being named here. The walk does not go into a list
    /// (its delegates are its rows, not the page's lists) or under an item not shown — one behind a pane not up owes
    /// nothing. A popup is not under the page; its lists are the picture's walk's (`WindowCensus`). A page stood in
    /// for by a test's object is no item and shows no list (`tst_census.qml`).
    function lists(page) {
        const out = []
        if (page instanceof Item)
            pageSettled.collect(page, out)
        return out
    }
    function collect(item, out) {
        const kids = item.children
        for (let i = 0; i < kids.length; i++) {
            const kid = kids[i]
            if (!kid.visible)
                continue
            if (kid instanceof ListView)
                out.push([pageSettled.nameOf(kid, out), kid])
            else
                pageSettled.collect(kid, out)
        }
    }
    /// A list's name in the term: the component holding it and its type (`DiffPane/AppListView`), numbered past the
    /// first of the same name (`NavSections/NavList#2`).
    function nameOf(view, out) {
        let name = pageSettled.typeOf(view) || "ListView"
        for (let holder = view.parent; holder; holder = holder.parent) {
            const type = pageSettled.typeOf(holder)
            if (type !== "") {
                name = type + "/" + name
                break
            }
        }
        let n = 1
        for (const [taken] of out)
            if (taken === name || taken.startsWith(name + "#"))
                n++
        return n === 1 ? name : name + "#" + n
    }
    /// The QML component an object is an instance of, "" for a type QML did not define (a `ColumnLayout`).
    function typeOf(object) {
        const text = String(object)
        const mark = text.indexOf("_QMLTYPE_")
        return mark > 0 ? text.substring(0, mark) : ""
    }
}
