pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Everything under the WIP pane's file list, pinned to the pane's bottom: the message pair, the amend row, the
// commit button, and the way out of a stopped operation — which alone stands while a rebase, pick or revert is
// stopped anywhere but an `edit` step (`composeHidden`).
//
// Scrolls on its own when the pane is too short: its contents keep their height, so once the list is down to its two
// rows they would be laid out below the pane's edge, and the window's floor cannot grow with a card git puts up
// (規約 §窓の床) — the exit card comes and goes with what git is in the middle of, so a floor holding it would widen
// the window the moment a rebase stops. In the right pane only the run between the bands and the list scrolls: the pane's heading and the
// bucket bands stay put (a list whose heading has scrolled away no longer says what it lists), and the list keeps two
// rows (`WipPane.blockRoom`). `DetailsMessageBlock` is the details pane's same surface.
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
    /// The commit button is pointed at without a pointer (`WipPane.pointAtCommit`): its card and its refusal come out
    /// as for a hand.
    required property bool commitPointedAt
    required property real blockRoom
    required property real listHeight

    /// Up to the pane: the amend tick and the press (the button's warning card is the pane's, asked off `commitWarned`
    /// and `commitPointed`).
    signal amendToggled(bool on)
    signal commitClicked()

    /// What the block would take with nothing in its way — the pane holds it against `blockRoom`.
    readonly property real wants: blockCol.implicitHeight
    /// A rebase, cherry-pick or revert standing anywhere but an `edit` step: the message pair, the amend row and the
    /// commit button go, and the exit card stands in the button's seat (user decision — ask before changing). Its rows
    /// are the way on there, and a commit of one's own would drop what the stopped commit carries — a rebase's author
    /// and message, a pick's message (measured). **Hidden, not dimmed**: the card has taken the button's part, the one
    /// exception to 規約 §無効「押せない物は押せないように見せる」. Keeping the boxes filled with the stopped commit's
    /// message (GitKraken, Tower, VS Code) is the road not taken; GitHub Desktop hides them too. Typed words survive
    /// the hiding. The edit step keeps them: amending is what it stops for (as every product with an `edit` step of its
    /// own keeps the commit boxes and Amend). A merge keeps them too: its `--continue` is the commit this button makes
    /// (user decision — ask before changing); the stepping three keep theirs on the card, where it only moves on.
    readonly property bool composeHidden: block.worktree.opStepping && !block.worktree.opEditing
    /// A stopped merge takes the amend row away, as no first commit does (user decision — ask before changing): git
    /// will not amend in the middle of one (`You are in the middle of a merge -- cannot amend.`), and the button there
    /// is the merge's own `--continue`. **Hidden, not greyed**: a greyed move that is not in this flow reads as "wait
    /// and it will press", and a disabled checkbox has no tooltip to say why (規約 §無効). A tick from before the merge
    /// goes with the row, or the button would still send `--amend`.
    readonly property bool amendBarred: block.worktree.opMerging
    /// What is staged takes back all the last commit changed, so amending would leave it changing nothing — git refuses
    /// that amend (with terminal steps: `--allow-empty` / `git reset HEAD^`), so the button does first, dimmed with
    /// `Words.amendEmpties` on hover (user decision — ask before changing; `RepoPage.amendInView`). A message-only
    /// amend of an empty HEAD is not this and goes through (`commit::commit`).
    readonly property bool amendEmpties: block.amending && block.worktree.stagedCount > 0
                                         && block.repoTab.amendEmpties
    // Writing `checked = false` emits no `toggled`, so the pane is told outright — else `RepoPage.amending` stays and
    // the button sends `--amend`. Headless: `amend-row` (`--preset conflict-staged`: `shown=false barred=true`).
    onAmendBarredChanged: {
        if (block.amendBarred && amendBox.checked) {
            amendBox.checked = false
            block.amendToggled(false)
        }
    }

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
    /// Automation: whether the amend row stands (`amendBarred` and the rest take it down).
    readonly property alias amendRowShown: amendRow.visible
    readonly property alias signingTipShown: commitButton.phraseSignatureTipShown
    /// Automation only: the block's own hand and the two the boxes carry, started and drifted without a pointer (a
    /// middle button cannot be injected).
    readonly property alias hand: hand
    readonly property alias summaryHand: msgEditor.summaryHand
    readonly property alias descriptionHand: msgEditor.descriptionHand

    // -- and what it does on the pane's word --
    // Functions, not writes through the aliases above: a `readonly property alias` cannot be assigned.
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
            // The pair, one block of one height (`MessageEditor`). Escape only takes the caret out, and a moved
            // selection or another tab leaves the words — the message of a commit not yet made exists nowhere else
            // (unlike the details pane's draft, `DetailsPane.syncMessage`).
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
                id: amendRow
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // Nothing to amend before the first commit, nor in a stopped merge.
                visible: block.worktree.headOid !== "" && !block.composeHidden && !block.amendBarred
                AppCheckBox {
                    id: amendBox
                    text: qsTr("Amend the last commit")
                    font.pixelSize: Theme.fontSm
                    onToggled: block.amendToggled(checked)
                }
                Item { Layout.fillWidth: true }
                // A tag, not a confirmation: rewriting a pushed commit is undone by a switch or a reset.
                Label {
                    id: pushedNote
                    visible: block.amending && block.headPublished
                    text: qsTr("already pushed")
                    color: Theme.warning
                    font.pixelSize: Theme.fontSm
                    readonly property TipAsk pushedTip: TipAsk {
                        target: pushedNote
                        asked: amendPushedHover.containsMouse
                        words: qsTr("Already on a remote — anyone who has it will be out of step")
                    }
                    MouseArea {
                        id: amendPushedHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }
            }
            // An amend keeps the commit's author (git rewrites the committer only); offered only where that identity
            // differs from the user's — someone else's commit, or one written under another identity, is the only
            // place it means anything — and unchecked whenever it goes away. Checked, the author date is now too
            // (`commit --amend --reset-author`). On its own line: beside the amend box, the row's three labels touch
            // the default 400 pane and overflow the 300 floor, and none elides — and as a `RowLayout`'s floor
            // becomes the `ColumnLayout`'s implicit width, what overflows is the whole block (summary, description
            // and the commit button's right edge past the pane, as in `details-fit`). A dependent choice stacks under
            // the box it hangs off (`spaceXs`, no indent). No preset stands all three at once: to judge the room,
            // shoot with a throwaway patch that pins `visible` true.
            AppCheckBox {
                id: authorBox
                visible: block.amending && block.repoTab.headAuthorDiffers && !block.composeHidden
                text: qsTr("Make me the author")
                font.pixelSize: Theme.fontSm
                onVisibleChanged: if (!visible) checked = false
                readonly property TipAsk authorTip: TipAsk {
                    target: authorBox
                    asked: authorBox.hovered
                    words: qsTr("Replaces the author %1 <%2> with you, dated now")
                           .arg(block.repoTab.headAuthorName)
                           .arg(block.repoTab.headAuthorEmail)
                }
            }
            // The toolbar's framed button, so its frame, tone and `!` say its state in a vocabulary already read.
            // One sentence says it all — `commit` `n files to` `<branch>` `by <face>` `+N`: the command run, how much,
            // the ref it lands on and who is recorded are everything a commit records, so the pane needs no other line
            // for them. What gives when it is short of room is `ActionButtonLabel.phraseFixed`'s.
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
                // The frame and the `!` follow as bindings, and so does the card's ask (`WipPane.commitCardAsked`).
                // No icon, words alone: the `!` (line endings) is the one mark this button needs, and a tick that
                // means nothing would bury it (user decision).
                centred: true
                // Plain words in both states: a coloured word means a hold (`Remove`, `push -f`), and this
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
                // The branch behind its mark (デザイン規約 §ref の種別「名前の印」); a detached `HEAD` names no branch.
                phraseTailMark: block.worktree.detached ? "" : "branch"
                // Whom it is committed as: the phrase's last word is the face `user.name` / `user.email` answer to now
                // (`… by <face>`). Not pressable (user decision — ask before changing): a target inside the button's
                // target; avatars are assigned in the settings list. Not a row of face and name either: at 40 it would
                // be the loudest thing in the editor, at 20 too little for a line. Co-authors are only the `+N` after
                // the face — the names are written right above, in the message's `Co-authored-by:` lines, and the
                // count grows as they are typed (`platitude_core::trailers`).
                phraseFace: block.repoTab.authorAvatar
                phraseFaceUrl: block.repoTab.authorAvatarUrl
                phraseMates: block.mateCount
                // Signing is on (`commit.gpgsign` alone — this is about a commit, so `tag.gpgsign` is no part of it): a
                // tick on the face's top right in quiet ink, a fact — not green, since nothing is signed yet (規約
                // §署名の表示). The mark alone says it (user decision — ask before changing): with the face there, the
                // signing is that face's; a `will be signed` at the row's end would say it again elsewhere. Its worth
                // is saying first that a press may raise the agent's passphrase prompt: the app never takes the key's
                // passphrase, so with nothing on screen a commit stopped there would have no visible cause.
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
                // Not while hidden either — `visible` leaves `enabled` alone, and the pane's own press
                // (`WipPane.pressCommit`) reads this. Nor before the
                // identity's read has landed: `identityReady` starts true so the band stays quiet meanwhile — nor
                // after it failed, which is an answer too (`identityUnread`; git's words are in the command log's error
                // line, and the button comes back at the next read that lands).
                enabled: !block.composeHidden && block.repoTab.busyCount === 0
                         && block.repoTab.identityRead && !block.repoTab.identityUnread
                         && block.repoTab.identityReady
                         && block.worktree.conflictCount === 0 && !block.amendEmpties
                         && (block.onStandingMessage
                             || (msgEditor.subjectText.trim() !== ""
                                 && (block.amending || block.worktree.stagedCount > 0
                                     || block.standingSubject !== "")))
                // One click even when warning: a confirmation on a daily operation gets pressed unread (デザイン規約
                // §可否・警告の出し場所).
                onActivated: block.commitClicked()
                // Why it cannot be pressed. The line-ending warning is the card's (`WipPane.commitCardAsked`), and only
                // on a button that can be pressed.
                readonly property TipAsk commitTip: TipAsk {
                    target: commitButton
                    asked: (commitHover.containsMouse || block.commitPointedAt) && !commitButton.enabled
                    words: !block.repoTab.identityRead
                           ? Words.otherCommandRunning
                           : block.repoTab.identityUnread
                           ? qsTr("The name and email for commits could not be read")
                           : !block.repoTab.identityReady
                           ? qsTr("No name or email set for commits")
                           : block.worktree.conflictCount > 0
                           ? qsTr("Files are still waiting on a decision")
                           : msgEditor.subjectText.trim() === ""
                           ? qsTr("A commit needs a summary")
                           : block.amendEmpties
                           ? Words.amendEmpties
                           : qsTr("Stage something to commit")
                }
                MouseArea {
                    id: commitHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
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
