pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
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
    /// The question bar over the file list was answered / walked away
    /// from. The page holds what the question guarded.
    signal askConfirmed()
    signal askCancelled()

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

    // ---- a standing question about one file ------------------------
    // The same bar the graph raises, over the list the file is in: the
    // question is written on the bar and the file's own row is marked
    // (デザイン規約 §可否・警告の出し場所).
    /// Which row is being asked about, as `<bucket>:<path>`.
    property string askKey: ""
    function startAsking(key, label, detail, accept) {
        wipPane.askKey = key
        askBar.label = label
        askBar.detail = detail
        askBar.accept = accept
    }
    function stopAsking() {
        wipPane.askKey = ""
        askBar.label = ""
    }
    /// Automation: answer the standing question the way a person does,
    /// by holding the pill down to the end.
    function completeHold() {
        askBar.completeHold()
    }
    /// A click on anything in this pane other than the bar walks away
    /// from the question, the way one anywhere else does — every click
    /// here either moves rows between the buckets the mark names or
    /// leaves the list behind.
    ///
    /// A click on a file row is the exception: it opens that file's diff
    /// in the centre, which is how one sees what is about to be thrown
    /// away, so the question waits.
    function leaveAsk() {
        if (askBar.open)
            wipPane.askCancelled()
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
                ToolTip.delay: 600
                ToolTip.text: qsTr("Set these changes aside for later")
                onClicked: {
                    wipPane.leaveAsk()
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
                ToolTip.delay: 600
                ToolTip.text: qsTr("Tree view")
                onClicked: wipPane.worktreeModel.setTreeView(true)
                contentItem: NavIcon {
                    kind: "hier"
                    tint: wipPane.worktreeModel.treeView ? Theme.accent
                                                         : Theme.textMuted
                }
            }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Paths view")
                onClicked: wipPane.worktreeModel.setTreeView(false)
                contentItem: NavIcon {
                    kind: "list"
                    tint: wipPane.worktreeModel.treeView ? Theme.textMuted
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
                    implicitHeight: Theme.controlHeight
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
                ToolTip.delay: 400
                ToolTip.text: qsTr("Replaces the author %1 <%2> with you, "
                                   + "dated now")
                              .arg(wipPane.repoTab.headAuthorName)
                              .arg(wipPane.repoTab.headAuthorEmail)
            }
            Item { Layout.fillWidth: true }
            // Said, not asked: rewriting a pushed commit is undone by a
            // switch or a reset, so the amend goes ahead and this tag
            // is all the warning it gets.
            Label {
                visible: wipPane.amending && wipPane.headPublished
                text: qsTr("already pushed")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                ToolTip.visible: amendPushedHover.containsMouse
                ToolTip.delay: 400
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
            onClicked: {
                wipPane.leaveAsk()
                wipPane.commitClicked()
            }
            ToolTip.visible: commitHover.containsMouse && !enabled
            ToolTip.delay: 300
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
    }
    // Between the editor and the rows it is about: the list moves down
    // rather than losing its top rows behind the bar.
    AskBar {
        id: askBar
        Layout.fillWidth: true
        // Everything asked over this list ends in work being thrown
        // away, and nothing here reaches past this machine (§状態) — so
        // the answer is taken the way an irreversible one is taken where
        // the intent can be shown on the spot: held, not clicked
        // (デザイン規約 §進行中・長押しの定数).
        danger: true
        hold: true
        onConfirmed: wipPane.askConfirmed()
        onCancelled: wipPane.askCancelled()
    }
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
                HoverToolButton {
                    visible: bucketHeader.section !== "conflicts"
                    text: bucketHeader.section === "staged"
                          ? qsTr("Unstage all") : qsTr("Stage all")
                    font.pixelSize: Theme.fontSm
                    ToolTip.visible: hovered
                    ToolTip.delay: 300
                    ToolTip.text: bucketHeader.section === "staged"
                        ? qsTr("Unstage everything")
                        : qsTr("Stage everything, untracked included")
                    onClicked: {
                        wipPane.leaveAsk()
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
            askKey: wipPane.askKey
            askDanger: askBar.danger
            chosen: wipPane.isChosen(bucket, fullName)
            onFileClicked: (bucket, path, origPath, modifiers) => {
                // Choosing rows is not reading one: only a plain click
                // moves the diff. A question about other rows goes.
                if (!wipPane.applyClick(bucket, path, modifiers)) {
                    wipPane.leaveAsk()
                    return
                }
                wipPane.fileActivated(bucket, path, origPath)
            }
            onFileMenuRequested: (bucket, path, origPath) => {
                if (!wipPane.isChosen(bucket, path))
                    wipPane.chooseOnly(bucket, path)
                wipPane.fileMenuRequested(bucket, path, origPath)
            }
            onFolderClicked: key => {
                wipPane.leaveAsk()
                wipPane.worktreeModel.toggleFolder(key)
            }
            onStageClicked: (bucket, path) => {
                wipPane.leaveAsk()
                if (bucket === "staged")
                    wipPane.repoTab.unstagePath(path)
                else
                    wipPane.repoTab.stagePath(path)
            }
        }
    }
}
