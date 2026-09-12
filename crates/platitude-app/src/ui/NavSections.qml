import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The open sidebar, as one column: the filter band, the five sections — each a fixed header band over a list that
// scrolls inside its own share — and the ground under the last of them. What a click means stays the pane's business:
// everything here reports upward, and the pane keeps the names its callers already reach for.
//
// The fold flags stay on the pane (`SidebarPane.exp*` — the page saves and restores them there by name); this column
// only reads them, and a click on a band goes back up as `sectionToggled(kind)`, so the pane stays the one writer of
// its own state.
ColumnLayout {
    id: sections

    required property var repoTab
    required property var branchesModel
    required property var remotesModel
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel
    /// The pane's row gestures (`SidebarRowGestures`): which row was clicked last and which is being typed into
    /// outlive both the delegates and these lists.
    required property var gestures
    // Section expansion, read only (see above).
    required property bool expBranches
    required property bool expRemotes
    required property bool expWorktree
    required property bool expStashes
    required property bool expTags
    /// The current branch's sticky stand-in is being pointed at — written on the pane (its smoke hooks), read here.
    property bool headPinPointed: false
    /// The `+` on the REMOTES band is held (`SidebarPane.doorsHeld`) — the one control in this column that writes.
    property bool addHeld: false

    /// A section's header band took a click; the pane flips the flag.
    signal sectionToggled(string kind)
    signal foldRequested(bool collapse)
    signal refActivated(string oidHex)
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    signal remoteMenuRequested(string name)
    signal addRemoteRequested()

    /// Which band and which list a section name answers to — the pane's smoke hooks reach both through these.
    function headOf(kind) {
        return kind === "branch" ? branchHead
            : kind === "remote" ? remoteHead
            : kind === "worktree" ? worktreeHead
            : kind === "stash" ? stashHead
            : kind === "tag" ? tagHead : null
    }
    function listOf(kind) {
        return kind === "branch" ? branchList
            : kind === "remote" ? remoteList
            : kind === "worktree" ? worktreeList
            : kind === "stash" ? stashList
            : kind === "tag" ? tagList : null
    }
    /// What the filter band holds — the pane's typeFilter writes it, the sections' expansion reads it.
    property alias filterText: refFilter.text
    /// Where the ground under the last section begins (PGG_AUTO_ACT=nav-close).
    readonly property real groundTop: ground.y
    /// The sticky stand-in, as lit and as worded (`HeadPinRow`).
    readonly property bool headPinLit: headPin.visible && headPin.pointed
    readonly property string headPinWords: headPin.tipWords
    /// Whether the stand-in is standing, which edge it took, and the coordinate that edge came out as
    /// (PGG_AUTO_ACT=nav-pin-edge). The third is not a second opinion on the second — the place is bound straight off
    /// it — but it is where the edge itself is, which a run can hold to a number while the pane's height cannot.
    readonly property bool headPinShown: headPin.visible
    readonly property bool headPinAbove: headPin.rowAbove
    readonly property real headPinY: headPin.y
    /// Jump the branches list to its end (PGG_AUTO_ACT=nav-pin-edge): the stand-in only changes edges under scroll,
    /// which a headless run cannot produce otherwise. Answers where the list came to rest — a list with no more rows
    /// than it can show has no end to go to, and a caller that could not tell that from a list still building would
    /// wait out its watchdog on a scroll that was never going to happen.
    function scrollBranchesToEnd() {
        branchList.contentY = Math.max(0, branchList.contentHeight - branchList.height)
        return branchList.contentY
    }

    // How the height nobody needs is handed out. Every section takes
    // what its own rows want and no more; the ground at the foot of the
    // column takes the rest — spare height inside a section would push
    // everything under it down. Both ends have to name a pull: Qt reads
    // an unset one as a zero and hands it nothing at all. At this
    // distance the ground keeps none of the height while any section
    // still has rows it cannot show, on a pane of any height (measured).
    readonly property int sectionPull: 10000
    readonly property int groundPull: 1

    spacing: 0
    SidebarFilterRow {
        id: refFilter
        Layout.fillWidth: true
        onTextChanged: {
            sections.branchesModel.setFilter(refFilter.text)
            sections.remotesModel.setFilter(refFilter.text)
            sections.worktreesModel.setFilter(refFilter.text)
            sections.stashesModel.setFilter(refFilter.text)
            sections.tagsModel.setFilter(refFilter.text)
        }
        onFoldRequested: sections.foldRequested(true)
    }

    NavHeader {
        id: branchHead
        caption: qsTr("BRANCHES")
        iconKind: "branch"
        iconTint: Theme.accent
        count: sections.branchesModel.total
        expanded: sections.expBranches || refFilter.text !== ""
        onToggled: sections.sectionToggled("branch")
    }
    NavList {
        id: branchList
        sectionModel: sections.branchesModel
        expanded: branchHead.showsRows
        kindHint: "branch"
        gestures: sections.gestures
        // The one list whose rows something else can sit at the head of: with no row to ride above, the stand-in
        // takes a row of its own and the rows begin under it (`HeadPinRow.seated`). The list asks for that much more
        // height along with it, or the row the margin pushed down would be the one that cannot be read.
        //
        // **A section with nothing in it keeps its 1px of ground and nothing else.** That 1px is an instruction, not
        // a consequence of anything here (デザイン規約 §QML 実装ルール のセクションの地), so a seat is never taken
        // out of it: a filter that matched no branch is answered by the section standing empty, not by one row
        // appearing in it.
        topMargin: headPin.seated && branchList.count > 0 ? Theme.rowHeight : 0
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)

        HeadPinRow {
            id: headPin
            // The list is a Flickable: children declared in one are
            // adopted by its content item and scroll away with it.
            // Parenting to the list itself is what keeps this still,
            // so the adoption is written here rather than inside the
            // component (app-ui.md).
            parent: branchList
            pointed: sections.headPinPointed
                     || (branchList.pointedTipRow >= 0
                         && branchList.pointedTipRow === sections.branchesModel.headRow)
            branchesModel: sections.branchesModel
            contentY: branchList.contentY
            viewHeight: branchList.height
            width: branchList.width
            onActivated: oidHex => sections.refActivated(oidHex)
        }
    }

    NavHeader {
        id: remoteHead
        caption: qsTr("REMOTES")
        iconKind: "remote"
        iconTint: Theme.textSecondary
        count: sections.remotesModel.total
        expanded: sections.expRemotes || refFilter.text !== ""
        onToggled: sections.sectionToggled("remote")
        // The only band that carries a way to make its own rows, and
        // the only one whose control outlives the section going
        // unavailable (デザイン規約 §左メニューの所作).
        showAddRemote: true
        addHeld: sections.addHeld
        onAddRemoteRequested: sections.addRemoteRequested()
    }
    NavList {
        id: remoteList
        sectionModel: sections.remotesModel
        expanded: remoteHead.showsRows
        kindHint: "remote"
        gestures: sections.gestures
        remotesPacked: sections.repoTab.remoteNames
        markedRemote: sections.repoTab.pushDefault
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)
        onRemoteMenuRequested: name => sections.remoteMenuRequested(name)
    }

    // git worktrees (checkouts); the changed-file lists live in the
    // right pane's WIP view. Clicking one goes to the commit that
    // checkout is standing on, double-clicking opens it as a new tab.
    NavHeader {
        id: worktreeHead
        caption: qsTr("WORKTREES")
        iconKind: "tree"
        iconTint: Theme.success
        count: sections.worktreesModel.total
        expanded: sections.expWorktree || refFilter.text !== ""
        onToggled: sections.sectionToggled("worktree")
    }
    NavList {
        id: worktreeList
        sectionModel: sections.worktreesModel
        expanded: worktreeHead.showsRows
        kindHint: "worktree"
        gestures: sections.gestures
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
    }

    NavHeader {
        id: stashHead
        caption: qsTr("STASHES")
        iconKind: "stash"
        iconTint: Theme.textSecondary
        count: sections.stashesModel.total
        expanded: sections.expStashes || refFilter.text !== ""
        onToggled: sections.sectionToggled("stash")
    }
    NavList {
        id: stashList
        sectionModel: sections.stashesModel
        expanded: stashHead.showsRows
        kindHint: "stash"
        gestures: sections.gestures
        Layout.verticalStretchFactor: sections.sectionPull
        // A stash is a commit: clicking shows its stashed changes
        // in the details pane.
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)
    }

    NavHeader {
        id: tagHead
        caption: qsTr("TAGS")
        iconKind: "tag"
        iconTint: Theme.refTag
        count: sections.tagsModel.total
        expanded: sections.expTags || refFilter.text !== ""
        onToggled: sections.sectionToggled("tag")
        showTagToggle: true
        tagsShown: sections.repoTab.tagsShown
        onTagsToggled: shown => sections.repoTab.setTagsShown(shown)
    }
    NavList {
        id: tagList
        sectionModel: sections.tagsModel
        expanded: tagHead.showsRows
        kindHint: "tag"
        gestures: sections.gestures
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)
    }

    // The ground under the last section. Nothing stands on it — it
    // is here to be given the height the sections have no rows for,
    // so that they stay packed against the top whichever of them
    // are closed.
    Item {
        id: ground
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.verticalStretchFactor: sections.groundPull
    }
}
