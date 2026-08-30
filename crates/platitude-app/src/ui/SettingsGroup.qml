import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One group of a settings category: the word that names how far its values reach, the rule under that word, and the
// chapters the caller lays out below (規約 §設定の画面).
//
// The level above `SettingsSection`, and it is ranked without changing the step: both are `fontMd` + `DemiBold` +
// upper case + `textPrimary`, because a heading is made by weight, colour and case rather than by size
// (規約 §タイポグラフィ 「見出しを段で作らない」) and both of these are headings. What ranks them is the rule and
// the inset — the group's is `borderDefault` and the chapter's `borderSubtle`, and the chapters are inset, so the
// group's rule runs past theirs on the left and the nesting is legible without reading a word of it. **Not the
// ink**: a screen whose skeleton was one step down read as the faintest thing on itself (`SettingsSection`).
ColumnLayout {
    id: group

    /// The word above, written in the case it is shown in.
    property string caption: ""

    /// What the caller lays out below the rule. Declared here rather than taken as the default property, because the
    /// inset is the group's to apply and a caller cannot be trusted to repeat it.
    default property alias content: body.data

    Layout.fillWidth: true
    spacing: Theme.spaceMd

    // The word and its rule are one thing, so they stand closer to each other than to what they head — the same
    // shape `SettingsSection` keeps.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        Label {
            text: group.caption
            color: Theme.textPrimary
            font.pixelSize: Theme.fontMd
            font.weight: Font.DemiBold
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
