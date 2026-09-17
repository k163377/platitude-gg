import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Slim single-line input shared by the toolbar search and the in-place
// name boxes: thin frame, compact height.
TextField {
    id: slim
    /// What is typed cannot be accepted (§可否・警告の出し場所: the frame
    /// says so where the typing is, and the reason waits in the tooltip).
    property bool refused: false
    /// The frame's colour while the box has the keyboard. The plain focus
    /// ring for a box that is only a box (the search band, a URL); the
    /// **kind's own colour** for one that is naming a ref, which is the
    /// rule the chip it becomes already follows (§ref の種別: 枠 = 種別).
    /// A refusal outranks it — that is a different axis, and the one the
    /// reader has to answer before anything else.
    property color focusTone: Theme.borderFocus
    /// How wide this box would have to be for what is in it to stand without scrolling — what is typed plus both
    /// insets. What a layout asks where this box is the one that may take the room it needs (`PublishForm`); a box
    /// narrower than this scrolls, so the text keeps all of itself.
    readonly property real wantedWidth: slim.contentWidth + slim.leftPadding + slim.rightPadding
    implicitHeight: Theme.iconLg
    font.pixelSize: Theme.fontMd
    // The one value every box in this application keeps between its frame and its words — the pane's own inset, which
    // is what the message frames already stand their text on (デザイン規約 §余白). The same on both sides: a box is
    // read the same way wherever it stands, so the frame around a name in the graph and the frame around a description
    // in the panel hold their text at the same distance. **`spaceXs`** — `spaceSm` was never chosen, only inherited
    // when these primitives were lifted out of `Main.qml`, and it puts the app's smallest boxes on a wider step than
    // the pane they sit in.
    leftPadding: Theme.spaceXs
    rightPadding: Theme.spaceXs
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: slim.refused ? Theme.warning : slim.activeFocus ? slim.focusTone : Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
