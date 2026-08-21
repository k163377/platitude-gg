import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The way out of a stopped operation, built into the working-tree pane under the button that finishes things.
//
// The commit button's seat is already "conclude this": during a merge `--continue` is literally the commit. Standing
// rather than dropped from a click — while an operation is stopped this is the pane's business, and the file list moves
// down to make room for it rather than the rows covering the list.
Rectangle {
    id: opExitCard

    required property var repoTab
    required property var workTree

    // The chip column and the mark's seat, kept on every row whether or not that row is a held one, so the sentences
    // start on one x and the card reads down its first letters — what `AppMenu` does for a menu's rows. The seat is the
    // menu's exactly: narrower than the mark, which overhangs it into the row padding on one side and the word gap on
    // the other (デザイン規約 §長押し — 語が払う字下げは印 1 個分より小さい).
    readonly property real holdIndent: Theme.iconMd - 2 * Theme.spaceXs
    /// Whether leaving the stopped commit out costs nothing.
    ///
    /// An interactive rebase stops on a commit that came out empty and names `--skip` as the way past it (measured —
    /// `an_interactive_rebase_stops_on_an_emptied_commit_and_names_skip`). Nothing is conflicted or staged while it
    /// stands there, and a clean tree under a stopped operation is what tells that stop from every other one this app
    /// can reach today.
    ///
    /// **Revisit when `edit` steps land** (full interactive rebase): an `edit` stop is clean too, and skipping one does
    /// lose the commit. Distinguishing them needs core to say why git stopped (P3-確認事項).
    readonly property bool skipIsFree:
        opExitCard.workTree.conflictCount === 0
        && opExitCard.workTree.stagedCount === 0
        && opExitCard.workTree.unstagedCount === 0
    readonly property real codeColW: {
        let widest = 0
        for (let i = 0; i < opExitCol.children.length; i++) {
            const row = opExitCol.children[i]
            if (row && row.visible && row.codeColSeat !== undefined)
                widest = Math.max(widest, row.codeColSeat)
        }
        return widest
    }

    /// Automation: run one of the held rows to its end, named by its flag.
    function completeOpExit(code) {
        for (let i = 0; i < opExitCol.children.length; i++) {
            const row = opExitCol.children[i]
            if (row && row.code === code) {
                row.completeHold()
                return true
            }
        }
        return false
    }

    visible: opExitCard.workTree.opText !== ""
    implicitHeight: opExitCol.implicitHeight + 2 * Theme.spaceXs
    color: Theme.bgBase
    radius: Theme.radiusMd
    border.color: Theme.warning
    border.width: Theme.borderWidth
    ColumnLayout {
        id: opExitCol
        anchors.fill: parent
        anchors.margins: Theme.spaceXs
        spacing: 0
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.spaceXs
            Layout.bottomMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: opExitCard.workTree.opText
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                color: Theme.warning
            }
            // Bisect, when it is running alongside. The mark between the two names is drawn, not typed (規約 §余白).
            DotMark {
                visible: opExitCard.workTree.opAlso !== ""
                tint: Theme.warning
                Layout.alignment: Qt.AlignVCenter
            }
            Label {
                visible: opExitCard.workTree.opAlso !== ""
                text: opExitCard.workTree.opAlso
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                color: Theme.warning
            }
            // How far it got. The count is the half a stopped rebase cannot say without it. The dash stays a dash: it
            // is a sentence here, and it measured the same on both OSes.
            Label {
                visible: opExitCard.workTree.opSteps > 0
                text: qsTr("— %1 of %2")
                      .arg(opExitCard.workTree.opStep)
                      .arg(opExitCard.workTree.opSteps)
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.warning
            }
            // Someone takes the slack, or the engine centres what it cannot fill.
            Item { Layout.fillWidth: true }
        }
        OpExitRow {
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--continue"
            text: qsTr("Carry on with what is staged")
            enabled: opExitCard.repoTab.busyCount === 0
                     && opExitCard.workTree.conflictCount === 0
            onPicked: opExitCard.repoTab.resolveOperation("continue")
        }
        // A merge steps through nothing: no commit to leave out, and nowhere to stop stepping.
        OpExitRow {
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--skip"
            text: qsTr("Leave this commit out")
            // What the skip costs, said where a menu row says it.
            note: opExitCard.skipIsFree ? qsTr("nothing in it") : ""
            visible: opExitCard.workTree.opStepping
            enabled: opExitCard.repoTab.busyCount === 0
            // The mark is what says a row takes something away (デザイン規約 §長押し), so it goes when the row does not: git
            // stops on a commit that came out empty and names `--skip` itself, and leaving that one out loses nothing.
            // Everywhere else the commit is real and only the reflog holds it afterwards.
            holdMs: opExitCard.skipIsFree ? 0 : Metrics.holdMs
            onPicked: opExitCard.repoTab.resolveOperation("skip")
        }
        OpExitRow {
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--quit"
            text: qsTr("Stop stepping, keep the tree")
            visible: opExitCard.workTree.opStepping
            enabled: opExitCard.repoTab.busyCount === 0
            onPicked: opExitCard.repoTab.resolveOperation("quit")
        }
        OpExitRow {
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--abort"
            text: qsTr("Undo it all and go back")
            enabled: opExitCard.repoTab.busyCount === 0
            holdMs: Metrics.holdMs
            onPicked: opExitCard.repoTab.resolveOperation("abort")
        }
    }
}
