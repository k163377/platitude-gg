import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The box a sidebar row's name goes into: the one for a new branch's name and the one a rename opens are the same
// field, told apart by the mode (`SidebarRowGestures`). Nothing is asked before it opens or when it is walked away
// from — what it costs is the typing (デザイン規約 §可否・警告の出し場所).
//
// **Drawn outside the list its row is in.** What the box holds is the whole of what it is for — the question while
// nothing has been typed, the name it opened with while something has — and a box too narrow to show it asks nothing
// (デザイン規約 §グラフ行のダブルクリック「箱はプレースホルダを切らない」). This side of the window has no lane to spill into, so it
// spills out of the pane and over the graph: head held where the name's head was, growing the one way, and out only
// while a hand is on it — the one direction the chip's card opens in.
SlimField {
    id: box

    /// The row this box stands on, and the seat in it the box's head is held to. Both are read for their geometry:
    /// the row for where it sits and how tall it is, the seat for where the name began and how much of the row is
    /// going spare.
    required property Item row
    required property Item seat
    /// Where the box is drawn. **Not the list** — a list clips, and this one is allowed past the pane's edge. Not the
    /// column the sections are laid out in either: a layout lays out whatever is parented into it, and a box put there
    /// is given the column's own next row (measured: it landed at the foot of the pane). What that column fills lays
    /// nothing out and clips nothing. Nothing at all keeps the box in its seat, which is the answer for a surface
    /// where reaching out of it would reach out of a floating panel.
    property Item drawnIn: null
    /// Where the list's rows begin in that layer, where the list itself begins, and how tall it is — handed down so
    /// that a scroll carries the box with the row it belongs to, and so that a row carried out of the list takes the
    /// box with it (`NavList`).
    property real rowsX: 0
    property real rowsY: 0
    property real rowsTop: 0
    property real rowsHeight: 0
    /// Whether this row is the one being typed into, what the box is being opened for ("branch" / "tag" / "rename"),
    /// and what it is carrying — all of it the sidebar's, since a delegate is recycled the moment its row scrolls off.
    property bool editing: false
    property string mode: ""
    property string carried: ""
    property string refusedWhy: ""
    /// What kind of ref this box is naming, so the frame can say it
    /// (`SlimField.focusTone` — §ref の種別: 枠 = 種別). The row's kind
    /// rather than the mode alone: a tag is being named whether it is
    /// being made or renamed.
    property string namesKind: ""

    /// Typed into, accepted, walked away from. **Not `accepted`** — that name is the field's own, for Enter landing in
    /// it, and this one is what the row does about it.
    signal typed(string text)
    signal submitted(string text)
    signal cancelled()

    /// Where this row's top sits in the layer, and whether the whole of the row is inside the list showing it.
    readonly property real rowTop: box.rowsY + box.row.y
    readonly property bool rowInView: box.rowTop >= box.rowsTop
        && box.rowTop + box.row.height <= box.rowsTop + box.rowsHeight
    /// What the box has to be able to show. Latched when it opens rather than followed as it is typed into: a box
    /// whose right edge walks out from under the caret is one nobody can aim at, and what it opens holding is the
    /// reason it opens wide (app-ui.md §測って決める値は押し出す).
    property string inkText: ""
    readonly property real wantWidth:
        Math.ceil(ink.implicitWidth) + box.leftPadding + box.rightPadding

    // The box carries the name into itself when it opens, and again when a scrolled-off row is built anew — the
    // delegate is recycled and the text is not its to keep. Watched here rather than from the row: the row's own
    // handler for the same change can run before this one's binding has caught up, and a box that read `editing`
    // false there came up with nothing in it and no keyboard (measured).
    onEditingChanged: box.takeFocus()
    Component.onCompleted: box.takeFocus()
    function takeFocus() {
        if (!box.editing)
            return
        box.text = box.carried
        // What the box has to show, taken as it opens: the question where nothing is in it, the name where something
        // is. A rebuilt row measures whatever it is coming back holding.
        box.inkText = box.carried === "" ? box.placeholderText : box.carried
        box.selectAll()
        box.forceActiveFocus()
    }

    /// Whether the box is drawn out in the layer rather than in its seat.
    ///
    /// **Not switched on `editing`.** Standing the box out only while its row is the one being typed into would spare
    /// the layer the boxes of the rows that are not — but an item that changes parent loses the keyboard, and the
    /// change lands either side of the hand-off that gives it (measured: `focused=false`). What the layer holds is one
    /// box per row the view has actually built, which is what fits on screen and a little either side.
    readonly property bool out: box.drawnIn !== null
    parent: box.out ? box.drawnIn : box.seat
    // Out of the layer with the row that carries it: a box left standing where its row has scrolled away is a box over
    // the graph belonging to nothing.
    visible: box.editing && (!box.out || box.rowInView)
    // The head, which is where the name's head was: the row's indent, the seat, and the way between them — all of it
    // the row's own layout has worked out already, so it is read off the seat rather than added up again.
    x: box.out ? box.rowsX + box.seat.x : 0
    y: (box.out ? box.rowsY + box.row.y : 0) + (box.row.height - box.height) / 2
    // Never narrower than the seat, and never so far out that it runs off the window (the cap the cards take, デザイン規約
    // §メニュー). Held to the seat where there is no layer to grow into.
    //
    // The window's own width is the measure because **the layer is the left menu, and the left menu begins at the
    // window's left edge** — it is the first pane of the row of them, so `x` here is a window coordinate.
    width: !box.out ? box.seat.width
         : Math.max(box.seat.width,
                    Math.min(box.wantWidth, box.Window.width - box.x - Theme.spaceXxl))
    font.pixelSize: Theme.fontMd
    // Only the tag has a colour of its own to say here. A branch's is the focus ring's own value, a remote's is grey
    // and a stash names no kind at all — none of the three would read as anything but the ring they already are.
    focusTone: box.namesKind === "tag" ? Theme.refTag : Theme.borderFocus
    placeholderText: box.mode === "branch" ? qsTr("Create branch here?")
                   : box.mode === "tag" ? qsTr("Create tag here?") : ""
    onTextEdited: box.typed(box.text)
    // Refused text stays in the box: Enter that does nothing is the answer, and the frame and its tooltip say why.
    onAccepted: {
        if (!box.refused)
            box.submitted(box.text)
    }
    Keys.onEscapePressed: box.cancelled()
    ToolTip.visible: box.refused && box.activeFocus && box.refusedWhy !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: box.refusedWhy

    /// Whether the box is on screen at all, and where it came out. Drawn outside the list, it has one more way to be
    /// missing than a box in a row has — a layer that cuts it, a row the list has scrolled away from — and neither is
    /// anything a picture answers (PG_AUTO_ACT=nav-branch-box / nav-rename-box).
    readonly property string cameOut:
        Math.round(box.x) + "," + Math.round(box.y)
        + " row=" + Math.round(box.rowTop) + " list=" + Math.round(box.rowsTop)
        + "+" + Math.round(box.rowsHeight) + " out=" + box.out

    // The box's floor, measured. Never drawn — it stands in for what the field lays out inside itself, which cannot be
    // measured before the box being measured for has a width (`GraphRowChips` measures its own the same way).
    Label {
        id: ink
        visible: false
        text: box.inkText
        font: box.font
    }
}
