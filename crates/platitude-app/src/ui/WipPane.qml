pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Right pane, working-tree (WIP) mode: the commit editor pinned on top
// of the grouped changed-file list, with stage/unstage affordances.
// The editor's text is owned here; the page drives it through
// setMessage / clearMessage (amend prefill, post-commit reset) and
// decides what a commit click means.
ColumnLayout {
    id: wipPane

    required property var repoTab
    required property var workTree
    required property var worktreeModel
    // Mirrored page state: whether the editor is in amend mode and
    // whether HEAD is already on a remote (shows the warning tag).
    property bool amending: false
    property bool headPublished: false

    signal amendToggled(bool on)
    signal commitClicked()
    signal fileActivated(string bucket, string path, string origPath)
    /// The stash card's Stash button: message / include untracked /
    /// keep index / staged only.
    signal stashSubmitted(string message, bool untracked, bool keepIndex, bool stagedOnly)
    /// Right-click on a file row; the page owns the menu because
    /// delegates are recycled out from under an open popup. `origPath` is
    /// where a rename came from ("" otherwise).
    signal fileMenuRequested(string bucket, string path, string origPath)

    /// Automation: run one of the stopped operation's held rows to its
    /// end, named by its flag.
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

    // ---- which rows are chosen -------------------------------------
    // Several files at once, the way a file list is used to being asked:
    // a plain click takes one, Ctrl adds or removes, Shift reaches from
    // the last one clicked. Held here, keyed `<bucket>:<path>`, because a
    // delegate is recycled the moment its row scrolls off.
    property var chosenKeys: ({})
    property int chosenCount: 0
    /// The row the next Shift-click reaches from.
    property int anchorRow: -1
    function isChosen(bucket, path) {
        return wipPane.chosenKeys[bucket + ":" + path] === true
    }
    /// Rows in list order, so a caller can act on what was chosen.
    function chosenRows() {
        const out = []
        for (let i = 0; i < wipList.count; i++) {
            const row = wipList.itemAtIndex(i)
            if (row && !row.folder && wipPane.isChosen(row.bucket, row.fullName))
                out.push(row)
        }
        return out
    }
    /// Makes one row the whole of the choice — what a plain click does,
    /// and what a right-click outside the choice does before opening the
    /// menu (the menu acts on what is highlighted).
    function chooseOnly(bucket, path) {
        wipPane.applyClick(bucket, path, Qt.NoModifier)
    }
    function clearChoice() {
        wipPane.chosenKeys = ({})
        wipPane.chosenCount = 0
        wipPane.anchorRow = -1
    }
    /// Applies a click to the choice. Returns whether the diff should
    /// follow it: adding to a choice is about the choice, not about which
    /// file is being read.
    function applyClick(bucket, path, modifiers) {
        const key = bucket + ":" + path
        const row = wipPane.rowIndexOf(key)
        if (modifiers & Qt.ShiftModifier && wipPane.anchorRow >= 0) {
            wipPane.chooseRange(wipPane.anchorRow, row)
            return false
        }
        if (modifiers & Qt.ControlModifier) {
            // A fresh object every time: the rows follow this property,
            // and assigning the same one back changes nothing to follow.
            const next = ({})
            for (const k in wipPane.chosenKeys)
                next[k] = true
            if (next[key] === true)
                delete next[key]
            else
                next[key] = true
            wipPane.chosenKeys = next
            wipPane.chosenCount = Object.keys(next).length
            wipPane.anchorRow = row
            return false
        }
        const only = ({})
        only[key] = true
        wipPane.chosenKeys = only
        wipPane.chosenCount = 1
        wipPane.anchorRow = row
        return true
    }
    function rowIndexOf(key) {
        for (let i = 0; i < wipList.count; i++) {
            const row = wipList.itemAtIndex(i)
            if (row && !row.folder && row.bucket + ":" + row.fullName === key)
                return i
        }
        return -1
    }
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const lo = Math.min(from, to)
        const hi = Math.max(from, to)
        const next = ({})
        for (let i = lo; i <= hi; i++) {
            const row = wipList.itemAtIndex(i)
            if (row && !row.folder)
                next[row.bucket + ":" + row.fullName] = true
        }
        wipPane.chosenKeys = next
        wipPane.chosenCount = Object.keys(next).length
    }

    // ---- the marks that come out together ---------------------------
    // A press on one row's `+` moves every highlighted row that can go the
    // same way, so the marks on those rows come out as soon as the pointer
    // reaches one of them: what a press is about to move is seen before it
    // is pressed (デザイン規約 §その他の操作). Held here, keyed the way the
    // choice is, because a delegate is recycled the moment its row scrolls
    // off. Automation writes it directly — hover cannot be injected.
    property string stageHotKey: ""
    function showStageTools(bucket, path) {
        wipPane.stageHotKey = bucket + ":" + path
    }
    /// Names the file whose line-ending sentence the hover should carry.
    /// The empty string clears it. Automation writes it directly, the same
    /// way it writes `showStageTools` — hover cannot be injected.
    function pointEol(path) {
        wipPane.worktreeModel.pointEol(path)
        if (path === "") {
            eolCard.close()
            return
        }
        // Under the row rather than under the pointer: these rows are a
        // pane wide at most, so the two are never far apart, and a card
        // placed from the row lands in the same place whether a pointer
        // or the automation named it.
        const row = wipPane.rowFor(path)
        if (row === null)
            return
        const at = row.mapToItem(wipPane, 0, row.height)
        eolCard.path = path
        eolCard.notice = wipPane.pointedEolText
        eolCard.x = at.x + Theme.spaceMd
        eolCard.y = at.y
        eolCard.open()
    }
    /// Puts the commit button's card out without a pointer, the way the
    /// rows' is put out — hover cannot be injected.
    property bool pointAtCommit: false
    onPointAtCommitChanged: wipPane.settleCommitCard()
    function settleCommitCard() {
        if (!commitButton.eolWarned
                || !(commitHover.containsMouse || wipPane.pointAtCommit)) {
            if (eolCard.path === "")
                eolCard.close()
            return
        }
        // No one file to name: the button speaks for the whole index, and
        // so has to hold for all four cases at once. **Not "change"** —
        // only one of them is a change. A new file has nothing to have
        // changed from, a mixed one is a file disagreeing with itself, and
        // a file that never had an ending has only gained its first.
        //
        // **`may`, and it is doing work.** Two of the four are read off a
        // sample of the neighbouring files, so the app does not know they
        // are problems — a new file deliberately written with the other
        // ending is not one. Said flatly, the summary would decide what
        // each row's own card is careful not to.
        eolCard.path = ""
        eolCard.notice = wipPane.workTree.eolStagedCount === 1
            ? qsTr("1 staged file may have line-ending problems")
            : qsTr("%1 staged files may have line-ending problems")
              .arg(wipPane.workTree.eolStagedCount)
        const at = commitButton.mapToItem(wipPane, 0, commitButton.height)
        eolCard.x = at.x
        eolCard.y = at.y + Theme.spaceXs
        eolCard.open()
    }
    // The one card both hovers open: only one pointer, so only one of them
    // is ever out. Owned here rather than by a row, which is recycled the
    // moment it scrolls off (app-ui.md).
    EolHoverCard {
        id: eolCard
    }
    /// The card itself is up. Read by the headless runs — reporting what
    /// asked for it would go green with the wiring cut.
    readonly property bool eolCardOpen: eolCard.opened
    /// What the named row is saying, for the headless report to read.
    readonly property string pointedEolPath: wipPane.worktreeModel.pointedEolPath
    readonly property string pointedEolText:
        wipPane.worktreeModel.pointedEolKind !== ""
        ? Words.lineEndings(wipPane.worktreeModel.pointedEolKind,
                            wipPane.worktreeModel.pointedEolFrom,
                            wipPane.worktreeModel.pointedEolTo,
                            wipPane.worktreeModel.pointedEolLines,
                            wipPane.worktreeModel.pointedEolScope,
                            wipPane.worktreeModel.pointedEolExt)
        : ""
    /// Whether a row should put its mark out because the pointer is on the
    /// mark of another row that would move with it. Only rows on the same
    /// side answer: `+` stages what is not staged, `−` takes back what is.
    function stagePeerOf(bucket, path) {
        const key = bucket + ":" + path
        if (wipPane.stageHotKey === "")
            return false
        // The row the pointer is actually on answers too: under a real
        // pointer it is already showing its mark, and this is the only way
        // a headless run can put that mark on screen (hover cannot be
        // injected — verify-ui スキル).
        if (wipPane.stageHotKey === key)
            return true
        if (!wipPane.isChosen(bucket, path))
            return false
        const cut = wipPane.stageHotKey.indexOf(":")
        const hotBucket = wipPane.stageHotKey.substring(0, cut)
        if (!wipPane.isChosen(hotBucket, wipPane.stageHotKey.substring(cut + 1)))
            return false
        return (hotBucket === "staged") === (bucket === "staged")
    }
    /// The highlighted rows that would move with a press on `bucket`'s
    /// side — the whole of what one press takes. A row that was not
    /// highlighted takes only itself.
    function stageTargets(bucket, path) {
        if (!wipPane.isChosen(bucket, path))
            return [path]
        const out = []
        const rows = wipPane.chosenRows()
        for (let i = 0; i < rows.length; i++)
            if ((rows[i].bucket === "staged") === (bucket === "staged"))
                out.push(rows[i].fullName)
        return out.length > 0 ? out : [path]
    }

    /// The row at an index — automation, like `rowFor` below.
    function rowAt(index) {
        return wipList.itemAtIndex(index)
    }
    /// The row a path is on — for the automation hooks, which enter a
    /// click where the row itself enters it. App code goes through the
    /// signals.
    function rowFor(path) {
        for (let i = 0; i < wipList.count; i++) {
            const row = wipList.itemAtIndex(i)
            if (row && row.fullName === path)
                return row
        }
        return null
    }

    readonly property string subjectText: wipSubject.text
    readonly property string bodyText: wipBody.text
    // Whether the amend should also put the current identity on the
    // commit it replaces (git keeps the original author otherwise).
    readonly property bool resetAuthor: authorBox.checked
    function setMessage(subject, body) {
        wipSubject.text = subject
        wipBody.text = body
    }
    function clearMessage() {
        wipSubject.text = ""
        wipBody.text = ""
    }
    function setAmendChecked(on) {
        amendBox.checked = on
    }
    function setResetAuthorChecked(on) {
        authorBox.checked = on
    }
    /// Smoke hook: the caret in the description box, the way a click in
    /// it puts it there (see DetailsPane — same box, same reason).
    function focusDescription() {
        wipBody.takeCaret()
    }
    readonly property color descriptionColor: wipBody.textColor
    readonly property bool descriptionFocused: wipBody.focused

    // -- what this pane lends the description box --
    //
    // Same bound as the details pane, read off what this pane has under
    // the box instead: the file list is the only thing that gives, and
    // the checkboxes, the commit button and the exit card keep their own
    // height by construction, so two rows of list is the whole of it
    // (デザイン規約 §コミットメッセージの 2 つの枠).
    readonly property real descRoom:
        Math.max(0, wipList.height - 2 * Theme.rowHeight)
    /// What the pane needs back — the list bottoms out at zero while the
    /// column keeps growing, so the overflow past this pane's own edge is
    /// the rest of the answer (see DetailsPane).
    readonly property real descOwed:
        Math.max(0, 2 * Theme.rowHeight - wipList.height)
        + Math.max(0, wipPane.implicitHeight - wipPane.height)
    // -- smoke hooks, forwarded to the box --
    function growDescription(dy) { wipBody.grow(dy) }
    readonly property bool descGrips: wipBody.grips
    readonly property real descHeight: wipBody.boxHeight
    readonly property real descWants: wipBody.wants
    readonly property real descCap: wipBody.cap
    readonly property int descListRows:
        Math.round(wipList.height / Theme.rowHeight)
    /// Whether what sits under the box is still inside the pane. Here it
    /// is the commit button that would go under the window's own footer,
    /// and that shot frames like one where it did not.
    readonly property bool descKeeps:
        commitButton.mapToItem(wipPane, 0, commitButton.height).y
        <= wipPane.height

    // The stash options open right under the button that asks for them,
    // as a mode of this pane rather than a window over it (デザイン規約
    // §可否・警告の出し場所: ダイアログでなければ成立しない UI ではない).
    // A stash destroys nothing, so the card chooses what goes; it never
    // asks whether it may.
    property bool stashPanelShown: false
    function openStashPanel() {
        stashName.text = ""
        stashUntracked.checked = true
        stashKeepIndex.checked = false
        stashStagedOnly.checked = false
        wipPane.stashPanelShown = true
        stashName.forceActiveFocus()
    }
    function closeStashPanel() {
        wipPane.stashPanelShown = false
    }
    /// The click path onto "Only the staged changes" (the page's smoke
    /// hook): setting `checked` skips `toggled`, a click does not.
    function stashClickStagedOnly() {
        stashStagedOnly.toggle()
        stashStagedOnly.toggled()
    }
    function stashApply() {
        // What is sent is what the boxes show — a box ruled out by the
        // staged-only choice does not smuggle its old tick through
        // (git refuses --staged together with --include-untracked).
        const stagedOnly = stashStagedOnly.checked
        wipPane.stashSubmitted(stashName.text,
                               !stagedOnly && stashUntracked.checked,
                               !stagedOnly && stashKeepIndex.checked,
                               stagedOnly)
        wipPane.closeStashPanel()
    }

    spacing: 0

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        // The hairline every pane header closes with (see PaneHeader).
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.borderWidth
            color: Theme.borderSubtle
            z: 1
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: qsTr("UNCOMMITTED CHANGES (%1)").arg(wipPane.worktreeModel.total)
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            Item { Layout.fillWidth: true }
            // Everything uncommitted, set aside in one entry. Opens the
            // options card below; nothing is thrown away, so nothing
            // asks beyond the card itself.
            HoverToolButton {
                text: qsTr("Stash…")
                font.pixelSize: Theme.fontSm
                enabled: wipPane.repoTab.busyCount === 0
                         && wipPane.worktreeModel.total > 0
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Set these changes aside for later")
                onClicked: {
                    if (wipPane.stashPanelShown)
                        wipPane.closeStashPanel()
                    else
                        wipPane.openStashPanel()
                }
            }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Tree view")
                onClicked: wipPane.worktreeModel.setTreeView(true)
                contentItem: NavIcon {
                    kind: "hier"
                    // See the same pair in DetailsPane: the unchosen half
                    // is what switches, so it dims rather than mutes.
                    tint: wipPane.worktreeModel.treeView ? Theme.accent
                                                         : Theme.accentDim
                }
            }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Paths view")
                onClicked: wipPane.worktreeModel.setTreeView(false)
                contentItem: NavIcon {
                    kind: "list"
                    tint: wipPane.worktreeModel.treeView ? Theme.accentDim
                                                         : Theme.accent
                }
            }
        }
    }

    // Stash options card, under the button that opened it. The same
    // framed-card inset rhythm as the editor below (帯 → 枠 → 枠 = one
    // even spaceXs step).
    Rectangle {
        visible: wipPane.stashPanelShown
        Layout.fillWidth: true
        Layout.leftMargin: Theme.spaceSm
        Layout.rightMargin: Theme.spaceSm
        Layout.topMargin: Theme.spaceXs
        implicitHeight: stashCol.implicitHeight + 2 * Theme.spaceSm
        color: Theme.bgBase
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
        ColumnLayout {
            id: stashCol
            anchors.fill: parent
            anchors.margins: Theme.spaceSm
            spacing: Theme.spaceXs
            FormField {
                id: stashName
                Layout.fillWidth: true
                placeholderText: qsTr("What this is, for finding it later")
                onAccepted: wipPane.stashApply()
                Keys.onEscapePressed: wipPane.closeStashPanel()
            }
            CheckBox {
                id: stashUntracked
                // A stash that leaves new files behind is the surprise
                // most often reported to other git GUIs, so this starts
                // on — the same choice the switch dialog makes.
                text: qsTr("Include files git is not tracking yet")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                enabled: !stashStagedOnly.checked
            }
            CheckBox {
                id: stashKeepIndex
                text: qsTr("Leave the staged changes staged")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                enabled: !stashStagedOnly.checked
            }
            CheckBox {
                id: stashStagedOnly
                text: qsTr("Only the staged changes")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                // git refuses it together with untracked files, and
                // cannot do it at all for a file changed on both sides:
                // it writes the entry, then fails to clear the tree and
                // leaves the entry behind with nothing else done
                // (measured). Refusing first is the only way that does
                // not surprise.
                enabled: wipPane.workTree.partiallyStagedCount === 0
                // A disabled box keeps its tick, and what is sent is
                // what the boxes show — so the boxes this tick rules
                // out are unticked, not just greyed with their ticks
                // still live.
                onToggled: {
                    if (checked) {
                        stashUntracked.checked = false
                        stashKeepIndex.checked = false
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                visible: wipPane.workTree.partiallyStagedCount > 0
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("%n file(s) are changed both in the staging area "
                           + "and on disk. git cannot take those apart, so "
                           + "the staged changes cannot go on their own.", "",
                           wipPane.workTree.partiallyStagedCount)
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                Item { Layout.fillWidth: true }
                HoverToolButton {
                    text: qsTr("Cancel")
                    font.pixelSize: Theme.fontSm
                    onClicked: wipPane.closeStashPanel()
                }
                HoverButton {
                    highlighted: true
                    text: qsTr("Stash")
                    enabled: wipPane.repoTab.busyCount === 0
                    onClicked: wipPane.stashApply()
                }
            }
        }
    }
    // Message editor pinned on top — identical shape in commit details,
    // amend and new-commit creation, inset the same way (see
    // DetailsPane) so switching modes doesn't move the box.
    ColumnLayout {
        Layout.fillWidth: true
        Layout.margins: Theme.spaceSm
        Layout.topMargin: Theme.spaceXs
        Layout.bottomMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Rectangle {
            Layout.fillWidth: true
            // Capped and scrolled like the description box below it, and
            // like the details pane's pair: a summary somebody pastes a
            // paragraph into otherwise grows until the file list has no
            // pane left to be in.
            Layout.preferredHeight: Math.min(wipSubject.implicitHeight
                                             + Theme.spaceSm,
                                             Theme.messageMaxHeight)
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                Component.onCompleted:
                    contentItem.boundsBehavior = Flickable.StopAtBounds
                SummaryArea {
                    id: wipSubject
                    placeholderText: qsTr("Commit summary")
                }
            }
        }
        // Two lines tall from the start, and pulled open by the corner
        // for a long one — the same component the details pane reads
        // messages in.
        DescriptionBox {
            id: wipBody
            placeholderText: qsTr("Description")
            room: wipPane.descRoom
            owed: wipPane.descOwed
        }
        // Amend replaces the newest commit instead of adding one, so it
        // starts from that commit's message rather than an empty editor.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            CheckBox {
                id: amendBox
                text: qsTr("Amend the last commit")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                onToggled: wipPane.amendToggled(checked)
            }
            // git records who committed, but leaves the author alone: an
            // amend of someone else's commit — or of one's own made under
            // a different name — keeps the name it had. Offered only
            // where the two identities actually differ, and unchecked
            // again whenever it goes away.
            CheckBox {
                id: authorBox
                visible: wipPane.amending && wipPane.repoTab.headAuthorDiffers
                text: qsTr("Make me the author")
                font.pixelSize: Theme.fontSm
                implicitHeight: Theme.controlHeight
                onVisibleChanged: if (!visible) checked = false
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Replaces the author %1 <%2> with you, "
                                   + "dated now")
                              .arg(wipPane.repoTab.headAuthorName)
                              .arg(wipPane.repoTab.headAuthorEmail)
            }
            Item { Layout.fillWidth: true }
            // What this button is about to do beyond committing: git will
            // ask an agent for the key, and that agent may put a
            // passphrase prompt on screen. Said only when signing is on,
            // and in the plain text colour — it is a fact, not a warning.
            // It gives the slot up to the pushed-amend warning: one tag
            // fits here, and a warning outranks a fact.
            Label {
                visible: wipPane.repoTab.signsCommits
                         && !(wipPane.amending && wipPane.headPublished)
                text: qsTr("will be signed")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                ToolTip.visible: signingHover.containsMouse
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: wipPane.repoTab.signingFormat === "ssh"
                              ? qsTr("Signed with your ssh key")
                              : wipPane.repoTab.signingFormat === "x509"
                                ? qsTr("Signed with your x509 certificate")
                                : qsTr("Signed with your gpg key")
                MouseArea {
                    id: signingHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
            // Said, not asked: rewriting a pushed commit is undone by a
            // switch or a reset, so the amend goes ahead and this tag
            // is all the warning it gets.
            Label {
                visible: wipPane.amending && wipPane.headPublished
                text: qsTr("already pushed")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                ToolTip.visible: amendPushedHover.containsMouse
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Already on a remote — anyone who has it "
                                   + "will be out of step")
                MouseArea {
                    id: amendPushedHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
        }
        // The framed button the rest of the app uses, at the size this pane
        // needs. Its frame, its tone and its `!` are the ones the toolbar's
        // buttons already wear, so the state this button can be in is said
        // in the vocabulary someone has read elsewhere — rather than in a
        // filled face that only this one button has.
        ActionButton {
            id: commitButton
            Layout.fillWidth: true
            implicitHeight: Theme.controlHeight
            /// Something staged says its line endings changed, so the
            /// commit is about to carry it. **Only the index counts** — a
            /// file marked on its working-tree side is not in this commit
            /// (`session::EolMark::staged`).
            readonly property bool eolWarned:
                wipPane.workTree.eolStagedCount > 0 && commitButton.enabled
            kind: "check"
            centred: true
            // **The word stays plain in both states.** A coloured word is
            // what `Remove` and a stopped fetch wear, and both of those are
            // held rather than clicked; this one is a click either way, and
            // borrowing their colour for the word would borrow the gesture
            // with it. The frame and the mark carry the state instead.
            tone: Theme.textPrimary
            // **The frame goes with the words.** A toolbar button is
            // measured to its content, so a frame left bright around dimmed
            // words still reads as a small live thing; one this wide reads
            // as a live button with grey words in it.
            frameColor: !commitButton.enabled ? Theme.borderDefault
                        : commitButton.eolWarned ? Theme.warning : Theme.accent
            // The mark the button family already puts at the end of its
            // own word when the thing it does needs reading first.
            alert: commitButton.eolWarned
            alertTone: Theme.warning
            alertTight: true
            text: wipPane.amending
                  ? qsTr("Amend commit (%1 staged)").arg(wipPane.workTree.stagedCount)
                  : qsTr("Commit changes (%1 staged)").arg(wipPane.workTree.stagedCount)
            // An amend can stand on its own (message only); a new commit
            // needs staged content and a summary, and git needs an
            // identity to attribute either one to.
            enabled: wipPane.repoTab.busyCount === 0
                     && wipPane.repoTab.identityReady
                     && wipSubject.text.trim() !== ""
                     && (wipPane.amending || wipPane.workTree.stagedCount > 0)
            // **Still one click.** Committing is a daily operation and a
            // confirmation on a daily operation becomes something people
            // press without reading, which spends the effect where it is
            // really needed (デザイン規約 §可否・警告の出し場所). The
            // frame, the mark and the hover say what is in it; the
            // decision stays the reader's.
            onActivated: wipPane.commitClicked()
            // Why it cannot be pressed. The other thing this button has to
            // say — that the index carries a line-ending change — is said
            // by the card `settleCommitCard` opens instead, and the two
            // cannot both be true: the warning wants a button that can be
            // pressed.
            ToolTip.visible: commitHover.containsMouse && !enabled
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: !wipPane.repoTab.identityReady
                          ? qsTr("No name or email set for commits")
                          : wipSubject.text.trim() === ""
                          ? qsTr("A commit needs a summary")
                          : qsTr("Stage something to commit")
            MouseArea {
                id: commitHover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
                onContainsMouseChanged: wipPane.settleCommitCard()
            }
        }
        // ==== 案 C: the way out of a stopped operation, built into ====
        // ==== the pane under the button that finishes things.     ====
        //
        // The commit button's seat is already "conclude this": during a
        // merge `--continue` is literally the commit. Standing rather
        // than dropped from a click — while an operation is stopped this
        // is the pane's business, and the file list moves down to make
        // room for it rather than the rows covering the list.
        Rectangle {
            id: opExitCard
            visible: wipPane.workTree.opText !== ""
            Layout.fillWidth: true
            Layout.topMargin: Theme.spaceXs
            implicitHeight: opExitCol.implicitHeight + 2 * Theme.spaceXs
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.warning
            border.width: Theme.borderWidth
            // The chip column and the mark's seat, shared by every row so
            // the sentences start on one x and the card reads down its
            // first letters — what `AppMenu` does for a menu's rows.
            // The mark's own seat, left on every row whether or not that
            // row is a held one — the words start on one x either way,
            // and the seat is the menu's exactly: narrower than the mark,
            // which overhangs it into the row padding on one side and the
            // word gap on the other (デザイン規約 §長押し — 語が払う
            // 字下げは印 1 個分より小さい).
            readonly property real holdIndent: Theme.iconMd - 2 * Theme.spaceXs
            /// Whether leaving the stopped commit out costs nothing.
            ///
            /// An interactive rebase stops on a commit that came out
            /// empty and names `--skip` as the way past it (measured —
            /// `an_interactive_rebase_stops_on_an_emptied_commit_and_names_skip`).
            /// Nothing is conflicted or staged while it stands there, and
            /// a clean tree under a stopped operation is what tells that
            /// stop from every other one this app can reach today.
            ///
            /// **Revisit when `edit` steps land** (full interactive
            /// rebase): an `edit` stop is clean too, and skipping one
            /// does lose the commit. Distinguishing them needs core to
            /// say why git stopped (P3-確認事項).
            readonly property bool skipIsFree:
                wipPane.workTree.conflictCount === 0
                && wipPane.workTree.stagedCount === 0
                && wipPane.workTree.unstagedCount === 0
            readonly property real codeColW: {
                let widest = 0
                for (let i = 0; i < opExitCol.children.length; i++) {
                    const row = opExitCol.children[i]
                    if (row && row.visible && row.codeColSeat !== undefined)
                        widest = Math.max(widest, row.codeColSeat)
                }
                return widest
            }
            ColumnLayout {
                id: opExitCol
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                spacing: 0
                Label {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.spaceXs
                    Layout.bottomMargin: Theme.spaceXs
                    // What is stopped, and how far it got. The count is
                    // the half a stopped rebase cannot say without it.
                    text: wipPane.workTree.opSteps > 0
                          ? qsTr("%1 — %2 of %3").arg(wipPane.workTree.opText)
                            .arg(wipPane.workTree.opStep).arg(wipPane.workTree.opSteps)
                          : wipPane.workTree.opText
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: Theme.warning
                }
                OpExitRow {
                    Layout.fillWidth: true
                    codeColW: opExitCard.codeColW
                    holdIndent: opExitCard.holdIndent
                    code: "--continue"
                    text: qsTr("Carry on with what is staged")
                    enabled: wipPane.repoTab.busyCount === 0
                             && wipPane.workTree.conflictCount === 0
                    onPicked: wipPane.repoTab.resolveOperation("continue")
                }
                // A merge steps through nothing: no commit to leave out,
                // and nowhere to stop stepping.
                OpExitRow {
                    Layout.fillWidth: true
                    codeColW: opExitCard.codeColW
                    holdIndent: opExitCard.holdIndent
                    code: "--skip"
                    text: qsTr("Leave this commit out")
                    // What the skip costs, said where a menu row says it.
                    note: opExitCard.skipIsFree ? qsTr("nothing in it") : ""
                    visible: wipPane.workTree.opStepping
                    enabled: wipPane.repoTab.busyCount === 0
                    // The mark is what says a row takes something away
                    // (デザイン規約 §長押し), so it goes when the row does
                    // not: git stops on a commit that came out empty and
                    // names `--skip` itself, and leaving that one out
                    // loses nothing. Everywhere else the commit is real
                    // and only the reflog holds it afterwards.
                    holdMs: opExitCard.skipIsFree ? 0 : Metrics.holdMs
                    onPicked: wipPane.repoTab.resolveOperation("skip")
                }
                OpExitRow {
                    Layout.fillWidth: true
                    codeColW: opExitCard.codeColW
                    holdIndent: opExitCard.holdIndent
                    code: "--quit"
                    text: qsTr("Stop stepping, keep the tree")
                    visible: wipPane.workTree.opStepping
                    enabled: wipPane.repoTab.busyCount === 0
                    onPicked: wipPane.repoTab.resolveOperation("quit")
                }
                OpExitRow {
                    Layout.fillWidth: true
                    codeColW: opExitCard.codeColW
                    holdIndent: opExitCard.holdIndent
                    code: "--abort"
                    text: qsTr("Undo it all and go back")
                    enabled: wipPane.repoTab.busyCount === 0
                    holdMs: Metrics.holdMs
                    onPicked: wipPane.repoTab.resolveOperation("abort")
                }
            }
        }
    }
    // No question bar over this list: what a file row throws away is held
    // down on the menu row that names it, where the hand already is
    // (デザイン規約 §長押し).
    ListView {
        id: wipList
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: wipPane.worktreeModel
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {}
        // GitKraken grouping: unstaged (incl. untracked) above, staged
        // below.
        section.property: "group"
        section.delegate: WipBucketHeader {
            listWidth: wipList.width
            repoTab: wipPane.repoTab
            workTree: wipPane.workTree
        }
        // The staged heading stands even with nothing under it: a section
        // with no rows has no heading, so the pane would otherwise never
        // name the place a staged file goes, and the band would grow in
        // under the hand at the first `+` (デザイン規約 §その他の操作). It
        // rides in the footer because staged is the last bucket — the seat
        // an empty one would take is exactly the end of the list.
        //
        // Only while something is uncommitted: on a clean tree the whole
        // list is empty and a lone `(0)` heads nothing.
        footer: WipBucketHeader {
            section: "staged"
            listWidth: wipList.width
            repoTab: wipPane.repoTab
            workTree: wipPane.workTree
            visible: wipPane.workTree.stagedCount === 0
                     && wipPane.worktreeModel.total > 0
        }
        delegate: NavItemDelegate {
            listWidth: wipList.width
            kindHint: "wt"
            showStage: true
            chosen: wipPane.isChosen(bucket, fullName)
            sideOurs: wipPane.workTree.sideOurs
            sideTheirs: wipPane.workTree.sideTheirs
            pointedEolPath: wipPane.worktreeModel.pointedEolPath
            pointedEolKind: wipPane.worktreeModel.pointedEolKind
            pointedEolFrom: wipPane.worktreeModel.pointedEolFrom
            pointedEolTo: wipPane.worktreeModel.pointedEolTo
            pointedEolLines: wipPane.worktreeModel.pointedEolLines
            pointedEolScope: wipPane.worktreeModel.pointedEolScope
            pointedEolExt: wipPane.worktreeModel.pointedEolExt
            onEolPointed: (path, on) => wipPane.pointEol(on ? path : "")
            onFileClicked: (bucket, path, origPath, modifiers) => {
                // Choosing rows is not reading one: only a plain click
                // moves the diff.
                if (wipPane.applyClick(bucket, path, modifiers))
                    wipPane.fileActivated(bucket, path, origPath)
            }
            onFileMenuRequested: (bucket, path, origPath) => {
                if (!wipPane.isChosen(bucket, path))
                    wipPane.chooseOnly(bucket, path)
                wipPane.fileMenuRequested(bucket, path, origPath)
            }
            onFolderClicked: key => wipPane.worktreeModel.toggleFolder(key)
            stagePeer: wipPane.stagePeerOf(bucket, fullName)
            onStageHovered: (bucket, path, on) => {
                if (on)
                    wipPane.showStageTools(bucket, path)
                else if (wipPane.stageHotKey === bucket + ":" + path)
                    wipPane.stageHotKey = ""
            }
            // One press, every highlighted row that can go the same way —
            // and one git command for the lot, whatever the count
            // (デザイン規約 §その他の操作).
            onStageClicked: (bucket, path) => {
                const paths = wipPane.stageTargets(bucket, path)
                if (paths.length === 1) {
                    if (bucket === "staged")
                        wipPane.repoTab.unstagePath(paths[0])
                    else
                        wipPane.repoTab.stagePath(paths[0])
                    return
                }
                wipPane.repoTab.beginPaths()
                for (let i = 0; i < paths.length; i++)
                    wipPane.repoTab.addPath(paths[i])
                if (bucket === "staged")
                    wipPane.repoTab.unstagePaths()
                else
                    wipPane.repoTab.stagePaths()
            }
        }
    }
}
