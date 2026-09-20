import QtQuick
import QtQuick.Layouts
import platitude.ui

// One group of a settings category: the word that names how far its values reach, the rule under that word, and the
// chapters the caller lays out below (規約 §設定の画面).
//
// **The word is a field**, the same as the chapter heading under it (`SettingsSection`).
//
// The level above `SettingsSection`, ranked at the same step: both are `fontMd` + `DemiBold` + upper case +
// `textPrimary`, because a heading is made by weight, colour and case
// (規約 §タイポグラフィ 「重み・色・大文字が作る」) and both of these are headings. What ranks them is the rule and
// the inset — the group's is `borderDefault` and the chapter's `borderSubtle`, and the chapters are inset, so the
// group's rule runs past theirs on the left and the nesting is legible at a glance. **The ink is the same**:
// a screen whose skeleton was one step down read as the faintest thing on itself (`SettingsSection`).
ColumnLayout {
    id: group

    /// The word above, written in the case it is shown in.
    property string caption: ""

    /// What the caller lays out below the rule. Aliased to `body`: the inset is the group's to apply and a caller
    /// cannot be trusted to repeat it.
    default property alias content: body.data

    Layout.fillWidth: true
    spacing: Theme.spaceMd

    // The word and its rule are one thing, so they stand closer to each other than to what they head — the same
    // shape `SettingsSection` keeps.
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
