import QtQuick
import QtQuick.Layouts
import platitude.ui

// One group of a settings category: the word that names how far its values reach, its rule, and the chapters below.
// Same heading step (規約 §タイポグラフィ「見出しは重みと色」) and the same capitals, `fontMd`, `fontWeightStrong` and
// `textPrimary` as `SettingsSection` (user decision — ask before changing): this screen is explanation all around, so a
// heading dropped to `textSecondary` would have nothing quieter left beside it. Only two things rank it above the
// chapters: the `borderDefault` rule (theirs is `borderSubtle`) and the `spaceLg` inset of what it holds, so its rule
// reaches further left than theirs.
ColumnLayout {
    id: group

    /// The word above, written in the case it is shown in.
    property string caption: ""

    /// What the caller lays out below the rule, inset by the group itself (`body`'s margin): left to each caller, the
    /// inset would be forgotten somewhere.
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
            weight: Theme.fontWeightStrong
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
