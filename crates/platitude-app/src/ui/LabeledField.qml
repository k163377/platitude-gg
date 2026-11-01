import QtQuick
import QtQuick.Layouts
import platitude.ui

// A caption above whatever a caller declares under it — the settings screen's fields and the clone / remote dialogs'.
// The caption is a `CardText` field: one taking no press is a dead strip on a surface whose words can be dragged over.
// In `textPrimary` (user decision — ask before changing; it reaches the dialogs too): on the settings screen the quiet
// side is the explanation (`HelpText`), and a caption in the note's ink would leave nothing to tell the two apart. Same
// step as a chapter heading in `Font.Normal`, no capitals and no rule (`SettingsSection`). A chapter whose heading
// already names its one box needs no caption (`LINE ENDINGS` / `MERGE EDITOR` / `GIT EXECUTABLE`). The recorded rule
// gives a caption only to a chapter of several boxes (the identity's `Name` / `Email address`), while the settings
// screen also captions single boxes its heading does not name (`Interval`, `At once`, `Initial commits`) — open
// (internal-docs/文書とコードの食い違い.md).
ColumnLayout {
    id: field

    property string caption: ""

    Layout.fillWidth: true
    spacing: Theme.spaceXs

    CardText {
        text: field.caption
        color: Theme.textPrimary
        pixelSize: Theme.fontMd
    }
}
