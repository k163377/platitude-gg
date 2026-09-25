import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The interactive-rebase plan in the graph's seat: the rows of `from^..HEAD` newest-first with their verbs, and the
// base as the list's last row. Composing runs nothing — the run button at the right pane's foot is the one door to the
// repository, and `Discard` only puts the draft away (デザイン規約 §履歴を合流させる).
//
// The page holds down what could move the history meanwhile; what it cannot hold (another session, a terminal) is
// answered by the model dropping the plan when the tip moves (`RebasePlanModel.noteHead`) and by core re-pinning the
// tip at run time.
Rectangle {
    id: planPane

    required property var planModel
    /// The page's selected commit; its row reads selected.
    property string selectedOid: ""
    /// The read that opens the plan is still out. The pane already stands (`RepoPage.planShown`) with the band's word
    /// and the spinner only — nothing about the base is known yet (デザイン規約 §フル interactive rebase).
    required property bool waiting

    /// Which commit the reader picked, for the page to select (the details pane follows it).
    signal rowPicked(string oidHex)

    /// `Discard` would throw away composed work (verbs, moves, a reword saved or still in the right pane's boxes), so
    /// it becomes a hold (デザイン規約 §長押し). Handed in: half the answer is the right pane's (`RepoPage.planDiscards`).
    property bool discards: false

    /// Automation only: the list, so a verb makes the same calls a hand on a row makes.
    readonly property alias view: planList

    color: Theme.bgBase

    // The widest verb sizes the chip column, measured off a real chip: a metrics method in a binding takes no
    // dependency and stays on the default font (rules/app-ui.md).
    CodeChip {
        id: verbProbe
        visible: false
        word: "reword"
        size: Theme.fontChip
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The head names the mode: nothing is running (§git 用語のコード表記 — 状態は通常表記).
        Item {
            id: planHead
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.toolbarHeight
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm
                // Named from the press; the rest of the sentence arrives with the rows. The `…` says the phrase will
                // be finished, not that something is in progress (規約 §進行中・長押しの定数「文がまだ途中」).
                Label {
                    text: planPane.waiting ? qsTr("Rebasing…")
                        : planPane.planModel.root ? qsTr("Rebasing back to the very first commit")
                        : qsTr("Rebasing onto")
                    font.pixelSize: Theme.fontMd
                    color: Theme.textPrimary
                }
                // The base, its branch name first and the id only as the fallback.
                Label {
                    visible: !planPane.waiting && !planPane.planModel.root && planPane.planModel.ontoRef !== ""
                    text: planPane.planModel.ontoRef
                    font.pixelSize: Theme.fontMd
                    color: Theme.accentHover
                }
                Label {
                    visible: !planPane.waiting && !planPane.planModel.root && planPane.planModel.ontoRef === ""
                    text: planPane.planModel.ontoOid.substring(0, 8)
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontCode
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
                ActionButton {
                    id: cancelButton
                    //: The way out of the plan being composed. What it throws away is the draft on screen.
                    text: qsTr("Discard")
                    // A plain word, not `--abort`: nothing has run, and git would refuse that command here
                    // (デザイン規約 §git 用語のコード表記「着せるのはコマンドだけ」).
                    // The dressing reads `armedMs`, not `discards`: the right pane's boxes can empty under a hand
                    // already holding (規約 §長押し).
                    frameColor: cancelButton.armedMs > 0 ? Theme.warning : Theme.borderDefault
                    tone: cancelButton.armedMs > 0 ? Theme.warning : Theme.textPrimary
                    holdMs: planPane.discards ? Metrics.holdMs : 0
                    onActivated: planPane.planModel.cancelPlan()
                    onHeld: planPane.planModel.cancelPlan()
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
            // No rows while waiting — by the model, not by hiding the list: a hidden child leaves the column nothing
            // to stretch, and the band drops to the pane's middle.
            model: planPane.waiting ? 0 : planPane.planModel

            /// What every row reads and calls (the delegate reaches the pane through its view).
            readonly property string selectedOid: planPane.selectedOid
            readonly property real verbColW: verbProbe.wantWidth
            function rowPicked(row, oidHex) {
                planPane.planModel.selectRow(row)
                planPane.rowPicked(oidHex)
            }
            function moveBegan() {
                planPane.planModel.beginMove()
            }
            function moveRequested(from, to) {
                planPane.planModel.moveStep(from, to)
            }
            function moveEnded() {
                planPane.planModel.endMove()
            }
            function verbMenuRequested(row) {
                verbMenu.openFor(row)
            }

            delegate: RebasePlanRow {}

            // The base, as the row after the oldest. An empty list still lays its footer out, so waiting takes it down
            // too.
            footer: Item {
                width: planList.width
                height: planPane.waiting || planPane.planModel.root ? 0 : Theme.graphRowHeight + Theme.spaceSm
                visible: !planPane.waiting && !planPane.planModel.root
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
                    // The base's name is in the header; this row is the commit, dressed like the others.
                    Label {
                        Layout.preferredWidth: planList.verbColW
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

    // The graph's first-load mark (`GraphEmptyState`), centred on the column under the head.
    SpinnerIcon {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: planHead.height / 2
        width: Theme.iconLg
        height: Theme.iconLg
        spinning: planPane.waiting
    }

    // The todo file's six verbs, chips first (デザイン規約 §git 用語のコード表記). The row and whether a fold lands are
    // frozen at open (デザイン規約 §メニュー「中身は開いた瞬間に決め、開いている間は凍らせる」); the landing question is
    // the model's `canFold`, the same answer the demotion enforces.
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
            // A fold lands in the nearest kept row below, so it is not offered with only drops (or nothing) under it.
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
