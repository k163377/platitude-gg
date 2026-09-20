import QtQuick
import QtQuick.Layouts
import platitude.ui

// One chapter of a settings screen: the word that names it, the rule under that word, and whatever the caller lays
// out below.
//
// **The word is a field** (`CardText`, 規約 §右のペインの字は掴める) — the same reason a caption is
// (`LabeledField`): a heading that took no press would be a dead strip inside a surface whose words are dragged over.
//
// Upper case, `fontMd`, `DemiBold`, `textPrimary` — the band heading's spelling with the ink of a heading
// (規約 §設定の画面). A sidebar band wears `textSecondary` among rows that are `textPrimary`, so it is the quiet
// thing in a loud column; here the whole screen around it is captions and explanations, and a heading that takes
// the same step down has nothing left to be quieter than — the skeleton reads as the faintest thing on the screen
// (observed). What ranks it under the group above is the rule and the inset (`SettingsGroup`). The chapter is
// heavier than its fields and it carries a rule.
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
