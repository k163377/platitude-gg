pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Everything under the WIP pane's file list, pinned to the pane's bottom: the message pair, the amend row, the
// button that writes the commit, and the way out of an operation git stopped in.
//
// A surface of its own that scrolls when the pane is too short to hold it. What is in here keeps its height by
// construction — the editor, the commit button, the exit card a stopped operation puts up — so the list was the
// only thing that could give, and past zero the rest was simply laid out below the pane's own edge (measured
// 2026-08-09: in a 320px window a stopped rebase drew `--continue` and `--skip` and left `--quit` and `--abort`
// under the window, with nothing to scroll to reach them). The window's floor cannot answer that on its own: the
// card comes and goes with what git is in the middle of, and a window that grew itself because a rebase stopped
// would be a stranger thing than a pane that scrolls (規約 §窓の床).
//
// The root is the `Flickable` it was inside the pane, so the layout attachments stay where they were and the item
// tree is the same depth it was (rules-refs/structure.md).
Flickable {
    id: block

    /// What the pane hands down: the tree this commit is made of, and the room it is given.
    required property var repoTab
    required property var workTree
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

    /// Back up to the pane: the tick, the press, and the moment the button's own warning could have changed
    /// (the card it opens is placed from the pane, which is the one thing that knows where the rows are).
    signal amendToggled(bool on)
    signal commitClicked()
    signal cardAsked()

    /// What the block would take with nothing in its way — the pane holds it against `blockRoom`.
    readonly property real wants: blockCol.implicitHeight

    // -- what the pane reads back out of here --
    readonly property alias subjectText: msgEditor.subjectText
    readonly property alias bodyText: msgEditor.bodyText
    readonly property alias descriptionColor: msgEditor.descriptionColor
    readonly property alias descriptionFocused: msgEditor.descriptionFocused
    readonly property alias descRefuses: msgEditor.descRefuses
    readonly property alias descPoint: msgEditor.descPoint
    readonly property alias descGrips: msgEditor.descGrips
    readonly property alias descHeight: msgEditor.descHeight
    readonly property alias descWants: msgEditor.descWants
    readonly property alias descCap: msgEditor.descCap
    readonly property alias descListRows: msgEditor.descListRows
    readonly property alias blockScrolls: msgEditor.blockScrolls
    readonly property alias descKeeps: msgEditor.descKeeps
    /// Whether the amend box is ticked, and whether the author box is — the two the page sets and reads.
    readonly property alias amendChecked: amendBox.checked
    readonly property alias resetAuthor: authorBox.checked
    /// The commit button's own two, for the card the pane places on it: whether it is warning at all, and
    /// whether a pointer is on it. `commitSeat` is the button itself, which the pane measures from.
    readonly property alias commitWarned: commitButton.eolWarned
    readonly property alias commitPointed: commitHover.containsMouse
    readonly property alias commitSeat: commitButton
    readonly property alias commitEnabled: commitButton.enabled
    readonly property alias signingTipShown: commitButton.phraseSignatureTipShown

    // -- and what it does on the pane's word --
    function setMessage(subject, body) { msgEditor.setMessage(subject, body) }
    function setAmendChecked(on) { amendBox.checked = on }
    function setResetAuthorChecked(on) { authorBox.checked = on }
    function focusDescription() { msgEditor.focusDescription() }
    function growDescription(dy) { msgEditor.growDescription(dy) }
    function pullDescriptionPast(down) { msgEditor.pullDescriptionPast(down) }
    function completeOpExit(code) { return opExitCard.completeOpExit(code) }
    function offersOpExit(code) { return opExitCard.offersOpExit(code) }
    /// Moves the block by a wheel a box on it could not use. The boxes cover most of the block, so without this
    /// the surface they stand on has no way to be reached by wheel at all (2026-08-09 ユーザー報告: the
    /// description box's own scrolling swallowed it and the block would not go down).
    function rollBlock(pixels) {
        const max = Math.max(0, block.contentHeight - block.height)
        // Taken away, not added: `pixels` is how far the wheel wanted the content to travel, and content
        // travels against `contentY` — the same subtraction the box makes on its own text. Added, the block
        // went the other way, which is a wheel that scrolls up and drags the surface down under it.
        block.contentY = Math.max(0, Math.min(max, block.contentY - pixels))
    }

    contentWidth: width
    contentHeight: blockCol.implicitHeight
    clip: true
    // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: PaneScrollBar {}
    ColumnLayout {
        id: blockCol
        width: block.width
        spacing: 0

        // Message editor — identical shape in commit details, amend and new-commit creation, inset
        // the same way (see DetailsPane) so switching modes doesn't move the box.
        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.spaceXs
            // The gutter this block's scroll bar is drawn in — see DetailsPane, which insets the same pair the
            // same way.
            Layout.rightMargin: Theme.navBarGutter
            spacing: Theme.spaceXs
            // The pair is one block of one height, in the same component the details pane reads messages in (デザイン規約
            // §コミットメッセージの 2 つの枠).
            MessageEditor {
                id: msgEditor
                Layout.fillWidth: true
                Layout.preferredHeight: msgEditor.pairHeight
                standingSubject: block.standingSubject
                standingBody: block.standingBody
                listHeight: block.listHeight
                blockRoom: block.blockRoom
                blockHeight: blockCol.implicitHeight
                onWheelPastEnd: pixels => block.rollBlock(pixels)
            }
            // Amend replaces the newest commit instead of adding one, so it starts from that commit's message
            // rather than an empty editor.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // Nothing to amend before the first commit: the row goes rather than standing down, since a
                // repository with no history has no "last commit" the reader is being kept from.
                visible: block.workTree.headOid !== ""
                AppCheckBox {
                    id: amendBox
                    text: qsTr("Amend the last commit")
                    font.pixelSize: Theme.fontSm
                    onToggled: block.amendToggled(checked)
                }
                Item { Layout.fillWidth: true }
                // Said, not asked: rewriting a pushed commit is undone by a switch or a reset, so the amend goes
                // ahead and this tag is all the warning it gets.
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
            // git records who committed, but leaves the author alone: an amend of someone else's commit — or of
            // one's own made under a different name — keeps the name it had. Offered only where the two
            // identities actually differ, and unchecked again whenever it goes away.
            //
            // **On its own line, under the box it depends on.** Beside it there is no room: the three words the
            // amend row can be carrying at once want about 392px between them, which is past the 400 this pane
            // opens at and well past the 300 it can be dragged to — and none of the three elides, so what went
            // over came off the tag at the end. Stacking is also how a dependent option reads: one under the box
            // it hangs off, at this spacing and with no indent.
            AppCheckBox {
                id: authorBox
                visible: block.amending && block.repoTab.headAuthorDiffers
                text: qsTr("Make me the author")
                font.pixelSize: Theme.fontSm
                onVisibleChanged: if (!visible) checked = false
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Replaces the author %1 <%2> with you, dated now")
                              .arg(block.repoTab.headAuthorName)
                              .arg(block.repoTab.headAuthorEmail)
            }
            // The framed button the rest of the app uses, at the size this pane needs. Its frame, its tone and
            // its `!` are the ones the toolbar's buttons already wear, so the state this button can be in is
            // said in the vocabulary someone has read elsewhere — rather than in a filled face that only this
            // one button has.
            ActionButton {
                id: commitButton
                Layout.fillWidth: true
                // **The one button in the pane that is the pane's own action**, and the only one told to fill a
                // width — so it is measured at the step above the controls beside it (`fontLg`, the size a
                // commit's summary is typed at) in the band the toolbar's own row stands at. A control-height
                // box around this wording read as a strip rather than as the thing being reached for
                // (2026-08-18 報告).
                font.pixelSize: Theme.fontLg
                implicitHeight: Theme.toolbarHeight
                /// Something staged says its line endings changed, so the commit is about to carry it. **Only
                /// the index counts** — a file marked on its working-tree side is not in this commit
                /// (`session::EolMark::staged`).
                readonly property bool eolWarned:
                    block.workTree.eolStagedCount > 0 && commitButton.enabled
                // The other side a resting pointer's card can change under: the frame and the `!` are bindings and
                // follow this on their own, the card has to be asked (`settleCommitCard`).
                onEolWarnedChanged: block.cardAsked()
                // **No mark beside the word.** The label already names the command and the branch it lands on,
                // which is two things to read; a tick in front of them says nothing a reader did not already
                // have, and the one mark this button does need — the `!` — has to stand out from it
                // (2026-08-18 ユーザー判断).
                centred: true
                // **The word stays plain in both states.** A coloured word is what `Remove` and a stopped fetch
                // wear, and both of those are held rather than clicked; this one is a click either way, and
                // borrowing their colour for the word would borrow the gesture with it. The frame and the mark
                // carry the state instead.
                tone: Theme.textPrimary
                // **The frame goes with the words.** A toolbar button is measured to its content, so a frame
                // left bright around dimmed words still reads as a small live thing; one this wide reads as a
                // live button with grey words in it.
                frameColor: !commitButton.enabled ? Theme.borderDefault
                            : commitButton.eolWarned ? Theme.warning : Theme.accent
                // The mark the button family already puts at the end of its own word when the thing it does
                // needs reading first.
                alert: commitButton.eolWarned
                alertTone: Theme.warning
                alertTight: true
                // **The command, and where it lands.** Both ends of the phrase are things git spells, so both
                // wear the chip that says so (デザイン規約 §git 用語のコード表記); the words between them are
                // the app's own and stay plain and translatable. The branch takes the accent, because the one
                // thing a reader can be wrong about here is which branch they are on — it is a switch away
                // from being another.
                phraseHead: block.amending ? "commit --amend" : "commit"
                phraseTail: block.commitTarget
                phraseTailTint: commitButton.enabled ? Theme.accent : Theme.textMuted
                // **And whom it will be attributed to.** The face ends the sentence the button is: the command,
                // what goes, where it lands, by whom — which is the whole of what a commit records, and the
                // one question the editor no longer has a row of its own for (デザイン規約 §アバターを与える).
                // `+N` when the message credits others, the same mark the graph's chips and the details pane's
                // credit line use; it follows the text being typed, so it appears as the trailer is written.
                phraseFace: block.repoTab.authorAvatar
                phraseFaceUrl: block.repoTab.authorAvatarUrl
                phraseMates: block.mateCount
                // A tick on that face when signing is on, never the green one: green is git's word that a
                // signature held, and nothing has been signed yet (規約 §署名の表示). What it is worth saying
                // is that a press may put a passphrase prompt on screen — the passphrase is the agent's, and a
                // commit that stops there has no reason on screen for having stopped.
                phraseSignature: block.repoTab.signsCommits ? "signed" : ""
                phraseSignaturePointedAt: block.signingPointedAt
                phraseSignatureTip: block.repoTab.signingFormat === "ssh"
                                    ? qsTr("Signed with your ssh key")
                                    : block.repoTab.signingFormat === "x509"
                                      ? qsTr("Signed with your x509 certificate")
                                      : qsTr("Signed with your gpg key")
                // **What goes, and where it lands.** The count is of files rather than of anything git would
                // call a change: it is the number the list above is showing, so the button and the list agree
                // without the reader converting between them. An amend with nothing staged is a message-only
                // rewrite and says so by naming no count at all.
                // The count stands apart from the words so the words can give way without taking it: how many
                // files go is the one number here, and it costs nothing to keep.
                phraseCount: block.workTree.stagedCount === 0
                             ? "" : block.workTree.stagedCount.toString()
                // With no count there is no noun for it to count: a message-only amend and a merge whose
                // resolution records nothing both press with nothing staged, and `commit files to main` names a
                // quantity that is not there.
                text: block.workTree.stagedCount === 0 ? qsTr("to")
                      : block.workTree.stagedCount === 1 ? qsTr("file to") : qsTr("files to")
                // An amend can stand on its own (message only); a new commit needs staged content and a
                // summary, and git needs an identity to attribute either one to.
                //
                // A file still waiting on a decision stops it: git will not write a commit over an index that
                // holds unmerged paths, whatever is staged beside them (デザイン規約 §可否・警告の出し場所).
                //
                // **A stopped merge asks for neither of the other two.** It carries its own message, so an
                // empty box is not an empty commit message; and a merge whose resolution records nothing at
                // all still has to be finished, so nothing staged is not nothing to do — git writes the empty
                // merge commit either way (実測 2.55). This seat is the whole way out of a merge
                // (§進行中の操作から出る), and a way out that will not press is not one.
                enabled: block.repoTab.busyCount === 0 && block.repoTab.identityReady
                         && block.workTree.conflictCount === 0
                         && (block.onStandingMessage
                             || (msgEditor.subjectText.trim() !== ""
                                 && (block.amending || block.workTree.stagedCount > 0
                                     || block.standingSubject !== "")))
                // **Still one click.** Committing is a daily operation and a confirmation on a daily operation
                // becomes something people press without reading, which spends the effect where it is really
                // needed (デザイン規約 §可否・警告の出し場所). The frame, the mark and the hover say what is in
                // it; the decision stays the reader's.
                onActivated: block.commitClicked()
                // Why it cannot be pressed. The other thing this button has to say — that the index carries a
                // line-ending change — is said by the card `settleCommitCard` opens instead, and the two cannot
                // both be true: the warning wants a button that can be pressed.
                ToolTip.visible: commitHover.containsMouse && !enabled
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: !block.repoTab.identityReady
                              ? qsTr("No name or email set for commits")
                              : block.workTree.conflictCount > 0
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
            // The way out of a stopped operation, under the button that finishes things (`OpExitCard`) — the file
            // list moves down to make room for it rather than the rows covering the list.
            OpExitCard {
                id: opExitCard
                Layout.fillWidth: true
                Layout.topMargin: Theme.spaceXs
                repoTab: block.repoTab
                workTree: block.workTree
            }
        }
    }
}
