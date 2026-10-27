pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Every QML component a verb's run shows, by type name, walked at two moments and never merged: at the verb's
/// completion (`markCompletion`) and at each picture (`reportAt`). The picture's walk is the census — `census=A,B,C`,
/// which verify-ui holds against the line's registered row (`xtask::verify::observed`; the table itself,
/// `crates/xtask/verb-lines.txt`, is written by people) — and the two walks held against each other are the one
/// claim every run makes of every picture: the parts standing in it are the parts the verb completed on
/// (反映前テストの機械化 §観測 / §検証時点; the walk's rules are the `WindowCensus` line of rules-refs/app-ui.md).
/// Not `went=`: another verb's line uses it, and `must_say` matches by substring.
///
/// Popups stand under the window's overlay, a child of the root item, so one walk from the root sees them too.
Item {
    id: census

    /// The window whose tree is walked. `var` because `Main` is the file the engine loads.
    required property var window
    /// The harness's own branch, which holds the window's parts by name to act on them: what an item there holds is
    /// not what the run showed (`harnessHolds`).
    property Item harnessBranch: null

    /// The names the current walk met, as keys.
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
            census.met[text.substring(0, mark)] = true
        }
    }

    /// A pooled delegate can still be `visible` and keep its old index; the view no longer returns it for that row.
    /// Headers and footers have the attached view too, but no row index. A row's `index` can be an action value
    /// (`ChosenCommitRow` always says -1), so find its list position by its geometry, not that property's value.
    /// A delegate the view keeps beyond its viewport (the cache buffer, built a frame at a time) is in no picture
    /// and counts for nothing: the census is what the picture shows.
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
        if (view.itemAtIndex(view.indexAt(point.x, point.y)) !== object)
            return false
        const box = object.mapToItem(view, 0, 0, object.width, object.height)
        return box.x < view.width && box.x + box.width > 0 && box.y < view.height && box.y + box.height > 0
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

    /// The one rule (`PageSettled`) of the page in front — the same the page and badge verbs wait on, never a copy.
    function pageSettled() {
        return census.pageMissing().length === 0
    }
    /// The page's reads still on their way (`PageSettled.missing`), for the completion's line and the census's.
    function pageMissing() {
        return PageSettled.missing(census.window ? census.window.curPage : null)
    }

    /// What the walk at the verb's completion met, held against each picture's walk; null until the verb completes.
    /// Never moved after: a view that builds its rows, or a read that lands, between the completion and a picture
    /// is a verb that completed on a scene still moving, and the picture's claim says so.
    property var atCompletion: null

    /// The walk at the verb's completion (`WindowHarness.finish`), in the completing turn: the set of parts the run
    /// completed on.
    function markCompletion() {
        census.walk()
        census.atCompletion = census.met
    }

    /// The walk at a picture (`AutoShotDriver.pictured`), in the picture's own turn. The claim every run makes of
    /// every picture: `pictured … name=window holds=` — the parts standing are the parts it completed on, named by
    /// what came (`gained=`) and what went (`lost=`); a read landing between the completion and the picture moves
    /// the set, and the picture is then of a state the run never claimed — and the page as it stood then (`page=`),
    /// judged as the completion's is: a picture of a page still arriving is a breach unless the verb named the read
    /// (`Awaited.arriving`), whatever the parts say. At the app's picture, the census itself follows:
    /// `census=<names>`, read one word at a time by the parent (`verify::observed::Observed::in_lines`).
    function reportAt(artifact) {
        if (Harness.autoAct === "")
            return
        census.walk()
        const before = census.atCompletion === null ? ({}) : census.atCompletion
        const gained = Object.keys(census.met).filter(name => before[name] !== true).sort()
        const lost = Object.keys(before).filter(name => census.met[name] !== true).sort()
        const holds = census.atCompletion !== null && gained.length === 0 && lost.length === 0
        const owed = census.pageMissing()
        const page = owed.length === 0 ? "settled" : "arriving:" + owed.join(",")
        Harness.report("pictured run=" + Awaited.runId + " artifact=" + artifact + " name=window holds=" + holds
                       + " gained=" + (gained.length === 0 ? "none" : gained.join("/"))
                       + " lost=" + (lost.length === 0 ? "none" : lost.join("/"))
                       + " page=" + page)
        if (artifact !== "app")
            return
        Harness.report("census run=" + Awaited.runId + " page=" + page + " " + census.terms())
        Harness.report("census=" + Object.keys(census.met).sort().join(","))
    }

    /// Why the page said `settled` or `arriving` at the picture, on the run's own line. Only terms that can still
    /// differ once it says yes: through `failed` / `stale` a run settles on whatever pass stands, which may hold
    /// no worktree row — and `WipTallyRow` (behind `GraphRowDelegate`'s loader) is the one component that stands
    /// or falls with that row.
    function terms() {
        const page = census.window ? census.window.curPage : null
        if (!page)
            return "state=nopage"
        const graph = page.pageGraph
        const state = page.pageTab.state === "" ? "none" : page.pageTab.state
        return "state=" + state + " finish=" + graph.finishCount
            + " failed=" + graph.failed + " stale=" + graph.stale + " wipRow=" + graph.wipRow
            + " wipRowStands=" + page.pageWorktree.wipRowStands
    }
}
