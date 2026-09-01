import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The settings screen's top band: what the screen is, what it is showing, and the way out of it (規約 §設定の画面).
//
// Laid in the same block the chapters are, so the title stands over the rail and the mark over the column it closes.
// **The block's lanes are written on what stands in them** rather than as margins on the band: a margin would sit
// outside the width being centred, and the band would land a few pixels off the body it lines up with (measured).
RowLayout {
    id: head

    /// What the screen is showing, for the second half of the title. Read from the one list the rail is laid out
    /// from, so the two can never come to call a category by different names (`SettingsDialog.categories`).
    required property string word
    /// Something on the screen is holding an edit git has not been given, so the way out costs something.
    required property bool unsaved
    /// A press was made over that and turned down: the next one goes through.
    required property bool armed

    /// The `✕`, or Escape, or anything else that means "leave".
    signal closed()

    Layout.fillWidth: true
    Layout.topMargin: Theme.spaceLg
    Layout.bottomMargin: Theme.spaceLg
    spacing: Theme.spaceSm

    Label {
        Layout.leftMargin: Theme.spaceXxl
        text: qsTr("Settings")
        font.pixelSize: Theme.fontXl
        font.weight: Font.DemiBold
    }
    // What the screen is showing, beside what the screen is (Apple HIG asks a settings window's title to name the
    // pane it is on). **The same step, told apart by weight and ink** — it is the second half of one title rather
    // than a subtitle, so dropping it a step would break the phrase
    // (規約 §タイポグラフィ 「見出しを段で作らない」). The rule between them is the one every divider in this
    // window is drawn in.
    Rectangle {
        Layout.alignment: Qt.AlignVCenter
        implicitWidth: Theme.borderWidth
        implicitHeight: Theme.iconLg
        color: Theme.borderDefault
    }
    Label {
        text: head.word
        font.pixelSize: Theme.fontXl
        color: Theme.textSecondary
    }
    Item { Layout.fillWidth: true }
    // **The mark alone, and the key in its one line.** No drawn `Esc` beside it: the products this screen was
    // checked against agree — IBM Carbon's modal lists the three ways out (the `✕` in the upper right, a click
    // outside, and Escape) and draws only the first. A legend for a key everyone already reaches for is furniture
    // in the corner of every reading.
    CloseToolButton {
        id: closeMark
        Layout.alignment: Qt.AlignVCenter
        // The block's lane with the air the mark holds inside its own seat taken off, so what stands a whole inset
        // from the block's edge is its ink rather than the box around it (デザイン規約 §余白) — which is also what
        // lines it up with the right edge of the column below.
        Layout.rightMargin: Theme.spaceXxl - closeMark.inkAir
        // A mark this small in a band this empty is a hard thing to hit, and at `iconSm` it reads as an
        // afterthought beside a `fontXl` word (observed). The seat grows into room that was doing
        // nothing (規約 §当たり判定); the mark grows because what it closes is the whole screen.
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
