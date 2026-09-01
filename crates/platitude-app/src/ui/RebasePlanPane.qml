import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The interactive-rebase plan, stood where the graph was: the rows of `from^..HEAD` newest-first, the verb each one
// carries, and the base they land on as the list's own last row. Composing runs nothing — the one door out of here
// that touches the repository is the run button standing at the right pane's foot, and Cancel up here only puts the
// draft away (デザイン規約 §履歴を合流させる).
//
// While this pane stands, the surfaces that could move the history under it are held down by the page (the sidebar,
// find, the toolbar's writes); what cannot be held — another session, a terminal — is answered by the model putting
// the plan away when the tip moves (`RebasePlanModel.noteHead`), and by core pinning the tip again at run time.
Rectangle {
    id: planPane

    required property var planModel
    /// The commit the page's selection sits on, so the row under it reads selected the way a graph row would.
    property string selectedOid: ""
    /// The read that opens the plan is still out. The pane takes the graph's seat at the press rather than at the
    /// answer (`RepoPage.planShown`), so this is the face it has until the rows land: the band's one word, the turning
    /// mark in the middle of the column that will hold them, and nothing it does not yet know — the base has no name
    /// and no id yet, and neither has the question of whether there is a base at all (デザイン規約 §フル interactive rebase).
    required property bool waiting

    /// Which commit the reader picked, for the page to select (the details pane follows it).
    signal rowPicked(string oidHex)

    /// Cancel would take composed work away with the plan — verbs set, rows moved, a reword saved into it, or one
    /// typed into the right pane's boxes and not yet given to it. None of that can be read back off the screen
    /// afterwards, so the button is held rather than clicked (デザイン規約 §長押し). Handed in: half the answer is the
    /// right pane's, and this pane owns nothing but the rows (`RepoPage.planDiscards`).
    property bool discards: false
    /// The same answer as the press under way was given it — **whether this is a hold is settled the moment the
    /// button goes down** (デザイン規約 §フル interactive rebase, the same line `RebasePlanRunBar` stands on).
    /// `discards` follows the boxes in the right pane, so it can fall under a hand that is already holding, and
    /// `ActionButton` reads the length again at the release: a hold begun on the warned button would come back as a
    /// click and put the plan away on a gesture nobody made. The frame and the word freeze with it.
    ///
    /// Declared with the live value so it reads right from the start; the `Binding` below is what keeps it, dropped
    /// while a press is under way with nothing put back after (`RestoreNone`) — which is the freeze itself.
    property bool cancelHolds: planPane.discards
    /// A gesture is under way. The pointer's is `down`; the keyboard's is not — an armed hold takes Space itself
    /// (`HoldDriver.pressKey`), so `down` never rises for it and the climbing fill is what says the press is there.
    readonly property bool cancelPressing: cancelButton.down || cancelButton.holdProgress > 0

    /// The list the rows stand in, so a verb can make the same calls a hand on a row makes. An automation-only
    /// exposure, the same one `GraphPane.view` is (app-ui.md).
    readonly property alias view: planList

    color: Theme.bgBase

    Binding {
        target: planPane
        property: "cancelHolds"
        value: planPane.discards
        when: !planPane.cancelPressing
        restoreMode: Binding.RestoreNone
    }

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
            id: planHead
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.toolbarHeight
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm
                // The mode is named from the press, before the read that would finish the sentence has landed: the
                // band says what this face is for the whole time it is up, and the rest of the sentence arrives with
                // the rows.
                //
                // **The `…` is the sentence's, not the wait's**: what it marks is that this phrase is cut short and
                // will be finished — the elision mark of 規約 §ウィンドウの縁, not the progress mark
                // §進行中・長押しの定数 refuses. Without it the band reads as a finished sentence and the name
                // arriving rewrites it; with it the name lands where the mark already said something was missing.
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
                    text: Words.cancel
                    // The frame carries what is about to be lost, and the word takes the colour only because this is
                    // a hold (規約 §長押し — `RebasePlanRunBar` と同じ線). The word itself does not change: pressing
                    // this still puts the plan away, hold or no hold. All three read the latch, not the live answer,
                    // so nothing about the button changes under a hand that is already holding.
                    frameColor: planPane.cancelHolds ? Theme.warning : Theme.borderDefault
                    tone: planPane.cancelHolds ? Theme.warning : Theme.textPrimary
                    holdMs: planPane.cancelHolds ? Metrics.holdMs : 0
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
            // No rows until they are the plan's — the half of this face that arrives late, and what makes the wait one
            // face rather than a band and a mark laid over whatever else is there. **Costs nothing while the read is
            // really out**: the model has no rows then either.
            //
            // The count, not `visible`: a hidden child leaves the column with nothing to stretch, and the band drops to
            // the middle of the pane with the mark on top of it (observed on Linux — where the rows land before the
            // grab does, and the report line stays green because it says what the edge saw).
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

            // The base the rows land on, as the row after the oldest one: what the squash arrows point at, and the
            // one row here nothing can be done to. A list with no rows still lays its footer out, so the wait takes it
            // down as well — the base is exactly what the read has not answered yet, and its row drawn empty would be
            // a commit with no face, no words and no id.
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
                    // The base's name is said once, in the header's own sentence — this row is the commit itself,
                    // dressed like every other row: face, subject, id.
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

    // The read walking the range, in the middle of the column that will hold its rows — the same mark at the same size
    // in the same seat the graph's own first load puts one in (`GraphEmptyState`), because this pane is standing where
    // that column was and waiting should not change its look when the seat changes hands.
    //
    // Centred below the head rather than in the pane: the band is up the whole time and the rows will start under it,
    // so the middle of what is waiting is half a head lower than the middle of the pane.
    SpinnerIcon {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: planHead.height / 2
        width: Theme.iconLg
        height: Theme.iconLg
        spinning: planPane.waiting
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
