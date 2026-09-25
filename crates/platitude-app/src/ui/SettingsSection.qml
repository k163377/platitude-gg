import QtQuick
import QtQuick.Layouts
import platitude.ui

// One chapter of a settings screen: its heading word, the rule under it, and whatever the caller lays out below.
// The word is a field (`CardText`), or it would be a dead strip in a surface whose words are dragged over. Its step
// and `textPrimary` ink: 規約 §設定の画面. Ranked under the group only by rule and inset (`SettingsGroup`).
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
        CardText {
            text: section.caption
            color: Theme.textPrimary
            pixelSize: Theme.fontMd
            weight: Font.DemiBold
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderSubtle
        }
    }
}
