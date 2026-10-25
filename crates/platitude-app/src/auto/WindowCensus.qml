pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Every QML component a verb's run showed, by type name, reported as `census=A,B,C` for verify-ui to record
/// (`crates/xtask/verb-census.txt`). The union of two walks — at the picture and at the settled edge — with `lost=`
/// naming what the picture had and the settled edge did not (反映前テストの機械化 §census; the walk's rules are the
/// `WindowCensus` line of rules-refs/app-ui.md). Not `went=`: another verb's line uses it, and `must_say` matches by
/// substring.
///
/// Popups stand under the window's overlay, a child of the root item, so one walk from the root sees them too.
Item {
    id: census

    /// The window whose tree is walked. `var` because `Main` is the file the engine loads.
    required property var window
    /// The harness's own branch, which holds the window's parts by name to act on them: what an item there holds is
    /// not what the run showed (`harnessHolds`).
    property Item harnessBranch: null

    /// Every name any walk met, as keys — the census itself.
    property var seen: ({})
    /// The names the current walk met, to hold the two walks against each other.
    property var met: ({})

    /// The window is noted itself: a Window is no item, so the root item's tree never names `Main`.
    function walk() {
        census.met = ({})
        if (!census.window || !census.window.contentItem)
            return
        note(census.window)
        // Per walk: properties followed by name can point back up the tree, which would otherwise walk forever.
        census.visited = ({})
        visit(census.window.contentItem, true, false)
    }

    /// The objects this walk visited, by their address-bearing string; `true` once visited as shown.
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

    /// A pooled delegate can still be `visible` and keep its old index; the view no longer returns it for that row.
    /// Headers and footers have the attached view too, but no row index. A row's `index` can be an action value
    /// (`ChosenCommitRow` always says -1), so find its list position by its geometry, not that property's value.
    function inList(object) {
        if (!(object instanceof Item))
            return true
        // Only direct children of a list's content item can be its delegates. Reading the attached object on
        // every Item would create ListView's attached state across the fixed tree just to discover it has no view.
        const holder = object.parent
        if (!holder || !(holder.parent instanceof ListView) || holder.parent.contentItem !== holder)
            return true
        const view = object.ListView.view
        if (!view)
            return true
        const hasRowIndex = object.index !== undefined || (object.model && object.model.index !== undefined)
        if (!hasRowIndex)
            return true
        if (!object.visible)
            return false
        const point = object.mapToItem(view.contentItem, object.width / 2, object.height / 2)
        return view.itemAtIndex(view.indexAt(point.x, point.y)) === object
    }

    /// What every item has, so the names a type adds stand out.
    readonly property Item plain: Item {}
    property var itemNames: null
    /// By type, the names its items hold a QML-defined `QtObject` part under: one object (a list such as
    /// `contentChildren` prints its rows' names), no item, no `parent` of its own as a handler or a popup has, and no
    /// children as a window has (an item holding the window would walk every popup under the overlay as shown).
    /// Found once a type, on its first item, among the names a plain item lacks: an item's property surface is wide,
    /// and some of it (`palette`) is made by being read.
    property var heldNames: ({})

    function heldBy(object, key) {
        const at = key.indexOf("(")
        const type = at > 0 ? key.substring(0, at) : key
        let names = census.heldNames[type]
        if (names !== undefined)
            return names
        if (census.itemNames === null) {
            census.itemNames = ({})
            for (const name in census.plain)
                census.itemNames[name] = true
        }
        names = []
        for (const name in object) {
            if (census.itemNames[name] === true)
                continue
            const value = object[name]
            if (value && typeof value === "object" && !(value instanceof Item) && value.parent === undefined
                    && value.length === undefined && value.data === undefined && value.contentData === undefined
                    && String(value).indexOf("_QMLTYPE_") > 0)
                names.push(name)
        }
        census.heldNames[type] = names
        return names
    }

    function harnessHolds(item) {
        for (let at = item; at; at = at.parent)
            if (at === census.harnessBranch)
                return true
        return false
    }

    /// Down through `data`, not `children` — a Popup is no item. `shown` is whether this branch is on screen: a shut
    /// popup is still walked (an open one can be declared inside it) with nothing noted until an open one. `offered`
    /// is a menu in an open menu's content, which shows as that menu's row whether open or not.
    function visit(object, shown, offered) {
        if (!object)
            return
        const key = String(object)
        // A list keeps pooled delegates alive, and hidden panes can receive their rows after the pane went away.
        // Their existence depends on arrival order; count only the delegates the visible list still owns.
        // Popups can stand outside a hidden declaring branch, and an offered submenu stands as its parent's row.
        if (object.opened !== undefined)
            shown = object.visible === true || offered === true
        else if (!census.inList(object))
            shown = false
        // A second visit goes through when it is the showing one: a branch first met through a property pointing
        // back up the tree would otherwise drop out of the census.
        if (census.visited[key] === true || (census.visited[key] !== undefined && !shown))
            return
        census.visited[key] = shown === true
        if (shown)
            note(object)
        const owned = object.data
        if (owned !== undefined && owned !== null)
            for (let i = 0; i < owned.length; i++)
                visit(owned[i], shown, false)
        // A `QtObject` holds its children by name only (the author cards hang off `DetailsAuthorCards`), so without
        // this no verb could record them. Only QML-defined holders and values: a model's property surface is wide,
        // records nothing, and reading it all costs the run what it measures.
        if (owned === undefined && object.contentData === undefined && key.indexOf("_QMLTYPE_") > 0)
            for (const name in object) {
                const held = object[name]
                if (held && typeof held === "object" && String(held).indexOf("_QMLTYPE_") > 0)
                    visit(held, shown, false)
            }
        // An item holds its non-visual parts by name too, never in `data` (rules-refs/app-ui.md「Item の中に置く
        // `QtObject` 製の部品」).
        if (owned !== undefined && key.indexOf("_QML") > 0) {
            const names = census.heldBy(object, key)
            if (names.length > 0 && !census.harnessHolds(object))
                for (const name of names)
                    visit(object[name], shown, false)
        }
        const content = object.contentData
        if (content !== undefined && content !== null)
            for (let i = 0; i < content.length; i++)
                visit(content[i], shown, false)
        // A menu's content lists a submenu only as its row; the submenus are asked for by index.
        if (object.menuAt !== undefined && object.count !== undefined)
            for (let i = 0; i < object.count; i++)
                visit(object.menuAt(i), shown, shown && object.opened !== undefined)
        const face = object.contentItem
        if (face !== undefined && face !== null && face !== object)
            visit(face, shown, false)
        // `background` is a property of its own, and every card and menu wears `AppCardFace` there.
        const back = object.background
        if (back !== undefined && back !== null && back !== object)
            visit(back, shown, false)
        // A Loader's object sits outside its `data` when it is no item (the popups `WindowDialogSeat` loads).
        const loaded = object.item
        if (loaded !== undefined && loaded !== null && loaded !== object)
            visit(loaded, shown, false)
    }

    /// The page's property edges wake the wait; the core's counted reads are queried again by the sampler below.
    readonly property bool settled: census.pageSettled()

    /// The one rule (`PageSettled`) of the page in front — the same the page and badge verbs wait on, never a copy.
    function pageSettled() {
        return PageSettled.settled(census.window ? census.window.curPage : null)
    }

    /// Both walks are reported; the shot driver holds the run open until this, so the second walk is never quit out
    /// from under.
    signal walked()
    property bool waiting: false
    /// The picture came before the window settled, so the second walk is from a later moment. Kept past `waiting`,
    /// which that walk clears before it reports.
    property bool waited: false
    /// A frame has been asked for, and the settled walk is taken at its `afterAnimating` — unless a read goes out
    /// first, and the settled edge after it arms the next one.
    property bool armed: false
    /// What the picture's walk met (`lost`).
    property var atShot: ({})
    function report() {
        if (Harness.autoAct === "")
            return
        census.waiting = true
        census.waited = !census.pageSettled()
        // In the picture's own turn, not on a frame: waiting for one is the stretch where the verb's leftover beats
        // fire.
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
        if (!census.waiting || census.armed || !census.pageSettled())
            return
        census.armed = true
        // A quiet scene draws no frame of its own; asked for one, the window polishes its items and says so.
        census.window.requestUpdate()
    }
    Connections {
        target: census.window
        enabled: census.armed
        function onAfterAnimating() {
            // A read went out since the ask: the next settled edge re-arms.
            if (!census.pageSettled()) {
                census.armed = false
                return
            }
            census.take()
        }
    }

    // Core's counted reads can end without changing a model (an unchanged graph pass). Its query has no property
    // edge, so sample the same predicate rather than relying on the binding above to wake this last part of the run.
    SampleTimer {
        running: census.waiting
        onTriggered: census.arm()
    }

    /// Why the window said `settled`, on the run's own line, so a census that moves says why. Only terms that can
    /// still differ once it says yes: through `failed` / `stale` a run settles on whatever pass stands, which may hold
    /// no worktree row — and `WipTallyRow` (behind `GraphRowDelegate`'s loader) is the one component that stands
    /// or falls with that row.
    function terms() {
        const page = census.window ? census.window.curPage : null
        const said = "waited=" + census.waited
        if (!page)
            return said + " state=nopage"
        const graph = page.pageGraph
        const state = page.pageTab.state === "" ? "none" : page.pageTab.state
        return said + " state=" + state + " finish=" + graph.finishCount
            + " failed=" + graph.failed + " stale=" + graph.stale + " wipRow=" + graph.wipRow
            + " wipRowStands=" + page.pageWorktree.wipRowStands
    }

    /// What the picture's walk met and this one did not: taken away by what the verb left running.
    function lost() {
        return Object.keys(census.atShot).filter(name => census.met[name] !== true).sort()
    }

    /// The settled edge's walk and the two lines, taken once however many times the window settles.
    function take() {
        census.waiting = false
        census.armed = false
        walk()
        const left = census.lost()
        // A line of its own, first: everything after `census=` is a name. The answer's line is read one word at a
        // time (`gate::census::page_settled_in`).
        Harness.report("census page=" + (census.pageSettled() ? "settled" : "arriving") + " " + census.terms()
                       + " lost=" + (left.length === 0 ? "none" : left.join("/")))
        Harness.report("census=" + Object.keys(census.seen).sort().join(","))
        // From the event loop: the ending this releases may quit the application, so the frame finishes first.
        Qt.callLater(census.done)
    }
    function done() {
        census.walked()
    }
}
