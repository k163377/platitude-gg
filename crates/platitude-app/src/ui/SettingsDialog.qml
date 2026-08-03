import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One setting so far. It applies to every open repository, because the
// question is how often this computer should talk to remotes at all.
AppDialog {
    id: settingsDialog

    onOpened: {
        fetchField.text = AppBackend.autoFetchMinutes > 0
                          ? String(AppBackend.autoFetchMinutes) : ""
        fetchField.forceActiveFocus()
    }
    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    function apply() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0
                                                              : Number(fetchField.text))
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
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            text: qsTr("Settings are not stored yet, so this returns to its "
                       + "default the next time platitude-gg starts.")
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
