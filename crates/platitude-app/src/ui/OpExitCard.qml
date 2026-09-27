import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The way out of a stopped operation, in the working-tree pane under the commit button (whose seat already means
// "conclude this"); the file list moves down for it.
Rectangle {
    id: opExitCard

    required property var repoTab
    required property var workTree

    // The mark's seat on every row, held or not, as `AppMenu.holdIndent`
    // (デザイン規約 §長押し「メニューでは印の席を全行が空ける」).
    readonly property real holdIndent: Theme.iconMd - 2 * Theme.spaceXs
    /// Whether leaving the stopped commit out costs nothing — core's rule (`offers::skip_is_free`).
    readonly property bool skipIsFree: opExitCard.workTree.opSkipFree
    readonly property real codeColW: {
        let widest = 0
        for (let i = 0; i < opExitCol.children.length; i++) {
            const row = opExitCol.children[i]
            if (row && row.visible && row.codeColSeat !== undefined)
                widest = Math.max(widest, row.codeColSeat)
        }
        return widest
    }

    /// Automation: whether the card offers a row — the picture cannot say, since the card sizes itself to what it
    /// holds.
    function offersOpExit(code) {
        if (code === "--abort" && abortButton.visible)
            return true
        for (let i = 0; i < opExitCol.children.length; i++) {
            const row = opExitCol.children[i]
            if (row && row.code === code)
                return row.visible
        }
        return false
    }

    /// Automation: run one of the held rows to its end, named by its flag.
    function completeOpExit(code) {
        if (code === "--abort" && abortButton.visible)
            return abortButton.completeHold()
        for (let i = 0; i < opExitCol.children.length; i++) {
            const row = opExitCol.children[i]
            if (row && row.code === code) {
                row.completeHold()
                return true
            }
        }
        return false
    }

    /// A stopped merge has no card: it is one button, and a frame and heading around one button would draw a box
    /// around a box (rules-refs/app-ui.md「止まった merge の出口は `ActionButton` 1 個で、カードごと消える」).
    readonly property bool bare: opExitCard.workTree.opMerging

    visible: opExitCard.workTree.opText !== ""
    implicitHeight: opExitCol.implicitHeight + (opExitCard.bare ? 0 : 2 * Theme.spaceXs)
    color: opExitCard.bare ? "transparent" : Theme.bgBase
    radius: Theme.radiusMd
    border.color: opExitCard.bare ? "transparent" : Theme.warning
    border.width: opExitCard.bare ? 0 : Theme.borderWidth
    ColumnLayout {
        id: opExitCol
        anchors.fill: parent
        // Bare, the button lines up with the commit button above.
        anchors.margins: opExitCard.bare ? 0 : Theme.spaceXs
        spacing: 0
        RowLayout {
            visible: !opExitCard.bare
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
            // Bisect, when it is running alongside. The mark between the two names is drawn (規約 §余白).
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
            // How far it got. The dash stays a character, not a drawn mark: it is part of a sentence, and renders
            // alike on both OSes.
            Label {
                visible: opExitCard.workTree.opSteps > 0
                text: qsTr("— %1 of %2")
                      .arg(opExitCard.workTree.opStep)
                      .arg(opExitCard.workTree.opSteps)
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.warning
            }
            // Takes the slack, or the layout centres what it cannot fill.
            Item { Layout.fillWidth: true }
        }
        // The plan's own `edit` stop: its tree is as clean as an emptied commit's, so without this line the card cannot
        // say why it stands — and `--skip` below is a hold again (`offers::skip_is_free`,
        // デザイン規約 §フル interactive rebase).
        RowLayout {
            visible: opExitCard.workTree.opEditing
            Layout.fillWidth: true
            Layout.leftMargin: Theme.spaceXs
            Layout.bottomMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            // The chip stands on the line's first row: both hang from the top and the shorter drops half the
            // difference (as `AskBar`'s heading).
            CodeChip {
                id: editChip
                word: "edit"
                size: Theme.fontSm
                tint: Theme.textSecondary
                Layout.alignment: Qt.AlignTop
                Layout.topMargin: Math.max(0, (editWords.lineHeight - editChip.implicitHeight) / 2)
            }
            // **Wrapped, never cut** (デザイン規約 §フル interactive rebase): the second half is what to do, and a cut
            // takes exactly that. What comes after it is the `--continue` row under it, and that more steps follow is
            // the heading's count. HEAD sits on that commit, so the message boxes above are the tool
            // (`offers::message_edit` の edit 停止の免除).
            CardText {
                id: editWords
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                Layout.topMargin: Math.max(0, (editChip.implicitHeight - editWords.lineHeight) / 2)
                text: opExitCard.workTree.opEditOid !== ""
                      ? qsTr("Stopped at %1 — amend it above")
                            .arg(opExitCard.workTree.opEditOid.substring(0, 8))
                      : qsTr("Stopped — amend the commit above")
                pixelSize: Theme.fontSm
                color: Theme.textSecondary
            }
        }
        // **Not for a merge**: its `--continue` *is* the commit the button above makes, and a row here would be a
        // second door onto it with no message box (デザイン規約 §進行中の操作から出る).
        OpExitRow {
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--continue"
            text: qsTr("Carry on with what is staged")
            visible: !opExitCard.workTree.opMerging
            enabled: opExitCard.repoTab.busyCount === 0
                     && opExitCard.workTree.conflictCount === 0
            onPicked: opExitCard.repoTab.resolveOperation("continue")
        }
        // A merge steps through nothing: no commit to leave out, and nowhere to stop stepping.
        OpExitRow {
            id: skipRow
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--skip"
            text: qsTr("Leave this commit out")
            // Off the length the press under way was given, so tag and gesture agree under a hand
            // (`OpExitRow.armedMs`).
            note: skipRow.armedMs <= 0 ? qsTr("nothing in it") : ""
            visible: opExitCard.workTree.opStepping
            enabled: opExitCard.repoTab.busyCount === 0
            // Held only where a real commit is lost (デザイン規約 §長押し); an emptied commit's stop loses nothing.
            holdMs: opExitCard.skipIsFree ? 0 : Metrics.holdMs
            onPicked: opExitCard.repoTab.resolveOperation("skip")
        }
        OpExitRow {
            Layout.fillWidth: true
            codeColW: opExitCard.codeColW
            holdIndent: opExitCard.holdIndent
            code: "--quit"
            text: qsTr("Stop stepping, keep the tree")
            // **The tree it keeps is the conflicted one**: git drops the operation but leaves every unmerged path, so
            // this card goes while the files still wait (デザイン規約 §進行中の操作から出る).
            note: opExitCard.workTree.conflictCount > 0 ? qsTr("conflicts stay") : ""
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
            visible: !opExitCard.workTree.opMerging
            enabled: opExitCard.repoTab.busyCount === 0
            holdMs: Metrics.holdMs
            onPicked: opExitCard.repoTab.resolveOperation("abort")
        }
        // A merge's only way out (`bare`): the commit button's shape in red, so the two read as the pair that finishes
        // the merge and the one that puts it back (デザイン規約 §進行中の操作から出る). The word is red because the
        // gesture is a hold (§長押し).
        ActionButton {
            id: abortButton
            Layout.fillWidth: true
            visible: opExitCard.bare
            centred: true
            tone: Theme.danger
            frameColor: Theme.danger
            holdMs: Metrics.holdMs
            // The whole command in git's spelling (§git 用語のコード表記), as the commit button's `commit --amend`.
            phraseHead: "merge --abort"
            // Where it lands: nothing done since the merge began survives it.
            text: qsTr("Back to before it started")
            enabled: opExitCard.repoTab.busyCount === 0
            onHeld: opExitCard.repoTab.resolveOperation("abort")
        }
    }
}
