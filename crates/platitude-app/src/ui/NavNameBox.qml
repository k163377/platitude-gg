import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The box a sidebar row's name goes into — new branch, new tag, new branch in a worktree of its own or rename, told
// apart by the mode (`SidebarRowGestures`). Drawn outside the list so it can grow past the pane over the graph, head
// held where the name's was: a box too narrow for its text asks nothing (デザイン規約 §グラフ行のダブルクリック「箱のプレースホルダは全文」).
SlimField {
    id: box

    /// The row this box stands on, and the seat in it that sets the box's head and least width.
    required property Item row
    required property Item seat
    /// Where the box is drawn (`NavList.boxLayer`); null keeps it in its seat, for a floating panel.
    property Item drawnIn: null
    /// The list's rows in that layer, and the list's own top and height — so the box scrolls with its row and
    /// leaves with it (`NavList`).
    property real rowsX: 0
    property real rowsY: 0
    property real rowsTop: 0
    property real rowsHeight: 0
    /// Whether this row is being typed into, for what ("branch" / "tag" / "worktree" / "rename"), and with what — held
    /// by the sidebar, since the delegate is recycled.
    property bool editing: false
    property string mode: ""
    property string carried: ""
    property string refusedWhy: ""
    /// A worktree's folder in `refusedWhy`, empty for none: the shared tip stands the tree mark in front of it
    /// (`SharedToolTip.tipMarkWord`, デザイン規約 §ref の種別「名前の印」).
    property string refusedMark: ""
    readonly property string tipMarkWord: box.refusedMark
    /// The kind of ref being named — the mode, or the row's kind on a rename — which the frame says
    /// (`SlimField.focusTone` — §ref の種別: 枠 = 種別).
    property string namesKind: ""

    /// `submitted`, not `accepted`: that is the field's own Enter signal.
    signal typed(string text)
    signal submitted(string text)
    signal cancelled()

    readonly property real rowTop: box.rowsY + box.row.y
    readonly property bool rowInView: box.rowTop >= box.rowsTop
        && box.rowTop + box.row.height <= box.rowsTop + box.rowsHeight
    /// What the box must show, latched when it opens: a right edge that walks under the caret cannot be aimed at
    /// (rules/app-ui.md「測って押し出す値は…」).
    property string inkText: ""
    readonly property real wantWidth:
        Math.ceil(ink.implicitWidth) + box.leftPadding + box.rightPadding

    // Filled here on open and on rebuild, not from the row's handler: that can run before this `editing` catches up,
    // leaving the box empty and without the keyboard.
    onEditingChanged: box.takeFocus()
    Component.onCompleted: box.takeFocus()
    function takeFocus() {
        if (!box.editing)
            return
        box.text = box.carried
        box.inkText = box.carried === "" ? box.placeholderText : box.carried
        box.selectAll()
        box.forceActiveFocus()
    }

    /// Whether the box is drawn out in the layer — whenever there is one, not only while editing: a reparent drops
    /// the keyboard.
    readonly property bool out: box.drawnIn !== null
    parent: box.out ? box.drawnIn : box.seat
    visible: box.editing && (!box.out || box.rowInView)
    x: box.out ? box.rowsX + box.seat.x : 0
    y: (box.out ? box.rowsY + box.row.y : 0) + (box.row.height - box.height) / 2
    // At least the seat, at most the window's edge less the cards' cap (デザイン規約 §メニュー). `x` is a window
    // coordinate: the left menu begins at the window's left edge.
    width: !box.out ? box.seat.width
         : Math.max(box.seat.width,
                    Math.min(box.wantWidth, box.Window.width - box.x - Theme.spaceXxl))
    font.pixelSize: Theme.fontMd
    // A tag has a colour to say, and so does a branch made out in another worktree (that worktree's green, as the
    // graph's box wears it — `GraphRowChips`); the others would read as the plain focus ring anyway.
    focusTone: box.namesKind === "tag" ? Theme.refTag : box.namesKind === "worktree" ? Theme.success : Theme.borderFocus
    placeholderText: box.mode === "branch" ? qsTr("Create branch here?")
                   : box.mode === "tag" ? qsTr("Create tag here?")
                   : box.mode === "worktree" ? qsTr("Create worktree here?") : ""
    onTextEdited: box.typed(box.text)
    // Refused: Enter does nothing, and the frame and tooltip say why.
    onAccepted: {
        if (!box.refused)
            box.submitted(box.text)
    }
    Keys.onEscapePressed: box.cancelled()
    // Why Enter did nothing (規約 §hover のツールチップ). Asks `visible`, not focus: the reparent's keyboard hand-off
    // lands inside this binding and Qt drops it as a loop
    // (rules-refs/app-ui.md「添付 `ToolTip.visible` の式が読むのは `visible`」).
    ToolTip.visible: box.refused && box.visible && box.refusedWhy !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: box.refusedWhy
    /// That tip answers what is typed, not a hand: a held scroll bar leaves it standing (`SharedToolTip.tipTyped`) —
    /// in the folded rail's peek the box stays open while the peek's own bar is used to look through the names.
    readonly property bool tipTyped: true

    /// Where the box came out (PGG_AUTO_ACT=nav-branch-box / nav-rename-box): a clipped or scrolled-away box is
    /// nothing a picture answers.
    readonly property string cameOut:
        Math.round(box.x) + "," + Math.round(box.y)
        + " row=" + Math.round(box.rowTop) + " list=" + Math.round(box.rowsTop)
        + "+" + Math.round(box.rowsHeight) + " out=" + box.out

    // Measures `inkText` apart from the field, whose own layout needs a width first.
    Label {
        id: ink
        visible: false
        text: box.inkText
        font: box.font
    }
}
