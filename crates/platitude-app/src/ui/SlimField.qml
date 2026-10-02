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
    implicitHeight: Theme.iconLg
    font.pixelSize: Theme.fontMd
    // Every box in the app holds its words `spaceXs` off its frame on both sides — the pane's own inset, which the
    // message frames also stand their text on (デザイン規約 §余白).
    leftPadding: Theme.spaceXs
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

    // The product's right-click menu, not the style's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    FieldMenuSeat {
        id: menuSeat
        editor: slim
    }
}
