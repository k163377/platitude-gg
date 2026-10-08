pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Everything under the WIP pane's file list, pinned to the pane's bottom: the message pair, the amend row, the
// commit button, and the way out of a stopped operation — which alone stands while a rebase, pick or revert is
// stopped anywhere but an `edit` step (`composeHidden`).
//
// Scrolls on its own when the pane is too short: its contents keep their height, so past the list's zero they would
// be laid out below the pane's edge, and the window's floor cannot grow with a card git puts up (規約 §窓の床).
Flickable {
    id: block

    /// What the pane hands down: the tree this commit is made of, and the room it is given.
    required property var repoTab
    required property var worktree
    required property string standingSubject
    required property string standingBody
    required property bool onStandingMessage
    required property bool amending
    required property bool headPublished
    required property int mateCount
    required property string commitTarget
    required property bool signingPointedAt
    required property real blockRoom
    required property real listHeight

    /// Up to the pane: the amend tick, the press, and the moment the button's warning card may need settling (the
    /// pane places it).
    signal amendToggled(bool on)
    signal commitClicked()
    signal cardAsked()

    /// What the block would take with nothing in its way — the pane holds it against `blockRoom`.
    readonly property real wants: blockCol.implicitHeight
    /// A rebase, cherry-pick or revert standing anywhere but an `edit` step: the message pair, the amend row and the
    /// commit button go, and the exit card stands in the button's seat (デザイン規約 §進行中の操作から出る). Its rows
    /// are the way on there, and a commit of one's own would drop what the stopped commit carries — a rebase's author
    /// and message, a pick's message. The edit step keeps them: amending is what it stops for. A merge keeps them too:
    /// its `--continue` is the commit this button makes.
    readonly property bool composeHidden: block.worktree.opStepping && !block.worktree.opEditing

    // -- what the pane reads back out of here --
    readonly property alias subjectText: msgEditor.subjectText
    readonly property alias bodyText: msgEditor.bodyText
    readonly property alias descriptionColor: msgEditor.descriptionColor
    readonly property alias descriptionFocused: msgEditor.descriptionFocused
    /// How far the words in the two boxes stand (verify-ui).
    readonly property alias descriptionAt: msgEditor.descriptionAt
    readonly property alias summaryAt: msgEditor.summaryAt
    readonly property alias descRefuses: msgEditor.descRefuses
    readonly property alias descPoint: msgEditor.descPoint
    readonly property alias descGrips: msgEditor.descGrips
    readonly property alias descHeight: msgEditor.descHeight
    readonly property alias descWants: msgEditor.descWants
    readonly property alias descCap: msgEditor.descCap
    readonly property alias descListRows: msgEditor.descListRows
    readonly property alias blockScrolls: msgEditor.blockScrolls
    readonly property alias descKeeps: msgEditor.descKeeps
    readonly property alias amendChecked: amendBox.checked
    readonly property alias resetAuthor: authorBox.checked
    /// For the card the pane places on the commit button; `commitSeat` is the button, which the pane measures from.
    readonly property alias commitWarned: commitButton.eolWarned
    readonly property alias commitPointed: commitHover.containsMouse
    readonly property alias commitSeat: commitButton
    /// Automation: the message pair itself, to read whether it stands (`composeHidden` takes it down).
    readonly property alias messageSeat: msgEditor
    readonly property alias commitEnabled: commitButton.enabled
    readonly property alias signingTipShown: commitButton.phraseSignatureTipShown
    /// Automation only: the block's own hand and the two the boxes carry, started and drifted without a pointer (a
    /// middle button cannot be injected).
    readonly property alias hand: hand
    readonly property alias summaryHand: msgEditor.summaryHand
    readonly property alias descriptionHand: msgEditor.descriptionHand

    // -- and what it does on the pane's word --
    function setMessage(subject, body) { msgEditor.setMessage(subject, body) }
    function setAmendChecked(on) { amendBox.checked = on }
    function setResetAuthorChecked(on) { authorBox.checked = on }
    function focusDescription() { msgEditor.focusDescription() }
    function growDescription(dy) { msgEditor.growDescription(dy) }
    function pullDescriptionPast(down) { msgEditor.pullDescriptionPast(down) }
    function completeOpExit(code) { return opExitCard.completeOpExit(code) }
    function offersOpExit(code) { return opExitCard.offersOpExit(code) }
    /// Moves the block by a wheel a box on it could not use — the boxes cover most of the block, which would
    /// otherwise never scroll by wheel.
    function rollBlock(pixels) {
        const max = Math.max(0, block.contentHeight - block.height)
        // Subtracted: `pixels` is how far the content travels, and content travels against `contentY`.
        block.contentY = Math.max(0, Math.min(max, block.contentY - pixels))
    }

    contentWidth: width
    contentHeight: blockCol.implicitHeight
    clip: true
    // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: PaneScrollBar {}
    // The middle button's hand, as `DetailsMessageBlock` carries; a box with a hand of its own takes the press.
    MiddleAutoScroll {
        id: hand
        parent: block
        anchors.fill: parent
        visible: block.ScrollBar.vertical.visible
        // `rollBlock` takes the wheel's sense — content travels against `contentY`.
        onDrifted: dy => block.rollBlock(-dy)
    }
    ColumnLayout {
        id: blockCol
        width: block.width
        spacing: 0

        // Inset as in DetailsPane, so switching between the two doesn't move the message box.
        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.spaceXs
            // The scroll bar's gutter.
            Layout.rightMargin: Theme.navBarGutter
            spacing: Theme.spaceXs
            // The pair, one block of one height (デザイン規約 §コミットメッセージの 2 つの枠).
            MessageEditor {
                id: msgEditor
                visible: !block.composeHidden
                Layout.fillWidth: true
                Layout.preferredHeight: msgEditor.pairHeight
                standingSubject: block.standingSubject
                standingBody: block.standingBody
                listHeight: block.listHeight
                blockRoom: block.blockRoom
                blockHeight: blockCol.implicitHeight
                onWheelPastEnd: pixels => block.rollBlock(pixels)
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // Nothing to amend before the first commit.
                visible: block.worktree.headOid !== "" && !block.composeHidden
                AppCheckBox {
                    id: amendBox
                    text: qsTr("Amend the last commit")
                    font.pixelSize: Theme.fontSm
                    onToggled: block.amendToggled(checked)
                }
                Item { Layout.fillWidth: true }
                // A tag, not a confirmation: rewriting a pushed commit is undone by a switch or a reset.
                Label {
                    visible: block.amending && block.headPublished
                    text: qsTr("already pushed")
                    color: Theme.warning
                    font.pixelSize: Theme.fontSm
                    ToolTip.visible: amendPushedHover.containsMouse
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: qsTr("Already on a remote — anyone who has it will be out of step")
                    MouseArea {
                        id: amendPushedHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }
            }
            // An amend keeps the commit's author; offered only where that identity differs from the user's, and
            // unchecked whenever it goes away. On its own line: beside the amend box, the row's three labels do not
            // fit the pane and none elides.
            AppCheckBox {
                id: authorBox
                visible: block.amending && block.repoTab.headAuthorDiffers && !block.composeHidden
                text: qsTr("Make me the author")
                font.pixelSize: Theme.fontSm
                onVisibleChanged: if (!visible) checked = false
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Replaces the author %1 <%2> with you, dated now")
                              .arg(block.repoTab.headAuthorName)
                              .arg(block.repoTab.headAuthorEmail)
            }
            // The toolbar's framed button, so its frame, tone and `!` say its state in a vocabulary already read.
            ActionButton {
                id: commitButton
                visible: !block.composeHidden
                Layout.fillWidth: true
                // The pane's own action, filling the width: a step up (`fontLg`, the summary's size) at the
                // toolbar's height — at control height it reads as a strip.
                font.pixelSize: Theme.fontLg
                implicitHeight: Theme.toolbarHeight
                /// A staged file's line endings changed. Only the index counts — a working-tree mark is not in this
                /// commit (`session::EolMark::staged`).
                readonly property bool eolWarned:
                    block.worktree.eolStagedCount > 0 && commitButton.enabled
                // The frame and the `!` follow as bindings; the card has to be asked (`settleCommitCard`).
                onEolWarnedChanged: block.cardAsked()
                // No icon: the `!` is the one mark this button needs, and has to stand out.
                centred: true
                // Plain words in both states: a coloured word means a hold (`Remove`, a stopped fetch), and this
                // is a click. The frame and the mark carry the state.
                tone: Theme.textPrimary
                // Disabled, the frame dims with the words: one this wide left bright reads as a live button with
                // grey words.
                frameColor: !commitButton.enabled ? Theme.borderDefault
                            : commitButton.eolWarned ? Theme.warning : Theme.accent
                alert: commitButton.eolWarned
                alertTone: Theme.warning
                // Command and branch are git's spelling, so both wear the chip (デザイン規約 §git 用語のコード表記);
                // the words between are translatable. The branch takes the accent: which branch one is on is the
                // thing to get wrong here.
                phraseHead: block.amending ? "commit --amend" : "commit"
                phraseTail: block.commitTarget
                phraseTailTint: commitButton.enabled ? Theme.accent : Theme.textMuted
                // Whom it is attributed to (デザイン規約 §アバターを与える), with `+N` for co-authors the message
                // credits as it is typed.
                phraseFace: block.repoTab.authorAvatar
                phraseFaceUrl: block.repoTab.authorAvatarUrl
                phraseMates: block.mateCount
                // A quiet-ink tick when signing is on — not green, which says a signature held (規約 §署名の表示).
                // It warns that a press may raise the agent's passphrase prompt.
                phraseSignature: block.repoTab.signsCommits ? "signed" : ""
                phraseSignaturePointedAt: block.signingPointedAt
                phraseSignatureTip: block.repoTab.signingFormat === "ssh"
                                    ? qsTr("Signed with your ssh key")
                                    : block.repoTab.signingFormat === "x509"
                                      ? qsTr("Signed with your x509 certificate")
                                      : qsTr("Signed with your gpg key")
                // Files, as the list above counts them; none for a message-only amend. Apart from the words, so
                // they can give way without taking it.
                phraseCount: block.worktree.stagedCount === 0
                             ? "" : block.worktree.stagedCount.toString()
                // No count, no noun: `commit files to main` would name a quantity that is not there.
                text: block.worktree.stagedCount === 0 ? qsTr("to")
                      : block.worktree.stagedCount === 1 ? qsTr("file to") : qsTr("files to")
                // Unmerged paths stop it: git will not commit over them (デザイン規約 §可否・警告の出し場所). A stopped
                // merge presses on its own, with an empty box and nothing staged — git writes the merge commit
                // either way, and this seat is the whole way out of a merge (§進行中の操作から出る).
                // Not while hidden either: the pane's own press (`WipPane.pressCommit`) reads this.
                enabled: !block.composeHidden && block.repoTab.busyCount === 0 && block.repoTab.identityReady
                         && block.worktree.conflictCount === 0
                         && (block.onStandingMessage
                             || (msgEditor.subjectText.trim() !== ""
                                 && (block.amending || block.worktree.stagedCount > 0
                                     || block.standingSubject !== "")))
                // One click even when warning: a confirmation on a daily operation gets pressed unread (デザイン規約
                // §可否・警告の出し場所).
                onActivated: block.commitClicked()
                // Why it cannot be pressed. The line-ending warning is the card's (`settleCommitCard`), and only
                // on a button that can be pressed.
                ToolTip.visible: commitHover.containsMouse && !enabled
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: !block.repoTab.identityReady
                              ? qsTr("No name or email set for commits")
                              : block.worktree.conflictCount > 0
                              ? qsTr("Files are still waiting on a decision")
                              : msgEditor.subjectText.trim() === ""
                              ? qsTr("A commit needs a summary")
                              : qsTr("Stage something to commit")
                MouseArea {
                    id: commitHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    onContainsMouseChanged: block.cardAsked()
                }
            }
            // The way out of a stopped operation (`OpExitCard`), under the button that finishes things.
            OpExitCard {
                id: opExitCard
                // No margin of its own: the column's step apart from the commit button, the block's margin under the
                // list's edge when the button is hidden — the one gap every piece of the block keeps.
                Layout.fillWidth: true
                repoTab: block.repoTab
                worktree: block.worktree
            }
        }
    }
}
