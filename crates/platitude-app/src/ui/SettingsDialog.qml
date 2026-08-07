import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Settings that apply to every open repository: how often this computer
// should talk to remotes at all, and which editor it hands a conflict to.
AppDialog {
    id: settingsDialog

    /// The tab whose repository answers for the merge tool. The setting is
    /// global, but "what would git launch" is read where the person is.
    property var curPage: null
    readonly property string mergeTool:
        settingsDialog.curPage ? settingsDialog.curPage.pageWt.mergeTool : ""
    /// Stops a late answer from overwriting something already typed.
    property bool toolTouched: false

    onOpened: {
        fetchField.text = AppBackend.autoFetchMinutes > 0
                          ? String(AppBackend.autoFetchMinutes) : ""
        settingsDialog.toolTouched = false
        toolField.text = settingsDialog.mergeTool
        // The status refresh only names the tool where something is
        // conflicted, so ask for it — the answer lands a beat later.
        if (settingsDialog.curPage)
            settingsDialog.curPage.pageTab.askMergeTool()
        fetchField.forceActiveFocus()
    }
    onMergeToolChanged: {
        if (settingsDialog.opened && !settingsDialog.toolTouched)
            toolField.text = settingsDialog.mergeTool
    }
    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    function apply() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0
                                                              : Number(fetchField.text))
        if (settingsDialog.toolTouched && settingsDialog.curPage)
            settingsDialog.curPage.pageTab.setMergeTool(toolField.text)
        settingsDialog.close()
    }
    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Fetch automatically")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: fetchField
                    implicitWidth: 160
                    placeholderText: qsTr("off")
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: 1
                        top: AppBackend.autoFetchMaxMinutes
                    }
                    onAccepted: settingsDialog.apply()
                }
                Label {
                    text: qsTr("minutes")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Runs git fetch --prune on every open repository, at "
                           + "most once per interval. Leave it empty to switch it "
                           + "off; %1 minutes is the longest interval offered.")
                      .arg(AppBackend.autoFetchMaxMinutes)
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Merge editor")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: toolField
                    implicitWidth: 160
                    placeholderText: qsTr("none")
                    onTextEdited: settingsDialog.toolTouched = true
                    onAccepted: settingsDialog.apply()
                }
                Item { Layout.fillWidth: true }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                // Named rather than picked from a list: the only way to
                // enumerate them is `git mergetool --tool-help`, whose
                // output is laid out for a person to read.
                text: qsTr("Which tool opens a conflicted file. It has to be "
                           + "a windowed one — this app gives git no console, "
                           + "so vimdiff and its kind cannot run.")
            }
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            // Two settings, two homes: one this app forgets on exit, one
            // git keeps and every other git on this computer can see.
            text: qsTr("The fetch interval is not stored yet, so it returns to "
                       + "its default the next time platitude-gg starts. The "
                       + "merge editor is stored by git as merge.guitool.")
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                implicitHeight: Theme.controlHeight
                text: qsTr("Cancel")
                onClicked: settingsDialog.close()
            }
            HoverButton {
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: qsTr("Save")
                onClicked: settingsDialog.apply()
            }
        }
    }
}
