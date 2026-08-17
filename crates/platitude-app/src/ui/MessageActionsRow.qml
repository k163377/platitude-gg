import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details editor's action row. Nothing until something is actually changed — until then the pane keeps its resting
// shape and nothing invites a rewrite. While the leaving question stands the row turns into it: the question belongs to
// the text, so it is asked where the text is.
ColumnLayout {
    id: actions

    /// The boxes differ from the commit's own message.
    property bool dirty: false
    /// The leaving question is standing.
    property bool asking: false
    /// Saving replays instead of amending (not HEAD's own commit).
    property bool replays: false
    /// A remote already has this commit.
    property bool published: false
    /// A write is already running, so nothing new starts.
    property bool busy: false
    /// The summary box holds something to save.
    property bool canSave: false

    signal revertRequested()
    signal saveRequested()
    /// The leaving question was answered: true drops the edits and lets the move through, false stays on this commit.
    signal leaveResolved(bool discard)

    visible: actions.dirty
    spacing: Theme.spaceXs

    // The newest commit is amended in place and costs nothing; an older one is replayed, and everything built on it
    // comes back as different commits. Only the second case is worth a line.
    Label {
        Layout.fillWidth: true
        visible: !actions.asking && actions.replays
        wrapMode: Text.Wrap
        text: qsTr("Saving replays this commit, so every commit after it gets a new identity.")
        color: Theme.textSecondary
        font.pixelSize: Theme.fontSm
    }
    // Said, not asked, like the amend editor's tag: the save still goes ahead, and this line is the warning it gets.
    Label {
        Layout.fillWidth: true
        visible: !actions.asking && actions.published
        wrapMode: Text.Wrap
        text: qsTr("This commit is on a remote. Rewriting it leaves anyone who already has it out of step.")
        color: Theme.warning
        font.pixelSize: Theme.fontSm
    }
    Label {
        Layout.fillWidth: true
        visible: actions.asking
        wrapMode: Text.Wrap
        text: qsTr("Moving to another commit leaves this text behind.")
        color: Theme.warning
        font.pixelSize: Theme.fontSm
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceSm
        Item { Layout.fillWidth: true }
        HoverToolButton {
            visible: !actions.asking
            text: qsTr("Cancel")
            font.pixelSize: Theme.fontSm
            onClicked: actions.revertRequested()
        }
        ActionButton {
            visible: !actions.asking
            implicitHeight: Theme.controlHeight
            kind: "check"
            besideWord: true
            frameColor: enabled ? Theme.accent : Theme.borderDefault
            activeFocusOnTab: true
            text: qsTr("Save message")
            enabled: !actions.busy && actions.canSave
            onActivated: actions.saveRequested()
        }
        // The two ways out of the question. Staying is the framed one: it is the answer that loses nothing.
        HoverToolButton {
            visible: actions.asking
            text: qsTr("Discard edits")
            font.pixelSize: Theme.fontSm
            onClicked: actions.leaveResolved(true)
        }
        ActionButton {
            visible: actions.asking
            implicitHeight: Theme.controlHeight
            kind: "pen"
            besideWord: true
            frameColor: Theme.accent
            activeFocusOnTab: true
            text: qsTr("Keep editing")
            onActivated: actions.leaveResolved(false)
        }
    }
}
