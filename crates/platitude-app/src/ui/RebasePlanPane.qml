import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The interactive-rebase plan, stood where the graph was: the rows of `from^..HEAD` newest-first, the verb each one
// carries, and the base they land on as the list's own last row. Composing runs nothing — the one door out of here
// that touches the repository is the run button standing at the right pane's foot, and Cancel up here only puts the
// draft away (デザイン規約 §履歴を合流させる / 提案 2026-08-30).
//
// While this pane stands, the surfaces that could move the history under it are held down by the page (the sidebar,
// find, the toolbar's writes); what cannot be held — another session, a terminal — is answered by the model putting
// the plan away when the tip moves (`RebasePlanModel.noteHead`), and by core pinning the tip again at run time.
Rectangle {
    id: planPane

    required property var planModel
    /// The commit the page's selection sits on, so the row under it reads selected the way a graph row would.
    property string selectedOid: ""

    /// Which commit the reader picked, for the page to select (the details pane follows it).
    signal rowPicked(string oidHex)

    color: Theme.bgBase

    // The widest verb decides the chip column, measured off a real chip so the resolved font is what measures it
    // (app-ui.md — a metrics call in a binding freezes on the default font; a Label's implicitWidth does not).
    CodeChip {
        id: verbProbe
        visible: false
        word: "reword"
        size: Theme.fontChip
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The head names the mode, not a command: nothing is running (§git 用語のコード表記 — 状態は通常表記).
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.toolbarHeight
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm
                Label {
                    text: planPane.planModel.root ? qsTr("Rebasing back to the very first commit")
                                                  : qsTr("Rebasing onto")
                    font.pixelSize: Theme.fontMd
                    color: Theme.textPrimary
                }
                // The base, its branch name first and the id only as the fallback (ユーザー判断 2026-08-30).
                Label {
                    visible: !planPane.planModel.root && planPane.planModel.ontoRef !== ""
                    text: planPane.planModel.ontoRef
                    font.pixelSize: Theme.fontMd
                    color: Theme.accentHover
                }
                Label {
                    visible: !planPane.planModel.root && planPane.planModel.ontoRef === ""
                    text: planPane.planModel.ontoOid.substring(0, 8)
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontCode
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
                ActionButton {
                    text: qsTr("Cancel")
                    frameColor: Theme.borderDefault
                    onActivated: planPane.planModel.cancelPlan()
                }
            }
            // The mode's own underline, where a standing bar draws its edge.
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.borderWidth
                color: Theme.accent
            }
        }

        AppListView {
            id: planList
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.topMargin: Theme.spaceXs
            model: planPane.planModel

            /// What every row reads and calls (the delegate reaches the pane through its view).
            readonly property string selectedOid: planPane.selectedOid
            readonly property real verbColW: verbProbe.wantWidth
            function rowPicked(row, oidHex) {
                planPane.planModel.selectRow(row)
                planPane.rowPicked(oidHex)
            }
            function moveRequested(from, to) {
                planPane.planModel.moveStep(from, to)
            }
            function verbMenuRequested(row) {
                verbMenu.openFor(row)
            }

            delegate: RebasePlanRow {}

            // The base the rows land on, as the row after the oldest one: what the squash arrows point at, and the
            // one row here nothing can be done to.
            footer: Item {
                width: planList.width
                height: planPane.planModel.root ? 0 : Theme.graphRowHeight + Theme.spaceSm
                visible: !planPane.planModel.root
                Rectangle {
                    id: footerRule
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.topMargin: Theme.spaceXs
                    height: Theme.borderWidth
                    color: Theme.borderSubtle
                }
                RowLayout {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: footerRule.bottom
                    height: Theme.graphRowHeight
                    anchors.leftMargin: Theme.spaceMd
                    anchors.rightMargin: Theme.spaceXs
                    spacing: Theme.spaceSm
                    // The base's name is said once, in the header's own sentence — this row is the commit itself,
                    // dressed like every other row: face, subject, id.
                    Label {
                        Layout.preferredWidth: planList.verbColW + Theme.iconMd
                        text: qsTr("onto")
                        font.pixelSize: Theme.fontSm
                        color: Theme.textMuted
                    }
                    IdentIcon {
                        Layout.preferredWidth: Theme.iconMd
                        Layout.preferredHeight: Theme.iconMd
                        Layout.alignment: Qt.AlignVCenter
                        code: planPane.planModel.ontoAvatar
                        imageUrl: planPane.planModel.ontoAvatarUrl
                    }
                    CutName {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        cutAt: "end"
                        text: planPane.planModel.ontoSubject
                        pixelSize: Theme.fontMd
                        color: Theme.textSecondary
                    }
                    Label {
                        Layout.alignment: Qt.AlignVCenter
                        text: planPane.planModel.ontoOid.substring(0, 8)
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontCode
                        color: Theme.textMuted
                    }
                }
            }
        }
    }

    // The verb menu: the same six words the todo file takes, chips first (デザイン規約 §git 用語のコード表記). What row
    // it is about — and whether a fold on it has anywhere to land — is frozen as it opens (app-ui.md §メニュー), and
    // the landing question is the model's own (`canFold`), so the menu cannot come to answer it differently from the
    // demotion that enforces it.
    Item {
        anchors.fill: parent
        AppMenu {
            id: verbMenu
            property int forRow: -1
            property bool foldLands: false
            function openFor(row) {
                verbMenu.forRow = row
                verbMenu.foldLands = planPane.planModel.canFold(row)
                verbMenu.offer()
            }
            AppMenuItem {
                code: "pick"
                text: qsTr("keep it as it is")
                onTriggered: planPane.planModel.setAction(verbMenu.forRow, "pick")
            }
            AppMenuItem {
                code: "reword"
                text: qsTr("change its message")
                onTriggered: planPane.planModel.setAction(verbMenu.forRow, "reword")
            }
            AppMenuItem {
                code: "edit"
                text: qsTr("stop there to amend")
                onTriggered: planPane.planModel.setAction(verbMenu.forRow, "edit")
            }
            AppMenuSeparator {}
            // The two folds land in the nearest row below that stays in the history, so a row with nothing under it
            // but drops — or nothing at all — never offers them (the model demotes what a later change strands, the
            // same line).
            AppMenuItem {
                code: "squash"
                text: qsTr("into parent, both messages")
                offered: verbMenu.foldLands
                onTriggered: planPane.planModel.setAction(verbMenu.forRow, "squash")
            }
            AppMenuItem {
                code: "fixup"
                text: qsTr("into parent, its message dropped")
                offered: verbMenu.foldLands
                onTriggered: planPane.planModel.setAction(verbMenu.forRow, "fixup")
            }
            AppMenuSeparator {}
            AppMenuItem {
                code: "drop"
                text: qsTr("leave it out")
                onTriggered: planPane.planModel.setAction(verbMenu.forRow, "drop")
            }
        }
    }
}
