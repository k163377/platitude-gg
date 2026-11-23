import QtQuick

/// Keeps a list's place through a swap of the bars over it, when the two that trade places stand at the same height
/// (`BarTravel`) but reach the list in different passes: one anchored beside the list moves it at once, one in a
/// `Layout` around it a polish later. In between the list is taller or shorter than it ends up, and Flickable pulls a
/// view grown past the end of its rows back on the spot (`QQuickFlickable::geometryChange` → `fixupY`) — the place
/// would stay short by the bar's height once the list is given its height back, and the frame would draw the rows
/// that much off. So the place is noted before the swap (`keep`) and put back each time the list's height changes
/// until that frame is laid out (`afterAnimating`): the polish's change comes before the list lays its rows out for
/// it, and Flickable's own fixup runs after the handler. Put back through the view's `clampY`, so a span or an end
/// the view keeps to still holds (`GraphList.keepToSpan`, run a turn later on the passing height, is undone too).
///
/// Declared as a readonly property's value, never as a bare child (`TipAsk`).
QtObject {
    id: place

    /// The list (`AppListView`, with its `clampY`).
    property Flickable view: null
    /// The view is not moving on its own — a flick, a glide: a view on the move is left to it.
    property bool still: true
    /// A swap is under way: the place noted (`keptY`) is put back as the list's height changes, till the frame after
    /// the swap is laid out.
    property bool keeping: false
    property real keptY: 0

    /// Notes the view's place: a swap of the bars over it is about to happen.
    function keep() {
        if (place.view === null || !place.still)
            return
        place.keptY = place.view.contentY
        place.keeping = true
        // The swap may move nothing the frame draws; the keeping ends at the frame that comes next.
        if (place.view.Window.window !== null)
            place.view.Window.window.update()
    }
    readonly property Connections heightWatch: Connections {
        target: place.keeping ? place.view : null
        function onHeightChanged() {
            const y = place.view.clampY(place.keptY)
            if (Math.abs(y - place.view.contentY) > 0.5)
                place.view.contentY = y
        }
    }
    readonly property Connections frameWatch: Connections {
        target: place.keeping && place.view !== null ? place.view.Window.window : null
        function onAfterAnimating() {
            place.keeping = false
        }
    }
}
