pragma ComponentBehavior: Bound

import QtQuick
import platitude

/// Every QML component the window is showing as a verb's run finishes, by type name.
///
/// The gate (`cargo xtask gate`) owes a change the verbs that show what it touched, and this is how a verb says what it
/// shows: the item tree is walked once, on the picture the run saved, and every type met is reported as
/// `census=A,B,C` for verify-ui to record against the line that ran (`crates/xtask/verb-census.txt`).
///
/// **One walk, on a window that has stopped arriving.** A sampled walk records where its ticks landed: a
/// row a run raises and takes away again is in the census of the machine whose tick caught it and out of
/// the census of the machine whose tick came late, so the file the gate reads moves under work that
/// never touched the application (measured: `avatar-remove` names `AvatarAssignRow` for its ticks
/// and nothing else).
///
/// **The point is past the verb's edge.** A verb is finished when what it acted on answers, and the repository is
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
/// **And the walk is taken from a frame.** What the tree holds is what the views have built, and a view
/// builds its rows when the window polishes its items for a frame. Two nearer moments each record a list
/// with no rows in it. The picture's callback comes as a posted event, so whatever was queued ahead of it
/// runs first — a diff's rows arriving is a model reset, and a list met between its reset and the next
/// polish holds no delegate at all. And the edge on which the window settles is a data edge: the refs land
/// and the nav list has yet to build a row from them, so a walk taken in the same call as that edge records
/// a sidebar with no rows in it (measured: `tab-hold 0` lost `NameCell NavItemDelegate NavRowBody` on the
/// runs whose refs were the last read to land). So once the picture has been called for and the window has
/// settled, a frame is asked for, and the walk is taken at `afterAnimating` — the window's word, on this
/// thread, that every item has been polished for the frame about to be synced, with nothing able to run in between.
///
/// A QML-defined type answers `String(item)` with `<File>_QMLTYPE_<n>(0x…)`, so the file's name is the part before the
/// mark; an inline component answers with its bare name and a C++ type with its class, and neither names a file, so
/// the runner drops them. Popups stand under the window's overlay, which is a child of the root item, so one walk from
/// the root sees them too. The object carries only the derived name, which is why the gate follows the
/// type-reference graph as well.
Item {
    id: census

    /// The window whose tree is walked. `var` because `Main` is the file the engine loads.
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

    /// Down the object tree: a Popup (every dialog, every menu) is an object under the item that declares it and no
    /// item at all, so `children` never names it — `data` (the default property: child items and resources alike)
    /// does, and its own content hangs off `contentItem` / `contentData`.
    ///
    /// `shown` is whether this branch is on screen. A popup that is open is something this run showed,
    /// wherever it is declared: the chip's card is a menu declared inside another menu that stays shut.
    /// So a shut popup (every dialog and menu the window keeps ready) is still walked, with nothing under
    /// it noted until an open one turns the light back on. `offered` is the one exception: a menu declared
    /// in an open menu's content is a row of that menu (`AppMenu` offers a submenu as a row), so the parent
    /// showing is the submenu showing, open or not.
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
        // A menu's content lists a submenu as the row that stands for it (measured); the submenus are asked for by
        // index.
        if (object.menuAt !== undefined && object.count !== undefined)
            for (let i = 0; i < object.count; i++)
                visit(object.menuAt(i), shown, shown && object.opened !== undefined)
        const face = object.contentItem
        if (face !== undefined && face !== null && face !== object)
            visit(face, shown, false)
        // A Control's background is a property of its own, and every card and every menu wears the same
        // one (`AppCardFace`), so without this road no verb shows it at all.
        const back = object.background
        if (back !== undefined && back !== null && back !== object)
            visit(back, shown, false)
        // A Loader's object sits outside its `data` when it is no item (the dialogs `WindowDialogSeat`
        // loads are popups), so it is reached by name.
        const loaded = object.item
        if (loaded !== undefined && loaded !== null && loaded !== object)
            visit(loaded, shown, false)
    }

    /// Whether the window has stopped arriving — the one point the walk is taken from, and a binding so the wait
    /// below has an edge to hear (every term is a property read, so the dependencies are captured
    /// through `pageSettled()` as they would be inline).
    readonly property bool settled: census.pageSettled()

    /// Whether the page this run photographed had stopped arriving — the one rule (`PageSettled`), asked of the page
    /// the window is showing. The page verbs ask the same rule of the act before they start (`AutoActDriver`), and the
    /// badge verbs before they arm their fault (`WindowBadgeActs`); what is walked here is what the rest waited for.
    function pageSettled() {
        return PageSettled.settled(census.window ? census.window.curPage : null)
    }

    /// The run is finishing: the walk once the window has settled, then the report lines. Held open by the shot
    /// driver until this says it is done, so every walk is taken from a window whose reads have landed.
    signal walked()
    property bool waiting: false
    /// The picture was called for before the window had settled, so the walk is from a later moment than the
    /// photograph. Held past `waiting`, which the walk clears before it says anything.
    property bool waited: false
    /// A frame has been asked for, and the walk is taken at its `afterAnimating` — unless a read goes out first, in
    /// which case the settled edge that follows arms the next one.
    property bool armed: false
    function report() {
        if (Harness.autoAct === "")
            return
        census.waiting = true
        census.waited = !census.settled
        census.arm()
    }
    onSettledChanged: {
        if (census.settled)
            census.arm()
        else
            census.armed = false
    }
    function arm() {
        if (!census.waiting || !census.settled)
            return
        census.armed = true
        // A quiet scene renders no frame of its own; asked for one, the window polishes its items and says so.
        census.window.requestUpdate()
    }
    Connections {
        target: census.window
        enabled: census.armed
        function onAfterAnimating() {
            // Still settled: a read that went out between the ask and the frame is a window arriving again, and the
            // edge on which it stops is what arms the frame after.
            if (!census.settled) {
                census.armed = false
                return
            }
            census.take()
        }
    }

    /// What the window answered `settled` with, beside the answer — so a census that moves says why on the run's own
    /// line.
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
        census.armed = false
        walk()
        // Said first, and on a line of its own: everything after `census=` is a name. The terms ride on the answer's
        // line, which is read one word at a time (`gate::census::page_settled_in`).
        Harness.report("census page=" + (census.settled ? "settled" : "arriving") + " " + census.terms())
        Harness.report("census=" + Object.keys(census.seen).sort().join(","))
        // Said from the event loop: the ending this releases may quit the application, and the frame is left to
        // finish first.
        Qt.callLater(census.done)
    }
    function done() {
        census.walked()
    }
}
