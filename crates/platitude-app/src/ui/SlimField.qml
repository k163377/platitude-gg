import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Slim single-line input shared by the toolbar search and the in-place name boxes: thin frame, compact height.
TextField {
    id: slim
    /// What is typed cannot be accepted: the frame says so, the reason waits in the tooltip
    /// (規約 §可否・警告の出し場所).
    property bool refused: false
    /// The frame's colour while the box has the keyboard: the focus ring, or the ref kind's own colour for a box naming
    /// a ref (規約 §ref の種別: 枠 = 種別). A refusal outranks it.
    property color focusTone: Theme.borderFocus
    /// The width at which what is typed stands without scrolling, insets included — for a layout that gives this box
    /// the room it needs (`PublishForm`).
    readonly property real wantedWidth: slim.contentWidth + slim.leftPadding + slim.rightPadding
    /// The kind of name the box takes, as a mark at its left edge that is no part of what is typed
    /// (`AppCombo.leadMark`, デザイン規約 §ref の種別「名前の印」). Empty for none.
    property string leadMark: ""
    /// The mark's seat's air on the frame's side, given back: the inset already parts the mark from the frame
    /// (`MarkSeat.airAt`, デザイン規約 §余白「枠の中の先頭に立つ印」).
    readonly property real frameAir: slim.leadMark !== "" ? MarkSeat.airAt(slim.font.pixelSize) : 0
    implicitHeight: Theme.iconLg
    font.pixelSize: Theme.fontMd
    // Every box in the app holds its words `spaceXs` off its frame on both sides — the pane's own inset, which the
    // message frames also stand their text on (デザイン規約 §余白) — and the kind's mark after it (`leadMark`).
    leftPadding: Theme.spaceXs + (slim.leadMark !== "" ? leadIcon.wordCell - slim.frameAir : 0)
    rightPadding: Theme.spaceXs
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: slim.refused ? Theme.warning
                    : slim.activeFocus || menuSeat.holding ? slim.focusTone : Theme.borderDefault
        border.width: Theme.borderWidth
    }

    // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    Keys.onMenuPressed: event => event.accepted = menuSeat.offer()
    // What is typed past the box scrolls to the caret; unclipped, its head is drawn out past the frame.
    clip: true
    FieldMenuSeat {
        id: menuSeat
        editor: slim
    }
    // The inset's own ground inside the frame, clear of its rounded corners, so the scrolled words' head passes under
    // the mark, not through it.
    Rectangle {
        visible: slim.leadMark !== ""
        x: Theme.borderWidth
        y: Theme.radiusSm
        width: slim.leftPadding - Theme.borderWidth
        height: slim.height - 2 * Theme.radiusSm
        color: Theme.bgBase
    }
    // In the inset, on the words' line: drawn, not typed.
    NavIcon {
        id: leadIcon
        visible: slim.leadMark !== ""
        x: Theme.spaceXs - slim.frameAir
        y: slim.topPadding
        width: leadIcon.wordCell
        height: slim.height - slim.topPadding - slim.bottomPadding
        kind: slim.leadMark !== "" ? slim.leadMark : "branch"
        set: "sign"
        size: slim.font.pixelSize
        wordBase: slim.baselineOffset - slim.topPadding
        nameAir: WordFace.nameAir(slim, slim.text)
        // Its kind's colour, as a name's mark is anywhere (`NavIcon.inkBeside`); grey with the box.
        tint: leadIcon.inkBeside(slim.enabled ? Theme.textPrimary : Theme.textMuted)
    }
}
