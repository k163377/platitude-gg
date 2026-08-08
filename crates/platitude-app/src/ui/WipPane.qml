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
            Layout.preferredHeight: wipSubject.implicitHeight + Theme.spaceSm
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            SummaryArea {
                id: wipSubject
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                placeholderText: qsTr("Commit summary")
            }
        }
        // Two lines tall from the start (matches the details pane's
        // description box).
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(Math.max(wipBody.implicitHeight,
                                                      2 * Theme.fontMdLine)
                                             + Theme.spaceSm, 120)
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderSubtle
            border.width: Theme.borderWidth
            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                // ScrollView keeps its Flickable private -- reach it
                // once it exists.
                Component.onCompleted:
                    contentItem.boundsBehavior = Flickable.StopAtBounds
                TextArea {
                    id: wipBody
                    wrapMode: TextArea.Wrap
                    placeholderText: qsTr("Description")
                    font.pixelSize: Theme.fontMd
                    color: Theme.textSecondary
                    background: null
                    padding: 0
                }
            }
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
        HoverButton {
            id: commitButton
            Layout.fillWidth: true
            highlighted: true
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
            onClicked: wipPane.commitClicked()
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
        section.delegate: Rectangle {
            id: bucketHeader
            required property string section
            /// An external merge tool holds the write queue until it is
            /// closed, which is the longest wait in the app and the only
            /// one with no upper bound.
            readonly property bool waitingForTool:
                bucketHeader.section === "conflicts"
                && wipPane.repoTab.busyOp === "mergetool"
            width: wipList.width
            height: Theme.rowHeight
            color: Theme.bgElevated
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceXs
                spacing: Theme.spaceXs
                Label {
                    text: bucketHeader.section === "staged"
                          ? qsTr("STAGED FILES (%1)").arg(wipPane.workTree.stagedCount)
                          : bucketHeader.section === "unstaged"
                          ? qsTr("UNSTAGED FILES (%1)")
                            .arg(wipPane.workTree.unstagedCount + wipPane.workTree.untrackedCount)
                          : qsTr("CONFLICTS")
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: bucketHeader.section === "conflicts"
                           ? Theme.danger : Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
                // The seat `Stage all` takes on the other two buckets.
                // The words stay — nothing else in view names the tool
                // being waited on — and the ring says it is still running.
                // No `…`: that is the word for a question standing, and
                // progress is the ring's job (規約 §進行中・長押しの定数).
                //
                // Nothing to press: killing `git mergetool` would leave
                // the editor it started running and its scratch behind.
                Label {
                    visible: bucketHeader.waitingForTool
                    text: qsTr("Waiting for %1").arg(wipPane.workTree.mergeTool)
                    font.pixelSize: Theme.fontSm
                    color: Theme.textSecondary
                }
                NavIcon {
                    visible: bucketHeader.waitingForTool
                    Layout.preferredWidth: Theme.iconSm
                    Layout.preferredHeight: Theme.iconSm
                    kind: "spinner"
                    tint: Theme.textSecondary
                    // On the render thread, so it keeps turning while the
                    // GUI thread drains models.
                    RotationAnimator on rotation {
                        running: bucketHeader.waitingForTool
                                 && AppBackend.shotDir === ""
                        loops: Animation.Infinite
                        from: 0
                        to: 360
                        duration: Metrics.spinMs
                    }
                }
                HoverToolButton {
                    visible: bucketHeader.section !== "conflicts"
                    text: bucketHeader.section === "staged"
                          ? qsTr("Unstage all") : qsTr("Stage all")
                    font.pixelSize: Theme.fontSm
                    ToolTip.visible: hovered
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: bucketHeader.section === "staged"
                        ? qsTr("Unstage everything")
                        : qsTr("Stage everything, untracked included")
                    onClicked: {
                        if (bucketHeader.section === "staged")
                            wipPane.repoTab.unstageAll()
                        else
                            wipPane.repoTab.stageAll()
                    }
                }
            }
        }
        delegate: NavItemDelegate {
            listWidth: wipList.width
            kindHint: "wt"
            showStage: true
            chosen: wipPane.isChosen(bucket, fullName)
            sideOurs: wipPane.workTree.sideOurs
            sideTheirs: wipPane.workTree.sideTheirs
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
            onStageClicked: (bucket, path) => {
                if (bucket === "staged")
                    wipPane.repoTab.unstagePath(path)
                else
                    wipPane.repoTab.stagePath(path)
            }
        }
    }
}
