pragma ComponentBehavior: Bound

import QtQuick
import platitude

/// Every QML component the window is showing as a verb's run finishes, by type name.
///
/// The gate (`cargo xtask gate`) owes a change the verbs that show what it touched, and this is how a verb says what it
/// shows: the item tree is walked once, on the picture the run saved, and every type met is reported as
/// `census=A,B,C` for verify-ui to record against the line that ran (`crates/xtask/verb-census.txt`).
///
/// **One walk, on a window that has stopped arriving — never on a clock.** A sampled walk records where its ticks
/// landed rather than what the run did: a row a run raises and takes away again is in the census of the machine whose
/// tick caught it and out of the census of the machine whose tick came late, so the file the gate reads moves under
/// work that never touched the application (measured: `avatar-remove` names `AvatarAssignRow` for its ticks and
/// nothing else).
///
/// **The verb's own edge is not that point.** A verb is finished when what it acted on answers, and the repository is
/// still being read behind it: the opening walks the log before the first status has said whether anything is
/// uncommitted, so the working-tree row — and the tallies on it — arrive with the pass that status asks for. A walk
/// taken at the picture is on the near side of that read about as often as the far side, and the two answers differ
/// by a name, so the checked-in file moved under every gate that ran (measured 2026-09-05: `settings-eol true` lost
/// `WipTallyRow` in 4 runs of 10 taken one after another and in 9 of 10 taken at once, and `file-menu-conflict
/// both.txt --preset conflict-kinds` in 10 of 10 taken at once — each time writing its line, because each run
/// answered that its page had settled).
///
/// So the walk waits for `settled`, and the run's ending waits for the walk (`AutoShotDriver.waitsForCensus`). A verb
/// that photographs a state on its way somewhere still photographs it — the picture is taken at the same edge as
/// before — but what it is recorded as showing is the whole of what it brought up, which is the same on every
/// machine. A window that never settles is the watchdog's to report; there is no clock here that would let one pass.
///
/// A QML-defined type answers `String(item)` with `<File>_QMLTYPE_<n>(0x…)`, so the file's name is the part before the
/// mark; an inline component answers with its bare name and a C++ type with its class, and neither names a file, so
/// the runner drops them. Popups stand under the window's overlay, which is a child of the root item, so one walk from
/// the root sees them too. A base a component derives from is not met — the object carries the derived name — which
/// is why the gate follows the type-reference graph as well.
Item {
    id: census

    /// The window whose tree is walked. `var` because `Main` is the file the engine loads, not a type anything names.
    required property var window

    /// The names met, as an object used for its keys.
    property var seen: ({})

    /// One walk: the window itself (a Window is no item, so the root item's tree never names `Main`), then from the
    /// root item down.
    function walk() {
        if (!census.window || !census.window.contentItem)
            return
        note(census.window)
        // One visit per object per walk: `item` and `contentItem` are followed by name, and a property of that name
        // pointing back up the tree would otherwise walk forever.
        census.visited = ({})
        visit(census.window.contentItem, true, false)
    }

    /// The objects this walk has seen, keyed by their address-bearing string form.
    property var visited: ({})

    function note(object) {
        const text = String(object)
        const mark = text.indexOf("_QMLTYPE_")
        if (mark > 0)
            census.seen[text.substring(0, mark)] = true
    }

    /// Down the object tree, not the item tree: a Popup (every dialog, every menu) is an object under the item that
    /// declares it and no item at all, so `children` never names it — `data` (the default property: child items and
    /// resources alike) does, and its own content hangs off `contentItem` / `contentData`.
    ///
    /// `shown` is whether this branch is on screen. A popup that is declared but shut (every dialog and menu the
    /// window keeps ready) is not something this run showed, and neither is what sits inside it — but a popup that
    /// is open *is*, wherever it is declared: the chip's card is a menu declared inside another menu that stays shut.
    /// So a shut popup is still walked, with nothing under it noted until an open one turns the light back on.
    /// `offered` is the one exception: a menu declared in an open menu's content is a row of that menu (`AppMenu`
    /// offers a submenu as a row), so the parent showing is the submenu showing, open or not.
    function visit(object, shown, offered) {
        if (!object)
            return
        const key = String(object)
        // One visit per object per walk, **unless the second one is the showing one**: a property can point back up
        // the tree, so a branch can be met through it before the road that shows it is walked, and an early return
        // there would drop that branch and everything under it from the run's census.
        if (census.visited[key] === true || (census.visited[key] !== undefined && !shown))
            return
        census.visited[key] = shown === true
        if (object.opened !== undefined)
            shown = object.visible === true || offered === true
        if (shown)
            note(object)
        const owned = object.data
        if (owned !== undefined && owned !== null)
            for (let i = 0; i < owned.length; i++)
                visit(owned[i], shown, false)
        // A `QtObject` has no `data` and no `contentData`, so what it holds is reachable by name and no other road:
        // the author's card and the co-authors' card hang off `DetailsAuthorCards`, which is a QtObject because an
        // item declared in its seat would draw as a row. Without this the walk dead-ends there and **no verb can
        // record those cards at all** — the gate then stops by name on a component nothing can bring up.
        //
        // **Only a QML-defined holder is walked by name, and only for the values that are themselves QML-defined.**
        // A Popup has no `data` either but carries `contentData` and is reached the ordinary way; a model's property
        // surface is wide, names nothing this census records, and reading all of it on the clock would cost the run
        // what it is measuring.
        if (owned === undefined && object.contentData === undefined && key.indexOf("_QMLTYPE_") > 0)
            for (const name in object) {
                const held = object[name]
                if (held && typeof held === "object" && String(held).indexOf("_QMLTYPE_") > 0)
                    visit(held, shown, false)
            }
        const content = object.contentData
        if (content !== undefined && content !== null)
            for (let i = 0; i < content.length; i++)
                visit(content[i], shown, false)
        // A menu's content lists a submenu as the row that stands for it, never as the submenu itself (measured);
        // the submenus are asked for by index.
        if (object.menuAt !== undefined && object.count !== undefined)
            for (let i = 0; i < object.count; i++)
                visit(object.menuAt(i), shown, shown && object.opened !== undefined)
        const face = object.contentItem
        if (face !== undefined && face !== null && face !== object)
            visit(face, shown, false)
        // A Control's background is neither its content nor its data, and every card and every menu wears the same
        // one (`AppCardFace`), so without this road no verb shows it at all.
        const back = object.background
        if (back !== undefined && back !== null && back !== object)
            visit(back, shown, false)
        // A Loader's object is its child but not in its `data` when it is no item (the dialogs `WindowDialogSeat`
        // loads are popups), so it is reached by name.
        const loaded = object.item
        if (loaded !== undefined && loaded !== null && loaded !== object)
            visit(loaded, shown, false)
    }

    /// Whether the window has stopped arriving — the one point the walk is taken from, and a binding rather than a
    /// call so the wait below has an edge to hear (every term is a property read, so the dependencies are captured
    /// through `pageSettled()` as they would be inline).
    readonly property bool settled: census.pageSettled()

    /// Whether the page this run photographed had stopped arriving. The page verbs ask a shorter version of this of
    /// the act before they start (`AutoActDriver`); this one is what the census is walked from and so has to name
    /// every read, not only the ones a verb needs standing before it presses anything.
    ///
    /// **Every read the page is still waiting on is named here, the selected commit's and the graph's second pass
    /// included.** A predicate that stopped at the working tree, the refs and the graph called a run settled while the
    /// changed-file list was on its way, and the verbs that finish on something else of their own then wrote and
    /// unwrote one another's `FileRowDelegate` from run to run (measured: `ref-list 1 --preset tags` lost it in 2 runs
    /// of 8 taken at once, and in none of 10 taken one after another — the window opens under load, which is exactly
    /// where the gate runs).
    ///
    /// **`finishCount` counts passes, and the first one is not the last word.** The opening starts the log walk before
    /// the first status has been read, so whether that walk carries the working-tree row is a race the status wins
    /// about half the time; when it loses, the status asks for a rebuild and the row — with `WipTallyRow` on it —
    /// arrives a pass later. What settles it is the rows agreeing with the status: the graph says what it holds
    /// (`GraphModel.wipRow`) and the working tree says what should stand (`WorkTree.wipRowStands`), both off the one
    /// rule the walk itself is built from (`platitude_core::graph::wip_row_stands`). A graph that could not be walked
    /// is waiting for nothing and answers yes with whatever it holds — `graph-stopped` leaves an empty column, and a
    /// swap that failed (`stale`) leaves the pass before it standing.
    ///
    /// **The question is whether rows are still on their way, not whether the repository opened.** A tab that was
    /// never given a path (`open-picker`) and one whose path was refused (`open-not-a-repo`) are both done: nothing
    /// is coming, so the run photographed the whole of what its verb shows and may write its line. Answering no for
    /// those would hold the run open until the watchdog.
    function pageSettled() {
        const page = census.window ? census.window.curPage : null
        if (!page)
            return true
        const state = page.pageTab.state
        if (state === "" || state === "error")
            return true
        // An accepted path is only the beginning: the rows arrive with the reads behind it.
        const graph = page.pageGraph
        return state === "open" && page.pageWt.loaded
            && page.pageRefsLoaded && graph.finishCount > 0 && page.pageDetailsSettled
            && (graph.failed || graph.stale || graph.wipRow === page.pageWt.wipRowStands)
    }

    /// The run is finishing: the walk once the window has settled, then the report lines. Held open by the shot
    /// driver until this says it is done, so a walk is never taken from a window with a read still out.
    signal walked()
    property bool waiting: false
    /// The picture was called for before the window had settled, so the walk is from a later moment than the
    /// photograph. Held past `waiting`, which the walk clears before it says anything.
    property bool waited: false
    function report() {
        if (Harness.autoAct === "")
            return
        if (!census.settled) {
            census.waiting = true
            census.waited = true
            return
        }
        census.take()
    }
    onSettledChanged: if (census.waiting && census.settled) census.take()

    /// What the window answered `settled` with, beside the answer — so a census that moves says why on the run's own
    /// line rather than leaving the next reader to reproduce a machine.
    ///
    /// **Only the terms that can still differ once the answer is yes.** Every read `pageSettled` waits on is true
    /// whenever it says settled, so repeating those would say nothing. The rows-against-the-status term is the one
    /// with a way out: through `failed` or `stale` a run settles on whatever pass is standing, and that pass may hold
    /// no working-tree row at all. **`WipTallyRow` is the only component that stands or falls with that row** — it
    /// sits behind `GraphRowDelegate`'s loader, where every other `Wip*` stands in the tree with `visible` false and
    /// is counted anyway — so these words are what says whether the row it hangs on was there to be met.
    ///
    /// `waited` is which moment the walk came from: the picture's, or a later one the run held itself open for.
    function terms() {
        const page = census.window ? census.window.curPage : null
        const said = "waited=" + census.waited
        if (!page)
            return said + " state=nopage"
        const graph = page.pageGraph
        const state = page.pageTab.state === "" ? "none" : page.pageTab.state
        return said + " state=" + state + " finish=" + graph.finishCount
            + " failed=" + graph.failed + " stale=" + graph.stale + " wipRow=" + graph.wipRow
            + " wipRowStands=" + page.pageWt.wipRowStands
    }

    /// The walk and the two lines, taken once however many times the window settles.
    function take() {
        census.waiting = false
        walk()
        // Said first, and on a line of its own: everything after `census=` is a name. The terms ride on the answer's
        // line, which is read one word at a time (`gate::census::page_settled_in`).
        Harness.report("census page=" + (census.settled ? "settled" : "arriving") + " " + census.terms())
        Harness.report("census=" + Object.keys(census.seen).sort().join(","))
        census.walked()
    }
}
