import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Puts the working tree away as one stash entry. Nothing is destroyed —
// the entry keeps every change — so the dialog is here to choose what
// goes, not to ask whether it may.
AppDialog {
    id: stashDialog

    /// Files changed on both sides at once (status `MM`). git cannot pull
    /// the index half of those out on its own, so "only the staged
    /// changes" is withheld while any exist.
    property int partiallyStaged: 0
    property bool busy: false

    /// message / include untracked / keep index / staged only.
    /// Named apart from Dialog's own `accepted`, which a signal of the
    /// same name would be an invalid override of.
    signal submitted(string message, bool untracked, bool keepIndex, bool stagedOnly)

    onOpened: {
        messageField.text = ""
        untrackedBox.checked = true
        keepIndexBox.checked = false
        stagedOnlyBox.checked = false
        messageField.forceActiveFocus()
    }
    /// The click path onto "Only the staged changes" (the page's smoke
    /// hook): setting `checked` skips `toggled`, a click does not.
    function clickStagedOnly() {
        stagedOnlyBox.toggle()
        stagedOnlyBox.toggled()
    }
    function apply() {
        // What is sent is what the boxes show — a box ruled out by the
        // staged-only choice does not smuggle its old tick through
        // (git refuses --staged together with --include-untracked).
        const stagedOnly = stagedOnlyBox.checked
        stashDialog.submitted(messageField.text,
                              !stagedOnly && untrackedBox.checked,
                              !stagedOnly && keepIndexBox.checked,
                              stagedOnly)
        stashDialog.close()
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Stash changes")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Name")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            FormField {
                id: messageField
                Layout.fillWidth: true
                placeholderText: qsTr("What this is, for finding it later")
                onAccepted: stashDialog.apply()
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            CheckBox {
                id: untrackedBox
                // A stash that leaves new files behind is the surprise
                // most often reported to other git GUIs, so this starts on
                // — the same choice the switch dialog makes.
                text: qsTr("Include files git is not tracking yet")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                enabled: !stagedOnlyBox.checked
            }
            CheckBox {
                id: keepIndexBox
                text: qsTr("Leave the staged changes staged")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                enabled: !stagedOnlyBox.checked
            }
            CheckBox {
                id: stagedOnlyBox
                text: qsTr("Only the staged changes")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                // git refuses it together with untracked files, and cannot
                // do it at all for a file changed on both sides: it writes
                // the entry, then fails to clear the tree and leaves the
                // entry behind with nothing else done (measured). Refusing
                // first is the only way that does not surprise.
                enabled: stashDialog.partiallyStaged === 0
                // A disabled box keeps its tick, and what is sent is what
                // the boxes show — so the boxes the tick rules out are
                // unticked, not just greyed with their ticks still live
                // (git refuses --staged with --include-untracked).
                onToggled: {
                    if (checked) {
                        untrackedBox.checked = false
                        keepIndexBox.checked = false
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                visible: stashDialog.partiallyStaged > 0
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("%n file(s) are changed both in the staging area and "
                           + "on disk. git cannot take those apart, so the "
                           + "staged changes cannot go on their own.", "",
                           stashDialog.partiallyStaged)
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                implicitHeight: Theme.controlHeight
                text: qsTr("Cancel")
                onClicked: stashDialog.close()
            }
            HoverButton {
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: qsTr("Stash")
                enabled: !stashDialog.busy
                onClicked: stashDialog.apply()
            }
        }
    }
}
