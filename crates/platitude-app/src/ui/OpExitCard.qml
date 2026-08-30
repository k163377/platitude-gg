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
    /// Whether leaving the stopped commit out costs nothing — the stop is on a commit that came out empty, which a
    /// clean tree under a stopped operation is what tells. The rule, its measurement and the revisit note for `edit`
    /// stops are core's (offers::skip_is_free).
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

    /// Automation: whether the card offers a row at all. A row that is not there and a row that is there and down
    /// crop to the same picture (the card sizes itself to what it holds).
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
        if (code === "--abort" && abortButton.visible) {
            abortButton.completeHold()
            return true
        }
        for (let i = 0; i < opExitCol.children.length; i++) {
            const row = opExitCol.children[i]
            if (row && row.code === code) {
                row.completeHold()
                return true
            }
        }
        return false
    }

    /// A stopped merge has no card left — it is one button, and the button is the whole of it.
    ///
    /// **A card is what holds a column together**: the frame gathers the rows, and the heading says which operation
    /// they belong to. One button gathers nothing, and its own chip already names the operation, so both would only
    /// draw a box around a box and say `MERGING` twice (the toolbar badge says it first — デザイン規約 §長さ).
    /// What is left is a red button standing under the one that finishes things, which is what it is.
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
        // Nothing of its own to inset by when the button is all there is: it lines up with the commit button above,
        // which is the pane's own column and pays its own margins.
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
        // The stop that was asked for: the plan's own `edit` step. Its tree is as clean as the emptied-commit stop's,
        // so without this line the card cannot say why it is standing — and here `--skip` takes the commit out, which
        // is why the row above went back to a hold (offers::skip_is_free, P3-確認事項 §A).
        RowLayout {
            visible: opExitCard.workTree.opEditing
            Layout.fillWidth: true
            Layout.leftMargin: Theme.spaceXs
            Layout.bottomMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            CodeChip {
                word: "edit"
                size: Theme.fontSm
                tint: Theme.textSecondary
                Layout.alignment: Qt.AlignVCenter
            }
            Label {
                Layout.fillWidth: true
                // The oid is abbreviated as git wrote it (`rebase-merge/stopped-sha`); HEAD sits on that commit, so
                // the boxes above are already the tool (offers::message_edit の edit 停止の免除).
                text: opExitCard.workTree.opEditOid !== ""
                      ? qsTr("Stopped on purpose at %1 — amend it above, then continue")
                            .arg(opExitCard.workTree.opEditOid.substring(0, 8))
                      : qsTr("Stopped on purpose — amend the commit above, then continue")
                elide: Text.ElideRight
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
            }
        }
        // **A merge does not get one.** `--continue` there *is* the commit — same tree, same two parents, same
        // message, same hooks as pressing the button above this card (measured, 2.55) — so the card would be offering a
        // second door onto the seat it is standing under, and the one with no message box attached. Everything else
        // here steps, and continuing a step is not a commit anybody is composing (デザイン規約 §進行中の操作から出る).
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
            // **The tree it keeps is the conflicted one.** git drops the operation and leaves every unmerged path
            // exactly where it stood (measured, 2.55), so the badge and this card go while the files still wait on a
            // decision — and a move out of here is refused all over again, in git's other wording. Said in the seat
            // `--skip` says its own cost from (デザイン規約 §進行中の操作から出る).
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
        // **A merge's only way out is a button, not a row.** A column of rows is what the other three are — the hand
        // that learned the reset submenu reads them down their first letters, and the chip column is what lines those
        // letters up. One row has no column and nothing to line up with (デザイン規約 §進行中の操作から出る).
        //
        // **The shape is the commit button's, in red**: a frame of its own, the phrase centred inside it, the hold
        // filling the frame it drew. The two then read as the pair they are — the one that finishes the merge and the
        // one that puts it back — and neither is a box drawn inside another box.
        //
        // **The word is red because the gesture is a hold.** Colour on a word is this application's mark of a press
        // that has to be held (§長押し), and taking the hold away would take the colour with it.
        ActionButton {
            id: abortButton
            Layout.fillWidth: true
            visible: opExitCard.bare
            centred: true
            tone: Theme.danger
            frameColor: Theme.danger
            holdMs: Metrics.holdMs
            // The command in git's own spelling, whole rather than as a bare flag: the chip is the row's only name
            // now, and `merge --abort` is what a terminal would be told (§git 用語のコード表記 — the same shape the
            // commit button's `commit --amend` takes).
            phraseHead: "merge --abort"
            // **What happens, not what is lost in general.** The hold's mark already says something goes; the words
            // say where it lands, and that landing is the whole of the answer — nothing done since the merge began
            // survives it.
            text: qsTr("Back to before it started")
            enabled: opExitCard.repoTab.busyCount === 0
            onHeld: opExitCard.repoTab.resolveOperation("abort")
        }
    }
}
