import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Navigation sidebar: fixed section headers, each section scrolls
// inside its own list. Owns its filter text and fold state; what a
// click means (jump, menu, open a tab) is reported upward.
Rectangle {
    id: sidebar

    required property var repoTab
    required property var workTree
    required property var branchesModel
    required property var remotesModel
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel

    /// Folded down to the rail. Held by the page: what folds it is going
    /// to include opening a diff, and that is the page's to know.
    required property bool collapsed
    /// A menu raised from one of the folded list's rows is standing over
    /// it. Also the page's to know — the menus are its.
    property bool menuOpen: false

    signal refActivated(string oidHex)
    /// Right-click on a row. `kind` is the section it came from, `name`
    /// what the row shows and `full` what git knows it by.
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    signal worktreeActivated(string path)
    /// Double-click on a branch row: `kind` is the chip letter the page's
    /// dispatcher reads ("L" local / "R" remote).
    signal refSwitchRequested(string kind, string name)
    /// A name was typed for a new branch on a tag's commit.
    signal branchAtRequested(string oidHex, string name)
    /// A row was renamed. `kind` is the section ("branch" / "tag" /
    /// "stash"), `id` what git knows the row by.
    signal renameSubmitted(string kind, string id, string name)
    /// Fold the list to its icons, or put it back.
    signal foldRequested(bool collapse)

    // ---- the row gestures ------------------------------------------
    // Held here rather than in a list or a delegate: only one row at a
    // time is the clicked one or the one being typed into, and both have
    // to outlive the delegates that show them (デザイン規約 §左メニューの
    // 所作).
    property string activeKey: ""
    property string editKey: ""
    property string editKind: ""
    property string editMode: ""
    property string editId: ""
    property string editOid: ""
    property string editText: ""
    /// The remote a row being renamed lives on (`origin`), empty for
    /// every other kind of row.
    readonly property string editRemote: sidebar.editKind !== "remote" ? ""
        : sidebar.editId.substring(0, sidebar.editId.indexOf("/"))
    /// A name the remote already carries. Refused here rather than left
    /// to git: a plain push to a name that exists fast-forwards it and
    /// reports success, so somebody else's branch would move instead of
    /// this one being renamed.
    readonly property bool editTaken: sidebar.editRemote !== ""
        && sidebar.editText.trim() !== ""
        && sidebar.remotesModel.oidOfName(
               sidebar.editRemote + "/" + sidebar.editText.trim()) !== ""
    /// What is typed cannot be accepted. The rules are git's own, asked of
    /// core (a stash's label is free text, not a ref name).
    readonly property bool editRefused: sidebar.editKey !== ""
        && (sidebar.editTaken
            || !(sidebar.editKind === "stash"
                 ? sidebar.repoTab.validStashMessage(sidebar.editText)
                 : sidebar.repoTab.validRefName(sidebar.editText)))
    readonly property string editRefusedWhy: !sidebar.editRefused ? ""
        : sidebar.editText.trim() === ""
          ? qsTr("A name is needed")
          : sidebar.editTaken
            ? qsTr("%1 already has a branch called that").arg(sidebar.editRemote)
            : sidebar.editKind === "stash"
              ? qsTr("One line, and nothing invisible in it")
              : qsTr("git will not take this as a name")

    function startEdit(kind, key, mode, id, oid, text) {
        // A name is typed in the list, not in a peek at it. The box wants
        // the keyboard, and a hovered list that has taken the keyboard is
        // one the pointer no longer owns — walking away from it would take
        // the half-typed name with it. So the list comes back first, and
        // the box opens on the same row in it (the key is the row's, not
        // the list's).
        if (sidebar.collapsed)
            sidebar.foldRequested(false)
        sidebar.editKind = kind
        sidebar.editMode = mode
        sidebar.editId = id
        sidebar.editOid = oid
        sidebar.editText = text
        sidebar.editKey = key
    }
    function stopEdit() {
        sidebar.editKey = ""
        sidebar.editText = ""
        sidebar.editMode = ""
    }
    function submitEdit(text) {
        const kind = sidebar.editKind
        const id = sidebar.editId
        const oid = sidebar.editOid
        const mode = sidebar.editMode
        // What the box opened with: a remote branch is typed without the
        // remote it is on, so the name it answers to is not what it shows.
        const was = kind === "remote" ? id.substring(id.indexOf("/") + 1) : id
        sidebar.stopEdit()
        if (mode === "branch")
            sidebar.branchAtRequested(oid, text.trim())
        else if (text.trim() !== was)
            sidebar.renameSubmitted(kind, id, text.trim())
    }
    /// A click landed somewhere: the row it landed on becomes the one a
    /// second click would name, and any box open elsewhere is walked away
    /// from (nothing is asked — what it costs is the typing).
    function noteClick(key) {
        if (sidebar.editKey !== "" && sidebar.editKey !== key)
            sidebar.stopEdit()
        sidebar.activeKey = key
    }
    /// Double-click: where the row leads (デザイン規約 §左メニューの所作).
    function activateRow(kind, name, full, oidHex) {
        const id = full !== "" ? full : name
        if (kind === "branch")
            sidebar.refSwitchRequested("L", id)
        else if (kind === "remote")
            sidebar.refSwitchRequested("R", id)
        else if (kind === "worktree")
            sidebar.worktreeActivated(full)
        else if (kind === "tag")
            // A tag is a mark, not somewhere to carry on from: the row
            // offers the one thing that would make it one.
            sidebar.startEdit(kind, "tag:" + id, "branch", id, oidHex, "")
        // A stash is not a place to stand, and a folder is not a row.
    }
    /// The menu's way into the same box, for anyone who does not know the
    /// gesture or cannot aim two separate clicks at one row.
    function beginRename(kind, id, text) {
        sidebar.startEdit(kind, kind + ":" + id, "rename", id, "", text)
    }
    function beginBranchAt(id, oidHex) {
        sidebar.startEdit("tag", "tag:" + id, "branch", id, oidHex, "")
    }

    /// Smoke hook (PG_AUTO_ACT=nav-peek): rest on one section's cell.
    /// Named rather than hovered — hover cannot be injected (verify-ui
    /// スキル). Whether that opens anything is the cell's answer, not this
    /// one's: an empty section is asked here the same way it is asked by a
    /// pointer, and answers no.
    function peekAt(kind) {
        rail.enterAt(kind, rail.topOf(kind))
    }
    /// Smoke hooks (PG_AUTO_ACT=nav-peek-away / nav-peek-shut): walk the
    /// pointer off the cell that opened it, and click that cell. All of
    /// these go in at the rail, so what the cells decide and report is
    /// part of what is being tested (NavRail.enterAt).
    function peekAway(kind) {
        rail.leaveAt(kind)
    }
    function peekTap(kind) {
        rail.tapAt(kind)
    }
    /// Smoke hooks (PG_AUTO_ACT=nav-peek-into / nav-peek-out): the pointer
    /// walked off the cell down into the open section, and then out of the
    /// section the other way (into the diff or the graph) instead of back
    /// over the cell. They write the same `peekEntered` the popup's own
    /// hover writes — the leaving and the being-inside are one state, and
    /// a headless run that cannot say "inside" cannot tell the exit that
    /// closes it from the one that must not.
    function peekInto(kind) {
        rail.leaveAt(kind)
        sidebar.peekEntered = true
    }
    function peekOut() {
        sidebar.peekEntered = false
    }

    /// Smoke hook (PG_AUTO_ACT=nav-close): close one section, by raising
    /// the signal its header band raises under a click — what closing
    /// means is the header's to say and this pane's to hold, and a hook
    /// that set `expTags` itself would be a second answer.
    function closeSection(kind) {
        const head = kind === "branch" ? branchHead
            : kind === "remote" ? remoteHead
            : kind === "worktree" ? worktreeHead
            : kind === "stash" ? stashHead
            : kind === "tag" ? tagHead : null
        if (head)
            head.toggled()
    }
    /// Where a section's header band has come to rest, and where the
    /// ground under the last section begins. Read together they say the
    /// sections are packed against the top: a closed section whose header
    /// has been pushed to the foot of the pane is what this is watching
    /// for (PG_AUTO_ACT=nav-close).
    function headerTopOf(kind) {
        const head = kind === "branch" ? branchHead
            : kind === "remote" ? remoteHead
            : kind === "worktree" ? worktreeHead
            : kind === "stash" ? stashHead
            : kind === "tag" ? tagHead : null
        return head ? head.y : -1
    }
    readonly property real groundTop: ground.y
    /// Where the section the folded rail has open begins and ends, for the
    /// smoke hooks (PG_AUTO_ACT=nav-peek). The panel is a popup, so a
    /// headless run reads its placement here rather than off the picture:
    /// it starts at the top edge of the cell that opened it and stops
    /// inside the pane, whatever the section's row count.
    readonly property real peekY: peek.y
    readonly property real peekBottom: peek.y + peek.height

    /// Smoke hook (PG_SCROLL_TO=nav-bottom): jump the branches list to
    /// its end. The current branch's sticky row only changes edges
    /// under scroll, which a headless run cannot produce otherwise.
    function scrollBranchesToEnd() {
        branchList.contentY = Math.max(
            0, branchList.contentHeight - branchList.height)
    }

    // The width the list goes back to. Read off the pane as it folds
    // rather than fixed, so one that has been widened comes back the
    // width it was left (規約 §レイアウト初期値 is only where it starts).
    property real openWidth: 260
    /// Narrower than this is not a width anybody dragged to.
    readonly property int minOpenWidth: 180
    SplitView.preferredWidth: sidebar.openWidth
    SplitView.minimumWidth: sidebar.minOpenWidth
    color: Theme.bgSurface

    // Folding is a size, and a size is the splitter's business: pinning
    // both ends to the rail's width is what takes the drag away while it
    // is folded. Assigned rather than bound — a drag writes the same
    // attached property, and a binding here would be gone after the first
    // one (leaving the fold with nothing to set).
    onCollapsedChanged: sidebar.applyFold()
    function applyFold() {
        // Whichever way it goes, the one section the rail had open goes
        // with the rail — including when what put the list back was a
        // row in that very section (startEdit).
        sidebar.closePeek()
        if (sidebar.collapsed) {
            // Only a width the splitter has actually handed over is worth
            // going back to. A page built already folded — one restored
            // from the last session — has not been laid out yet and
            // reports 0, and folding that in would lose the width the
            // session was left at.
            if (sidebar.width >= sidebar.minOpenWidth)
                sidebar.openWidth = sidebar.width
            sidebar.SplitView.minimumWidth = Theme.railWidth
            sidebar.SplitView.maximumWidth = Theme.railWidth
            sidebar.SplitView.preferredWidth = Theme.railWidth
        } else {
            sidebar.SplitView.minimumWidth = sidebar.minOpenWidth
            sidebar.SplitView.maximumWidth = Number.POSITIVE_INFINITY
            sidebar.SplitView.preferredWidth = sidebar.openWidth
        }
    }

    // Section expansion (filter reveals collapsed sections).
    property bool expBranches: true
    property bool expRemotes: true
    property bool expWorktree: true
    property bool expStashes: true
    property bool expTags: true

    // How the height nobody needs is handed out. Every section takes
    // what its own rows want and no more; the ground at the foot of the
    // column takes the rest. It is not the last open section's to hold —
    // spare height inside a section pushes everything under it down,
    // and a closed section, being nothing but its header, would be
    // pushed all the way to the bottom edge (which is what closing TAGS
    // used to do). Both ends have to name a pull: Qt reads an unset one
    // as a zero and hands it nothing at all. At this distance the ground
    // keeps none of the height while any section still has rows it
    // cannot show, on a pane of any height (measured).
    readonly property int sectionPull: 10000
    readonly property int groundPull: 1

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: !sidebar.collapsed
        // This pane's header band: a frameless filter that takes all the
        // width left over, and the fold control at the end of it. The
        // control stands beside the input rather than inside its frame —
        // in it, it reads as part of what is being typed. It is the band
        // the other panes' headers line up with, so it takes the header
        // height, and the hairline that closes it is the band's rather
        // than the input's (it runs on under the button).
        Item {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            RowLayout {
                anchors.fill: parent
                spacing: 0
                TextField {
                    id: refFilter
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    font.pixelSize: Theme.fontMd
                    leftPadding: Theme.spaceSm
                    rightPadding: Theme.spaceSm
                    topPadding: 0
                    bottomPadding: 0
                    placeholderText: qsTr("Filter")
                    background: null
                    onTextChanged: {
                        sidebar.branchesModel.setFilter(text)
                        sidebar.remotesModel.setFilter(text)
                        sidebar.worktreesModel.setFilter(text)
                        sidebar.stashesModel.setFilter(text)
                        sidebar.tagsModel.setFilter(text)
                    }
                }
                // The fold control has nothing to do with what is being
                // typed, so it gets a block of its own at the end of the
                // band — the header's ground, divided off by a hairline.
                // The whole block is the button: the reach is the band's
                // full height, not a mark's worth of it. The rail's band
                // is the same block with the mark pointing back.
                FoldBlock {
                    Layout.fillHeight: true
                    Layout.preferredWidth: Theme.headerHeight
                    tip: qsTr("Fold the list to its icons")
                    onActivated: sidebar.foldRequested(true)
                }
            }
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.borderWidth
                color: refFilter.activeFocus ? Theme.borderFocus
                                             : Theme.borderSubtle
            }
        }

        NavHeader {
            id: branchHead
            caption: qsTr("BRANCHES")
            iconKind: "branch"
            iconTint: Theme.accent
            count: sidebar.branchesModel.total
            expanded: sidebar.expBranches || refFilter.text !== ""
            onToggled: sidebar.expBranches = !sidebar.expBranches
        }
        NavList {
            id: branchList
            sectionModel: sidebar.branchesModel
            expanded: sidebar.expBranches || refFilter.text !== ""
            kindHint: "branch"
            gestures: sidebar
            Layout.verticalStretchFactor: sidebar.sectionPull
            headTrack: sidebar.workTree.upstream !== ""
                       ? "↑" + sidebar.workTree.ahead + " ↓" + sidebar.workTree.behind : ""
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)

            // The current branch never leaves the viewport: while its own
            // row is scrolled off, this stand-in rides the edge the row
            // went out of, and it steps aside the moment the row itself
            // is on screen — so the sidebar never shows the branch twice.
            // A detached HEAD (and a branch a filter or a folded folder
            // hides) has no row at all, so the stand-in stays on top.
            Rectangle {
                id: headPin
                // The list is a Flickable: children declared in one are
                // adopted by its content item and scroll away with it.
                // Parenting to the list itself is what keeps this still.
                parent: branchList

                readonly property real rowTop:
                    sidebar.branchesModel.headRow * Theme.rowHeight
                readonly property bool rowAbove:
                    sidebar.branchesModel.headRow < 0
                    || rowTop < branchList.contentY
                readonly property bool rowBelow:
                    rowTop + Theme.rowHeight
                        > branchList.contentY + branchList.height

                visible: (sidebar.branchesModel.headName !== ""
                          || sidebar.workTree.detached)
                         && (rowAbove || rowBelow)
                width: branchList.width
                height: Theme.rowHeight
                y: rowAbove ? 0 : branchList.height - height
                // Dressed as the row it stands for, down to the margins:
                // the current branch's own highlight, not a header band.
                color: Theme.accentMuted
                Rectangle {
                    anchors.fill: parent
                    color: Theme.bgHover
                    visible: headRowMouse.containsMouse
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spaceMd
                    anchors.rightMargin: Theme.spaceSm
                    spacing: Theme.spaceXs
                    // The rows' mark slot, left empty: the stand-in has
                    // no mark of its own, but its name has to begin in
                    // the same column as the rows it rides above.
                    Item {
                        Layout.preferredWidth: Theme.iconMd
                        Layout.preferredHeight: Theme.iconMd
                        Layout.alignment: Qt.AlignVCenter
                    }
                    Label {
                        Layout.fillWidth: true
                        text: sidebar.workTree.detached ? qsTr("DETACHED HEAD")
                                                        : sidebar.branchesModel.headName
                        color: sidebar.workTree.detached ? Theme.warning : Theme.textLink
                        font.weight: Font.DemiBold
                        font.pixelSize: Theme.fontMd
                        elide: Text.ElideMiddle
                    }
                    Label {
                        visible: !sidebar.workTree.detached && sidebar.workTree.upstream !== ""
                        text: "↑" + sidebar.workTree.ahead + " ↓" + sidebar.workTree.behind
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                    }
                    NavIcon {
                        visible: !sidebar.workTree.detached
                                 && (sidebar.branchesModel.headHasRemote
                                     || sidebar.branchesModel.headHasPr)
                        kind: sidebar.branchesModel.headHasPr ? "pr" : "remote"
                        tint: sidebar.branchesModel.headHasPr ? Theme.success
                                                              : Theme.textSecondary
                        width: Theme.iconSm
                        height: Theme.iconSm
                    }
                }
                // Hairline on the side the scrolled rows pass under.
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    y: headPin.rowAbove ? parent.height - height : 0
                    height: Theme.borderWidth
                    color: Theme.borderSubtle
                }
                MouseArea {
                    id: headRowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    enabled: sidebar.branchesModel.headOid !== ""
                    onClicked: sidebar.refActivated(sidebar.branchesModel.headOid)
                }
            }
        }

        NavHeader {
            id: remoteHead
            caption: qsTr("REMOTES")
            iconKind: "remote"
            iconTint: Theme.textSecondary
            count: sidebar.remotesModel.total
            expanded: sidebar.expRemotes || refFilter.text !== ""
            onToggled: sidebar.expRemotes = !sidebar.expRemotes
        }
        NavList {
            sectionModel: sidebar.remotesModel
            expanded: sidebar.expRemotes || refFilter.text !== ""
            kindHint: "remote"
            gestures: sidebar
            Layout.verticalStretchFactor: sidebar.sectionPull
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        // git worktrees (checkouts), GitKraken-style; the changed-file
        // lists live in the right pane's WIP view. Clicking one opens
        // it as a new tab.
        NavHeader {
            id: worktreeHead
            caption: qsTr("WORKTREES")
            iconKind: "tree"
            iconTint: Theme.success
            count: sidebar.worktreesModel.total
            expanded: sidebar.expWorktree || refFilter.text !== ""
            onToggled: sidebar.expWorktree = !sidebar.expWorktree
        }
        NavList {
            sectionModel: sidebar.worktreesModel
            expanded: sidebar.expWorktree || refFilter.text !== ""
            kindHint: "worktree"
            gestures: sidebar
            Layout.verticalStretchFactor: sidebar.sectionPull
        }

        NavHeader {
            id: stashHead
            caption: qsTr("STASHES")
            iconKind: "stash"
            iconTint: Theme.textSecondary
            count: sidebar.stashesModel.total
            expanded: sidebar.expStashes || refFilter.text !== ""
            onToggled: sidebar.expStashes = !sidebar.expStashes
        }
        NavList {
            sectionModel: sidebar.stashesModel
            expanded: sidebar.expStashes || refFilter.text !== ""
            kindHint: "stash"
            gestures: sidebar
            Layout.verticalStretchFactor: sidebar.sectionPull
            // A stash is a commit: clicking shows its stashed changes
            // in the details pane.
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        NavHeader {
            id: tagHead
            caption: qsTr("TAGS")
            iconKind: "tag"
            iconTint: Theme.refTag
            count: sidebar.tagsModel.total
            expanded: sidebar.expTags || refFilter.text !== ""
            onToggled: sidebar.expTags = !sidebar.expTags
            showTagToggle: true
            tagsShown: sidebar.repoTab.tagsShown
            onTagsToggled: shown => sidebar.repoTab.setTagsShown(shown)
        }
        NavList {
            sectionModel: sidebar.tagsModel
            expanded: sidebar.expTags || refFilter.text !== ""
            kindHint: "tag"
            gestures: sidebar
            Layout.verticalStretchFactor: sidebar.sectionPull
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        // The ground under the last section. Nothing stands on it — it
        // is here to be given the height the sections have no rows for,
        // so that they stay packed against the top whichever of them
        // are closed.
        Item {
            id: ground
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.verticalStretchFactor: sidebar.groundPull
        }
    }

    // ---- folded ------------------------------------------------------
    NavRail {
        id: rail
        anchors.fill: parent
        visible: sidebar.collapsed
        branchesModel: sidebar.branchesModel
        remotesModel: sidebar.remotesModel
        worktreesModel: sidebar.worktreesModel
        stashesModel: sidebar.stashesModel
        tagsModel: sidebar.tagsModel
        tagsShown: sidebar.repoTab.tagsShown
        openKind: peek.visible ? sidebar.peekKind : ""
        onPeekRequested: (kind, top) => sidebar.openPeek(kind, top)
        onPeekLeft: kind => sidebar.leavePeek(kind)
        onPeekToggled: (kind, top) => sidebar.togglePeek(kind, top)
        onUnfoldRequested: sidebar.foldRequested(false)
    }

    // ---- the folded list's one open section --------------------------
    // Which one is open, and whether it still has the pointer, are held
    // here rather than on a cell: the cell is left behind the moment the
    // pointer walks into what it opened.
    property string peekKind: ""
    property real peekTop: 0
    property bool peekWanted: false
    /// The pointer is down in the open section itself. The popup's own
    /// hover writes this and so do the smoke hooks, so a headless run and
    /// a real pointer come to one answer (the same shape as the diff's
    /// hunk hover — 規約 §diff の中のステージ). What happens when it goes
    /// false hangs off the change rather than off the hover, so there is
    /// no way to say "gone" without the settle that has to follow.
    property bool peekEntered: false
    onPeekEnteredChanged: {
        if (!sidebar.peekEntered)
            sidebar.settlePeek()
    }
    /// What holds it open with the pointer elsewhere: a menu raised from
    /// one of its rows is standing over it, and taking the row away from
    /// under an open menu reads as the row having gone.
    readonly property bool peekPinned: sidebar.menuOpen
    readonly property var peekModel:
        sidebar.peekKind === "" ? null : rail.modelOf(sidebar.peekKind)
    // The section headers' own words, said again for the one section the
    // folded list shows: the rail has only an icon to name it with.
    readonly property string peekCaption:
        sidebar.peekKind === "branch" ? qsTr("BRANCHES")
        : sidebar.peekKind === "remote" ? qsTr("REMOTES")
        : sidebar.peekKind === "worktree" ? qsTr("WORKTREES")
        : sidebar.peekKind === "stash" ? qsTr("STASHES")
        : sidebar.peekKind === "tag" ? qsTr("TAGS") : ""

    function openPeek(kind, top) {
        sidebar.peekKind = kind
        sidebar.peekTop = top
        sidebar.peekWanted = true
        peek.open()
    }
    /// A click landed on the cell the pointer is resting on. The section
    /// standing beside the rail goes away, and a second click brings it
    /// back — the hover cannot, because the pointer has not moved and so
    /// nothing about it has changed.
    function togglePeek(kind, top) {
        // Asked of `peekKind` rather than the popup: on the way out it is
        // still visible, and a click that arrived then would close what it
        // was meant to open.
        if (sidebar.peekKind === kind)
            sidebar.closePeek()
        else
            sidebar.openPeek(kind, top)
    }
    /// The pointer left a cell. Only the cell whose section is open can
    /// take it away — the one being left on the way to another has
    /// already been replaced by the time this runs, in whichever order
    /// the two arrive.
    function leavePeek(kind) {
        if (sidebar.peekKind === kind)
            sidebar.peekWanted = false
        sidebar.settlePeek()
    }
    // The section opens flush against the rail, so walking into it takes
    // the pointer off the cell, and walking back out puts it on again.
    // Both hovers change in the same frame and in no fixed order, so the
    // answer waits for the end of this round of events, by which time
    // whichever of the two now holds the pointer has said so.
    function settlePeek() {
        Qt.callLater(function () {
            if (!sidebar.peekEntered && !sidebar.peekWanted
                    && !sidebar.peekPinned)
                sidebar.closePeek()
        })
    }
    function closePeek() {
        peek.close()
        sidebar.peekKind = ""
        sidebar.peekWanted = false
        // The list it was in has gone, so the pointer is not in it
        // whatever the last hover said.
        sidebar.peekEntered = false
    }
    // The menu that was standing over it has gone: whether the pointer
    // came back in the meantime decides what happens now.
    onPeekPinnedChanged: sidebar.settlePeek()

    Popup {
        id: peek
        parent: sidebar
        // Flush against the rail, with nothing in between for the pointer
        // to fall through, and starting level with the cell that opened
        // it. It only ever grows downwards from there: a section with more
        // rows than the pane can hold would otherwise be laid out from the
        // top edge of the pane, and a list that opens nowhere near the
        // mark it came out of is one the pointer has to go and look for
        // (a repository with 45,000 tags puts every peek up there —
        // reported 2026-08-08). What it costs is height: the cells further
        // down get less of it, and the rows that do not fit scroll.
        x: sidebar.width
        y: sidebar.peekTop
        width: sidebar.openWidth
        // As tall as it has rows, and never past the foot of the pane it
        // comes out of. It never opens with no rows at all — a cell
        // holding a zero does not open (NavRail).
        height: Math.min(Theme.headerHeight + peekList.count * Theme.rowHeight
                         + Theme.borderWidth,
                         Math.max(0, sidebar.height - sidebar.peekTop))
        padding: 0
        margins: 0
        // Leaving it is what closes it (above). Escape is for the reader
        // whose pointer is already inside it.
        closePolicy: Popup.CloseOnEscape

        // It keeps the list's own ground rather than a menu's: what is in
        // it is the sidebar, and the header band would be lost against
        // `bgElevated`. The frame is what floats it (規約 §メニュー), and
        // there is no rounding on a panel that starts flush against the
        // rail.
        background: Rectangle {
            color: Theme.bgSurface
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }

        contentItem: ColumnLayout {
            spacing: 0
            HoverHandler {
                id: peekHover
                // The other half of leaving. The cells can only see the
                // way back over themselves; walking out of the list the
                // other way — right into the diff or the graph, or off
                // its top or bottom edge — is an exit no cell is told
                // about, and without this it raises no event at all.
                onHoveredChanged: sidebar.peekEntered = peekHover.hovered
            }
            NavHeader {
                caption: sidebar.peekCaption
                iconKind: rail.sectionOf(sidebar.peekKind).icon
                iconTint: rail.sectionOf(sidebar.peekKind).tint
                count: sidebar.peekModel ? sidebar.peekModel.total : 0
                // Nothing to fold away to: this list is the only thing on
                // screen. What closes it is the pointer leaving.
                foldable: false
                showTagToggle: sidebar.peekKind === "tag"
                tagsShown: sidebar.repoTab.tagsShown
                onTagsToggled: shown => sidebar.repoTab.setTagsShown(shown)
            }
            NavList {
                id: peekList
                sectionModel: sidebar.peekModel
                expanded: true
                kindHint: sidebar.peekKind
                gestures: sidebar
                stretch: true
                headTrack: sidebar.peekKind === "branch"
                           && sidebar.workTree.upstream !== ""
                           ? "↑" + sidebar.workTree.ahead
                             + " ↓" + sidebar.workTree.behind : ""
                onRefActivated: oidHex => sidebar.refActivated(oidHex)
                onRefMenuRequested: (kind, name, full, oidHex) =>
                    sidebar.refMenuRequested(kind, name, full, oidHex)
            }
        }
    }
}
