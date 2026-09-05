import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The settings screen's top band: what the screen is, what it is showing, and the way out of it (規約 §設定の画面).
//
// **Two lanes, not one block**: the title is laid on the block the chapters are, so it stands over the rail; the exit
// is hung off the band's own right edge instead, because a hand aiming at it aims at the corner of the screen
// (規約 §設定の画面). That is what the plain `Item` is for — a RowLayout could only place the two in one run.
Item {
    id: head

    /// How wide the screen's one block is (`SettingsDialog.blockWidth`). Read to find where that block's left edge
    /// falls, so the title starts level with the rail below it; the band itself is always the full width.
    required property real blockWidth
    /// What the screen is showing, for the second half of the title. Read from the one list the rail is laid out
    /// from, so the two can never come to call a category by different names (`SettingsDialog.categories`).
    required property string word
    /// The mark for that same category, off that same list — the rail's row and the title wear the one mark.
    required property string categoryIcon
    /// Something on the screen is holding an edit git has not been given, so the way out costs something.
    required property bool unsaved
    /// A press was made over that and turned down: the next one goes through.
    required property bool armed

    /// The `✕`, or Escape, or anything else that means "leave".
    signal closed()

    Layout.fillWidth: true
    // The band's own height, since nothing in it fills: the exit's seat with a step of air either side of it.
    implicitHeight: Theme.toolbarHeight + 2 * Theme.spaceLg

    RowLayout {
        // The block's left edge, plus the lane the rail keeps inside it. **Written on what stands in the lane**
        // rather than as a margin on the band: a margin would sit outside the width being centred, and the title
        // would land a few pixels off the column it lines up with (measured).
        anchors.left: parent.left
        anchors.leftMargin: Math.max(0, (head.width - head.blockWidth) / 2) + Theme.spaceXxl
        anchors.right: closeMark.left
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceSm

        NavIcon {
            Layout.preferredWidth: Theme.iconLg
            Layout.preferredHeight: Theme.iconLg
            kind: "gear"
            tint: Theme.textPrimary
        }
        Label {
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        // What the screen is showing, beside what the screen is. **The same step, told apart by weight and ink** —
        // it is the second half of one title rather than a subtitle, so dropping it a step would break the phrase
        // (規約 §タイポグラフィ 「見出しを段で作らない」). The rule between them is the one every divider in this
        // window is drawn in.
        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconLg
            color: Theme.borderDefault
        }
        NavIcon {
            Layout.preferredWidth: Theme.iconLg
            Layout.preferredHeight: Theme.iconLg
            kind: head.categoryIcon
            tint: Theme.textSecondary
        }
        Label {
            text: head.word
            font.pixelSize: Theme.fontXl
            color: Theme.textSecondary
        }
        Item { Layout.fillWidth: true }
    }
    // **The mark alone, and the key in its one line.** No drawn `Esc` beside it: the products this screen was checked
    // against draw only the `✕`, and a legend for a key everyone already reaches for is furniture in the corner of
    // every reading (規約 §設定の画面).
    CloseToolButton {
        id: closeMark
        // Hard against the band's right edge, with no margin of its own — the air around the mark is the seat's, so
        // what the hand meets in the screen's corner is the target rather than the gap beside it (規約 §当たり判定).
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        // **A square of `toolbarHeight`, which covers the window button's `railWidth` cell** (規約 §当たり判定 — the
        // seat grows, the paint does not). The mark grows too, because what it closes is the whole screen: at
        // `iconSm` it reads as an afterthought beside a `fontXl` word (observed).
        seat: Theme.toolbarHeight
        markSize: Theme.iconMd
        // **It carries the warning, and it is never disabled** — a dead `✕` says "no way out" and gives no reason.
        // Ordinary ink when there is nothing to lose, `warning` while something is unsaved, and armed it changes
        // shape as well as colour, because a press that appeared to do nothing needs more than a hue to answer it.
        tone: head.armed || !head.unsaved ? Theme.textPrimary : Theme.warning
        armed: head.armed
        Accessible.name: qsTr("Close the settings")
        tip: head.armed ? qsTr("Press again to close without saving")
           : head.unsaved ? qsTr("An identity here has not been given to git")
           : qsTr("Escape closes this too")
        onClicked: head.closed()
    }
}
