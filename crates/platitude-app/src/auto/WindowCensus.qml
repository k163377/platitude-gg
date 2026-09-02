pragma ComponentBehavior: Bound

import QtQuick
import platitude

/// Every QML component alive in the window at any moment of a verb's run, by type name.
///
/// The gate (`cargo xtask gate`) owes a change the verbs that show what it touched, and this is how a verb says what it
/// shows: the item tree is walked on a short clock through the run and once more as the run finishes, and every type
/// met is reported as `census=A,B,C` for verify-ui to record against the line that ran (`crates/xtask/verb-census.txt`).
/// A QML-defined type answers `String(item)` with `<File>_QMLTYPE_<n>(0x…)`, so the file's name is the part before the
/// mark; an inline component answers with its bare name and a C++ type with its class, and neither names a file, so
/// the runner drops them. Popups stand under the window's overlay, which is a child of the root item, so one walk from
/// the root sees them too. A base a component derives from is not met — the object carries the derived name — which
/// is why the gate follows the type-reference graph as well.
Item {
    id: census

    /// The window whose tree is walked. `var` because `Main` is the file the engine loads, not a type anything names.
    required property var window

    /// The names met so far, as an object used for its keys.
    property var seen: ({})

    function begin() {
        if (AppBackend.autoAct !== "")
            clock.start()
    }

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
        // A Loader's object is its child but not in its `data` when it is no item (the dialogs `WindowDialogSeat`
        // loads are popups), so it is reached by name.
        const loaded = object.item
        if (loaded !== undefined && loaded !== null && loaded !== object)
            visit(loaded, shown, false)
    }

    /// The run is finishing: one last walk, then the report line.
    function report() {
        clock.stop()
        if (AppBackend.autoAct === "")
            return
        walk()
        AppBackend.report("census=" + Object.keys(census.seen).sort().join(","))
    }

    Timer {
        id: clock
        interval: 100
        repeat: true
        onTriggered: census.walk()
    }
}
