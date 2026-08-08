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
    /// Names to offer, packed the way the graph's label records are.
    readonly property var toolChoices: {
        const packed = settingsDialog.curPage
                       ? settingsDialog.curPage.pageTab.mergeTools : ""
        return packed === "" ? [] : packed.split(String.fromCharCode(31))
    }
    /// Stops a late answer from overwriting something already typed.
    property bool toolTouched: false

    /// Smoke hook. The candidates arrive in two waves and the configured
    /// name in a third, so the value has three chances to be knocked out
    /// by something that is not a person — report it at each.
    function reportTool() {
        if (AppBackend.autoAct === "settings-tools")
            AppBackend.report("merge_editor wanted=" + toolField.wanted
                              + " shown=" + toolField.editText
                              + " configured=" + settingsDialog.mergeTool)
    }
    onToolChoicesChanged: settingsDialog.reportTool()

    onOpened: {
        fetchField.text = AppBackend.autoFetchMinutes > 0
                          ? String(AppBackend.autoFetchMinutes) : ""
        toolField.wanted = settingsDialog.mergeTool
        settingsDialog.toolTouched = false
        if (settingsDialog.curPage) {
            // The status refresh only names the configured tool where
            // something is conflicted, so ask for it. The candidates are
            // a separate, far slower read — hence the turning indicator.
            settingsDialog.curPage.pageTab.askMergeTool()
            settingsDialog.curPage.pageTab.askMergeTools()
        }
        fetchField.forceActiveFocus()
    }
    onMergeToolChanged: {
        if (settingsDialog.opened && !settingsDialog.toolTouched) {
            toolField.wanted = settingsDialog.mergeTool
            settingsDialog.toolTouched = false
        }
        settingsDialog.reportTool()
    }
    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    function apply() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0
                                                              : Number(fetchField.text))
        if (settingsDialog.toolTouched && settingsDialog.curPage)
            settingsDialog.curPage.pageTab.setMergeTool(toolField.wanted)
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
                // As wide as the dialog gives it: a tool's name has no
                // fixed length, and an input only takes a fixed width
                // when its content does (デザイン規約 §レイアウト初期値).
                AppCombo {
                    id: toolField
                    Layout.fillWidth: true
                    placeholder: qsTr("none")
                    // Smoke hook: the popup is drawn here rather than by
                    // Fusion, so it needs its own look at (PG_AUTO_ACT).
                    Timer {
                        running: settingsDialog.opened
                                 && AppBackend.autoAct === "settings-tools"
                        interval: 400
                        onTriggered: toolField.popup.open()
                    }
                    loading: settingsDialog.curPage
                             && settingsDialog.curPage.pageTab.mergeToolsLoading
                    model: settingsDialog.toolChoices
                    onWantedChanged: settingsDialog.toolTouched = true
                    onAccepted: settingsDialog.apply()
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                // Named rather than picked from a list: the only way to
                // enumerate them is `git mergetool --tool-help`, whose
                // output is laid out for a person to read.
                text: qsTr("Which tool opens a conflicted file. It must not "
                           + "need a console — this app gives git none, so "
                           + "vimdiff and its kind cannot run.")
            }
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            // Two settings, two homes: one this app keeps for itself, one
            // git keeps where every other git on this computer can see it.
            text: qsTr("The fetch interval is stored by Platitude GG. The "
                       + "merge editor is stored by git as merge.guitool, "
                       + "where every other git on this computer sees it.")
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                text: qsTr("Cancel")
                onClicked: settingsDialog.close()
            }
            HoverButton {
                highlighted: true
                text: qsTr("Save")
                onClicked: settingsDialog.apply()
            }
        }
    }
}
