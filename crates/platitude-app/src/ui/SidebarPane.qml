import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Navigation sidebar: fixed section headers, each section scrolls
// inside its own list. Owns its filter text and per-section fold
// state; what a click means (jump, menu, open a tab) is reported
// upward.
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
    /// Only ever a rename's rule: the box for a new branch's name opens on
    /// a remote row too, and what it makes is a local branch — a name the
    /// remote happens to carry is no answer to that.
    readonly property bool editTaken: sidebar.editMode === "rename" && sidebar.editRemote !== ""
        && sidebar.editText.trim() !== ""
        && sidebar.remotesModel.oidOfName(sidebar.editRemote + "/" + sidebar.editText.trim()) !== ""
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
        // A name is typed in the list, not in a peek at it: walking away
        // from a hovered list that has taken the keyboard would take the
        // half-typed name with it. So the list comes back first, and the
        // box opens on the same row in it (the key is the row's, not the
        // list's).
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
    /// The same box on any row that names a commit, not just a tag's: a
    /// branch is most often started where another one already stands.
    function beginBranchAt(kind, id, oidHex) {
        sidebar.startEdit(kind, kind + ":" + id, "branch", id, oidHex, "")
    }

    /// Smoke hook (PG_AUTO_ACT=nav-filter): type into the filter band.
    /// Written into the field itself, so what the sections are asked is
    /// what a typist asks them.
    function typeFilter(text) {
        refFilter.text = text
        return refFilter.text
    }

    /// Smoke hook (PG_AUTO_ACT=nav-peek): rest on one section's cell.
    /// Named rather than hovered — hover cannot be injected (verify-ui
    /// スキル). Whether that opens anything is the cell's answer: an
    /// empty section answers no.
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

    /// Smoke hook (PG_AUTO_ACT=nav-close): close one section by raising
    /// the signal its header band raises under a click — a hook that set
    /// `expTags` itself would be a second answer.
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
    /// ground under the last section begins — what PG_AUTO_ACT=nav-close
    /// reads to see the sections packed against the top.
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
        branchList.contentY = Math.max(0, branchList.contentHeight - branchList.height)
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
    // column takes the rest — spare height inside a section would push
    // everything under it down. Both ends have to name a pull: Qt reads
    // an unset one as a zero and hands it nothing at all. At this
    // distance the ground keeps none of the height while any section
    // still has rows it cannot show, on a pane of any height (measured).
    readonly property int sectionPull: 10000
    readonly property int groundPull: 1

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: !sidebar.collapsed
        SidebarFilterRow {
            id: refFilter
            Layout.fillWidth: true
            onTextChanged: {
                sidebar.branchesModel.setFilter(refFilter.text)
                sidebar.remotesModel.setFilter(refFilter.text)
                sidebar.worktreesModel.setFilter(refFilter.text)
                sidebar.stashesModel.setFilter(refFilter.text)
                sidebar.tagsModel.setFilter(refFilter.text)
            }
            onFoldRequested: sidebar.foldRequested(true)
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
            headTracks: sidebar.workTree.upstream !== ""
            headAhead: sidebar.workTree.ahead
            headBehind: sidebar.workTree.behind
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)

            HeadPinRow {
                // The list is a Flickable: children declared in one are
                // adopted by its content item and scroll away with it.
                // Parenting to the list itself is what keeps this still,
                // so the adoption is written here rather than inside the
                // component (app-ui.md).
                parent: branchList
                branchesModel: sidebar.branchesModel
                workTree: sidebar.workTree
                contentY: branchList.contentY
                viewHeight: branchList.height
                width: branchList.width
                onActivated: oidHex => sidebar.refActivated(oidHex)
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
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        // git worktrees (checkouts); the changed-file lists live in the
        // right pane's WIP view. Clicking one opens it as a new tab.
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
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
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
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
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
    // The section itself, and the bookkeeping that says when it is open,
    // live in SectionPeekPopup. The pane keeps the names the rail, the
    // fold and the smoke hooks already call it by.
    property alias peekKind: peek.kind
    property alias peekTop: peek.top
    property alias peekEntered: peek.entered

    function openPeek(kind, top) {
        peek.openAt(kind, top)
    }
    function togglePeek(kind, top) {
        peek.toggleAt(kind, top)
    }
    function leavePeek(kind) {
        peek.leaveAt(kind)
    }
    function closePeek() {
        peek.shut()
    }

    SectionPeekPopup {
        id: peek
        parent: sidebar
        repoTab: sidebar.repoTab
        workTree: sidebar.workTree
        rail: rail
        gestures: sidebar
        paneW: sidebar.width
        paneH: sidebar.height
        listW: sidebar.openWidth
        pinned: sidebar.menuOpen
        onRefActivated: oidHex => sidebar.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
    }
}
