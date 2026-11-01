import QtQuick
import QtQuick.Layouts
import platitude.ui

// One chapter of a settings screen: its heading word, the rule under it, and whatever the caller lays out below.
// The word is a field (`CardText`), or it would be a dead strip in a surface whose words are dragged over. The heading
// is a band heading's spelling (capitals, `fontWeightStrong`) in `textPrimary`, a `borderSubtle` rule right under it; a
// field's caption (`LabeledField`) has the same step and ink in `Font.Normal`, so weight, capitals and the rule tell a
// chapter from a field. Ranked under the group only by rule and inset (`SettingsGroup`).
//
// Inside a chapter, what a thing is goes first and what it is now goes after its box: a sentence placed after two
// boxes about both reads as about the second. So `Written as user.name and user.email.` / `Written as core.autocrlf
// in that repository. …` / `Runs git fetch --prune …` stand right under the heading, and `A commit made there now
// would be attributed to …`, the line endings' effective value and git's errors under the box. Where a value is stored
// is one sentence before the chapters (one per group in `Git`).
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
            weight: Theme.fontWeightStrong
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderSubtle
        }
    }
}
