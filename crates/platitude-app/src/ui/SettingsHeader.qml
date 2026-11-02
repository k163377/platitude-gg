import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The settings screen's top band: the title, laid on the chapters' centred block, and the exit, hung off the band's
// right edge (`SettingsDialog.blockWidth`). A plain `Item`, because a RowLayout could only place the two in one run.
// The `✕`'s corner hit is pinned by `tst_settingsheader`. Its marks are `Canvas`es, which can miss the overlay mirror's
// grab: check them in `scene.png`, not `overlay.png`.
Item {
    id: head

    /// The screen's block width (`SettingsDialog.blockWidth`), so the title starts level with the rail.
    required property real blockWidth
    /// The category showing, for the second half of the title (`SettingsDialog.categories`).
    required property string word
    /// That category's mark, off the same list.
    required property string categoryIcon
    /// Something on the screen is holding an edit git has not been given, so the way out costs something.
    required property bool unsaved
    /// A press was made over that and turned down: the next one goes through.
    required property bool armed

    /// The `✕` was pressed.
    signal closed()

    Layout.fillWidth: true
    implicitHeight: Theme.toolbarHeight + 2 * Theme.spaceLg

    RowLayout {
        // The block's left edge plus the rail's lane, set here: a margin on the band would fall outside the centred
        // width and shift the title off the rail.
        anchors.left: parent.left
        anchors.leftMargin: Math.max(0, (head.width - head.blockWidth) / 2) + Theme.spaceXxl
        anchors.right: closeMark.left
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceSm

        // Each half's mark heads its words: a step over their capitals, its box their line so the two share a baseline
        // (デザイン規約 §アイコンの席 の文頭記号の見出し). A new mark is added as a `NavIcon` kind, as `gear` is.
        NavIcon {
            Layout.preferredWidth: Theme.iconLg
            Layout.preferredHeight: titleWord.implicitHeight
            width: Theme.iconLg
            height: titleWord.implicitHeight
            kind: "gear"
            set: "head"
            size: titleWord.font.pixelSize
            wordBase: titleWord.baselineOffset
            tint: Theme.textPrimary
        }
        Label {
            id: titleWord
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Theme.fontWeightStrong
        }
        // The category, beside the title: the same step, told apart by weight and ink — one title in two halves
        // (規約 §タイポグラフィ 「重み・色・大文字が作る」).
        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconLg
            color: Theme.borderDefault
        }
        NavIcon {
            Layout.preferredWidth: Theme.iconLg
            Layout.preferredHeight: categoryWord.implicitHeight
            width: Theme.iconLg
            height: categoryWord.implicitHeight
            kind: head.categoryIcon
            set: "head"
            size: categoryWord.font.pixelSize
            wordBase: categoryWord.baselineOffset
            tint: Theme.textSecondary
        }
        PlainTextLabel {
            id: categoryWord
            text: head.word
            font.pixelSize: Theme.fontXl
            color: Theme.textSecondary
        }
        Item { Layout.fillWidth: true }
    }
    // The one way out, the `✕` alone (user decision — ask before changing): a picture of the key everybody presses
    // would sit in the corner like furniture every time the screen is read. The key is said in its tooltip, in one
    // line (規約 §hover のツールチップ「ラベルに入らなかった 1 点」).
    CloseToolButton {
        id: closeMark
        // Flush with the band's right edge, all of its air inside the seat (規約 §QML 実装ルール「当たり判定は塗りより大きくてよい」).
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        // A `toolbarHeight` square seat, no smaller than a window button's `railWidth` × `toolbarHeight` hit. The mark
        // is the window buttons' `iconMd`: "a step under what it closes" is the rows' and tabs' `✕`; this one closes a
        // whole screen beside a `fontXl` title, where `iconSm` reads as an afterthought.
        seat: Theme.toolbarHeight
        markSize: Theme.iconMd
        // Warning ink while something is unsaved, yet always pressable — a dead `✕` says there is no way out without
        // saying why. Armed changes shape too (`CloseToolButton.armed`): colour alone is one signal, and a press that
        // did nothing needs a second.
        tone: head.armed || !head.unsaved ? Theme.textPrimary : Theme.warning
        armed: head.armed
        Accessible.name: qsTr("Close the settings")
        tip: head.armed ? qsTr("Press again to close without saving")
           : head.unsaved ? qsTr("An identity here has not been given to git")
           : qsTr("Escape closes this too")
        onClicked: head.closed()
    }
}
