import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The stash options, open right under the button that asks for them, as
// a mode of the pane rather than a window over it (デザイン規約 §可否・
// 警告の出し場所: ダイアログでなければ成立しない UI ではない). A stash
// destroys nothing, so the card chooses what goes; it never asks whether
// it may.
Rectangle {
    id: card

    required property var repoTab
    required property var workTree

    /// Whether the card is out. Its own bool rather than a read of
    /// `visible`, which is stale when read from another file
    /// (rules-refs/app-ui.md の RefusalBadge の項).
    property bool shown: false

    /// The Stash button: message / include untracked / keep index /
    /// staged only.
    signal submitted(string message, bool untracked, bool keepIndex, bool stagedOnly)

    function open() {
        stashName.text = ""
        stashUntracked.checked = true
        stashKeepIndex.checked = false
        stashStagedOnly.checked = false
        card.shown = true
        stashName.forceActiveFocus()
    }
    function close() {
        card.shown = false
    }
    /// The click path onto "Only the staged changes" (the page's smoke
    /// hook): setting `checked` skips `toggled`, a click does not.
    function clickStagedOnly() {
        stashStagedOnly.toggle()
        stashStagedOnly.toggled()
    }
    function apply() {
        // What is sent is what the boxes show — a box ruled out by the
        // staged-only choice does not smuggle its old tick through
        // (git refuses --staged together with --include-untracked).
        const stagedOnly = stashStagedOnly.checked
        card.submitted(stashName.text, !stagedOnly && stashUntracked.checked, !stagedOnly && stashKeepIndex.checked,
                       stagedOnly)
        card.close()
    }

    visible: card.shown
    implicitHeight: stashCol.implicitHeight + 2 * Theme.spaceSm
    color: Theme.bgBase
    radius: Theme.radiusMd
    border.color: Theme.borderDefault
    border.width: Theme.borderWidth
    ColumnLayout {
        id: stashCol
        anchors.fill: parent
        anchors.margins: Theme.spaceSm
        spacing: Theme.spaceXs
        FormField {
            id: stashName
            Layout.fillWidth: true
            placeholderText: qsTr("What this is, for finding it later")
            onAccepted: card.apply()
            Keys.onEscapePressed: card.close()
        }
        CheckBox {
            id: stashUntracked
            // A stash that leaves new files behind is the surprise
            // most often reported to other git GUIs, so this starts
            // on — the same choice the switch dialog makes.
            text: qsTr("Include files git is not tracking yet")
            font.pixelSize: Theme.fontMd
            implicitHeight: Theme.controlHeight
            enabled: !stashStagedOnly.checked
        }
        CheckBox {
            id: stashKeepIndex
            text: qsTr("Leave the staged changes staged")
            font.pixelSize: Theme.fontMd
            implicitHeight: Theme.controlHeight
            enabled: !stashStagedOnly.checked
        }
        CheckBox {
            id: stashStagedOnly
            text: qsTr("Only the staged changes")
            font.pixelSize: Theme.fontMd
            implicitHeight: Theme.controlHeight
            // git refuses it together with untracked files, and
            // cannot do it at all for a file changed on both sides:
            // it writes the entry, then fails to clear the tree and
            // leaves the entry behind with nothing else done
            // (measured). Refusing first is the only way that does
            // not surprise.
            enabled: card.workTree.partiallyStagedCount === 0
            // A disabled box keeps its tick, and what is sent is
            // what the boxes show — so the boxes this tick rules
            // out are unticked, not just greyed with their ticks
            // still live.
            onToggled: {
                if (checked) {
                    stashUntracked.checked = false
                    stashKeepIndex.checked = false
                }
            }
        }
        Label {
            Layout.fillWidth: true
            visible: card.workTree.partiallyStagedCount > 0
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            text: qsTr("%n file(s) are changed both in the staging area and on disk. git cannot take those apart, so the staged changes cannot go on their own.", "",
                       card.workTree.partiallyStagedCount)
        }
        // The card is a mode of the pane rather than a window, but its
        // foot is a dialog's: the way out on the left, the thing it is
        // for on the right, and the accent following what that one can do
        // (`DialogActions`).
        DialogActions {
            cancelText: qsTr("Cancel")
            acceptKind: "stash"
            acceptText: qsTr("Stash")
            acceptEnabled: card.repoTab.busyCount === 0
            onCancelled: card.close()
            onAccepted: card.apply()
        }
    }
}
