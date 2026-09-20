pragma ComponentBehavior: Bound

import QtQuick
import platitude

/// Every QML component a verb's run showed, by type name.
///
/// The gate (`cargo xtask gate`) owes a change the verbs that show what it touched, and this is how a verb says what it
/// shows: the item tree is walked and every type met is reported as `census=A,B,C` for verify-ui to record against the
/// line that ran (`crates/xtask/verb-census.txt`).
///
/// **The run has two edges, and the census is both of them together.** A walk is one instant, and the file it writes is
/// checked in, so an instant the run does not choose is an instant the machine chooses — and the file then moves under
/// work that never touched the application. The run owns exactly two moments:
///
/// - **the picture** — the window the run photographed, which is the window a person judges. Walked in the same turn
///   the picture is saved in (`AutoShotDriver.appPictured`), before anything the verb left running has had a turn.
/// - **the settled edge** — the window once its reads have landed (`PageSettled`), on a polished frame. A verb is
///   finished when what it acted on answers, and the repository is still being read behind it: the opening walks the
///   log before the first status has said whether anything is uncommitted, so the working-tree row — and the tallies on
///   it — arrive with the pass that status asks for (measured 2026-09-05: `settings-eol true` lost `WipTallyRow` in 4
///   runs of 10 taken one after another, and `file-menu-conflict both.txt --preset conflict-kinds` in 10 of 10 taken at
///   once). Its frame is the window's word that every item has been polished, which two nearer moments are not: the
///   picture's callback comes as a posted event, so a model reset queued ahead of it leaves a list holding no delegate
///   at all, and the data edge itself lands before the nav list has built a row from it (measured: `tab-hold 0` lost
///   `NameCell NavItemDelegate NavRowBody` on the runs whose refs were the last read to land).
///
/// **Neither moment alone is a verb's answer, and together they are.** What is between them is a stretch of wall clock
/// the run does not hold — hundreds of milliseconds under a loaded machine — and every beat the verb left running gets
/// a turn in it: the hover keep (`HoverCardHost`, `Metrics.hoverKeepMs`), a popup's exit, a list's rebuild. Recorded
/// from the settled edge alone, a card the picture holds is in the census of the fast run and out of the census of the
/// slow one (measured 2026-09-20: `nav-peek-out branch`, 24 runs at once, lost `SectionPeekPopup AppCardFace` in 5 of
/// them and photographed the card in all 24). Recorded from the picture alone, the rows a read was still bringing are
/// out. So both are walked and the names are added together: a name has to be met by neither walk to be out, and
/// nothing that happens between them can put it there.
///
/// **What the picture had and the settled edge did not is the run saying it left something moving** — reported as
/// `lost=`, the one thing a verb can do about its own census. A verb waits for what it set in motion
/// (app-ui.md §UI 自動化の因果性); a `lost=` is the name of what it did not. (`went=` is spoken for: one verb's own
/// report line uses it for which of two bars went, and `must_say` reads a line by substring.)
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

    /// Every name any walk of this run met, as an object used for its keys — the census itself.
    property var seen: ({})
    /// The names this walk alone met, so the two walks can be held against each other.
    property var met: ({})

    /// One walk: the window itself (a Window is no item, so the root item's tree never names `Main`), then from the
    /// root item down. Adds to `seen` and answers in `met`.
    function walk() {
        census.met = ({})
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
        if (mark > 0) {
            const name = text.substring(0, mark)
            census.seen[name] = true
            census.met[name] = true
        }
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

    /// The run is finishing: the picture's walk now, the settled edge's when the reads have landed, then the report
    /// lines. Held open by the shot driver until this says it is done, so the second walk is never quit out from under.
    signal walked()
    property bool waiting: false
    /// The picture was called for before the window had settled, so the second walk is from a later moment than the
    /// photograph. Held past `waiting`, which that walk clears before it says anything.
    property bool waited: false
    /// A frame has been asked for, and the settled walk is taken at its `afterAnimating` — unless a read goes out
    /// first, in which case the settled edge that follows arms the next one.
    property bool armed: false
    /// What the picture's walk met, kept to be held against the settled edge's (`lost`).
    property var atShot: ({})
    function report() {
        if (Harness.autoAct === "")
            return
        census.waiting = true
        census.waited = !census.settled
        // The picture's own turn. Taken here rather than from a frame of its own, because a frame is asked for and
        // waited on, and that wait is the stretch the beats the verb left running fire in.
        census.walk()
        census.atShot = census.met
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

    /// What the picture held and this walk did not meet — the beats the verb left running, by the name of what they
    /// took away. Nothing here means the window the run photographed was the window it left behind.
    function lost() {
        return Object.keys(census.atShot).filter(name => census.met[name] !== true).sort()
    }

    /// The settled edge's walk and the two lines, taken once however many times the window settles.
    function take() {
        census.waiting = false
        census.armed = false
        walk()
        const left = census.lost()
        // Said first, and on a line of its own: everything after `census=` is a name. The terms ride on the answer's
        // line, which is read one word at a time (`gate::census::page_settled_in`).
        Harness.report("census page=" + (census.settled ? "settled" : "arriving") + " " + census.terms()
                       + " lost=" + (left.length === 0 ? "none" : left.join("/")))
        Harness.report("census=" + Object.keys(census.seen).sort().join(","))
        // Said from the event loop: the ending this releases may quit the application, and the frame is left to
        // finish first.
        Qt.callLater(census.done)
    }
    function done() {
        census.walked()
    }
}
