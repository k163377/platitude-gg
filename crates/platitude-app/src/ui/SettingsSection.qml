import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One chapter of a settings screen: the word that names it, the rule under that word, and whatever the caller lays
// out below.
//
// Upper case, `fontMd`, `DemiBold`, `textPrimary` — the band heading's spelling with the ink of a heading rather
// than of a note (規約 §設定の画面). A sidebar band wears `textSecondary` among rows that are `textPrimary`, so it
// is the quiet thing in a loud column; here the whole screen around it is captions and explanations, and a heading
// that took the same step down had nothing left to be quieter than — the skeleton read as the faintest thing on the
// screen (2026-08-30 ユーザー報告). What ranks it under the group above is the rule and the inset, not the ink
// (`SettingsGroup`). The chapter is not louder than its fields, it is heavier and it carries a rule.
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
            color: Theme.textPrimary
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
