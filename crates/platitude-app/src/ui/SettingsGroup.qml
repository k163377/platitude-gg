import QtQuick
import QtQuick.Layouts
import platitude.ui

// One group of a settings category: the word that names how far its values reach, its rule, and the chapters below.
// Same heading step and ink as `SettingsSection`; only the `borderDefault` rule and the chapters' inset rank it above
// them (規約 §設定の画面).
ColumnLayout {
    id: group

    /// The word above, written in the case it is shown in.
    property string caption: ""

    /// What the caller lays out below the rule, inset by the group itself.
    default property alias content: body.data

    Layout.fillWidth: true
    spacing: Theme.spaceMd

    // The word and its rule stand closer to each other than to what they head.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        CardText {
            text: group.caption
            color: Theme.textPrimary
            pixelSize: Theme.fontMd
            weight: Font.DemiBold
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }
    }

    ColumnLayout {
        id: body
        Layout.fillWidth: true
        Layout.leftMargin: Theme.spaceLg
        spacing: Theme.spaceXl
    }
}
