import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One chapter of a settings screen: the word that names it, the rule under that word, and whatever the caller lays
// out below.
//
// The word is spelled the way every other band heading in this app is (規約 §タイポグラフィ 帯の見出し) —
// upper case, `fontMd`, `DemiBold`, `textSecondary` — so a chapter here reads as the same kind of thing as
// BRANCHES does in the sidebar. That spelling is also what keeps it apart from the captions inside it
// (`LabeledField`: the same step and colour at `Font.Normal`). The chapter is not louder, it is heavier, and the
// rule under the word is what says where one chapter ends and the next begins.
ColumnLayout {
    id: section

    /// The word above, written in the case it is shown in.
    property string caption: ""

    Layout.fillWidth: true
    spacing: Theme.spaceMd

    // The word and its rule are one thing, so they stand closer to each other than to what they head.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        Label {
            text: section.caption
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMd
            font.weight: Font.DemiBold
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderSubtle
        }
    }
}
