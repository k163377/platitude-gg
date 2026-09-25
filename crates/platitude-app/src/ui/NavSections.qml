import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The open sidebar as one column: the filter band, the five sections (a header band over a list scrolling inside its
// own share), and the ground under the last. Everything here reports upward.
//
// The fold flags stay on the pane (`SidebarPane.exp*` — the page saves and restores them there by name); this column
// only reads them.
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
    /// Automation (PGG_AUTO_ACT=nav-open): what the open row says under itself, asked of the rows — they are what a
    /// run has to find wired up. One key names the open row (`SidebarRowGestures.openKey`), so these ask each list in
    /// turn; **the stand-in first** — while it is on screen its own row is not (`HeadPinRow`).
    readonly property var openable: [headPin, branchList, remoteList, worktreeList, tagList]
    function openWords() {
        for (const list of sections.openable) {
            const said = list.openWords()
            if (said !== "")
                return said
        }
        return ""
    }
    function openFactsItem() {
        for (const list of sections.openable) {
            const lines = list.openFactsItem()
            if (lines !== null)
                return lines
        }
        return null
    }
    /// Where the open row landed. **Not asked of the stand-in**: it is pinned to an edge and opens the other way
    /// (`HeadPinRow.openShown`).
    function openGeom() {
        return branchList.openRow() !== null ? branchList.openGeom()
             : remoteList.openRow() !== null ? remoteList.openGeom()
             : worktreeList.openRow() !== null ? worktreeList.openGeom() : tagList.openGeom()
    }
    /// Whether the open row is showing whole, lines included — asked of whichever list holds it.
    function openShown() {
        for (const list of sections.openable) {
            if (list.openFactsItem() !== null)
                return list.openShown()
        }
        return false
    }
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
    /// Automation (PGG_AUTO_ACT=nav-pin-edge): whether the stand-in is standing, which edge it took, and where that
    /// edge came out — a number a run can hold, where the pane's height is not.
    readonly property bool headPinShown: headPin.visible
    readonly property bool headPinAbove: headPin.rowAbove
    readonly property real headPinY: headPin.y
    /// The folded row the stand-in is sitting under, or -1 while it is sitting under none
    /// (PGG_AUTO_ACT=nav-open head).
    readonly property int headPinUnder: headPin.seatedUnder ? headPin.underRow : -1
    /// Where that seat came out, **read off the row holding it** — a witness outside the arithmetic the stand-in
    /// placed itself by. -1 while no row holds a seat, or the view has not built it.
    function headPinSeatY() {
        const row = headPin.seatedUnder ? branchList.itemAtIndex(headPin.underRow) : null
        return row ? row.y + row.height - row.pinSeat - branchList.contentY : -1
    }
    /// Automation (PGG_AUTO_ACT=nav-pin-edge): scroll the branches list to its end — headless has no other scroll.
    /// Answers where it came to rest, so a caller can tell a list with nothing to scroll from one still building.
    function scrollBranchesToEnd() {
        branchList.contentY = Math.max(0, branchList.contentHeight - branchList.height)
        return branchList.contentY
    }

    // Sections take what their rows want and the ground the rest (デザイン規約 §QML 実装ルール). Both ends must name a
    // pull — Qt reads an unset one as zero and hands it nothing — and at this ratio the ground gets nothing while any
    // section has rows it cannot show.
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
        // Rows open their facts; the working copies' model says which copy holds each branch.
        offersFacts: true
        worktreesModel: sections.worktreesModel
        expanded: branchHead.showsRows
        kindHint: "branch"
        gestures: sections.gestures
        // With no row to ride above, the stand-in takes a row of its own: under the folder that closed over the
        // branch (`pinSeatRow`), else at the head of the list — and the list asks for that much more height, or the
        // row pushed down could not be read. **Not in an empty section**: its 1px of ground is an instruction
        // (デザイン規約 §QML 実装ルール のセクションの地).
        topMargin: headPin.seated && !headPin.seatedUnder && branchList.count > 0 ? Theme.rowHeight : 0
        pinSeatRow: headPin.seatedUnder ? headPin.underRow : -1
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)

        HeadPinRow {
            id: headPin
            // Kept off the scrolling content (rules-refs/app-ui.md「Flickable(ListView 含む)に宣言した子は
            // contentItem に養子入りする」).
            parent: branchList
            pointed: sections.headPinPointed
                     || (branchList.pointedTipRow >= 0
                         && branchList.pointedTipRow === sections.branchesModel.headRow)
            branchesModel: sections.branchesModel
            // Opens the way the row it stands for does (デザイン規約 §左メニューの所作); on the bottom edge it grows
            // upward (`HeadPinRow.y`).
            opensFacts: true
            gestures: sections.gestures
            worktreesModel: sections.worktreesModel
            contentY: branchList.contentY
            viewHeight: branchList.height
            rowInset: branchList.rowInset
            nestStep: branchList.nestStep
            // The room a row opened above the seat took, which the seat moved down by (`HeadPinRow.roomAbove`).
            roomAbove: branchList.openIndex >= 0 && branchList.openIndex <= headPin.underRow
                       ? branchList.openRoom : 0
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
        // The only band with a way to make its own rows, and whose control outlives the section going unavailable
        // (デザイン規約 §左メニューの所作).
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
        // Rows open like BRANCHES ones and name a branch: the working copies' model says which copy holds it…
        offersFacts: true
        worktreesModel: sections.worktreesModel
        // …and the branches model where it stands, for a press on its name.
        branchesModel: sections.branchesModel
        remoteNames: sections.repoTab.remoteNames
        markedRemote: sections.repoTab.pushDefault
        originRemote: sections.repoTab.markedOrigin
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)
        onRemoteMenuRequested: name => sections.remoteMenuRequested(name)
    }

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
        // Rows open with the branch the copy holds, read off BRANCHES. **No working copies' model**: a row here *is*
        // a working copy.
        offersFacts: true
        branchesModel: sections.branchesModel
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
        // A stash is a commit: clicking shows its stashed changes in the details pane.
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
        // Rows open on the remotes carrying the tag (デザイン規約 §左メニューの所作). `pushRemote` is the remote the
        // row's own menu acts on, so the lines read against the same one.
        offersFacts: true
        pushRemote: sections.repoTab.defaultRemote
        Layout.verticalStretchFactor: sections.sectionPull
        onRefActivated: oidHex => sections.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sections.refMenuRequested(kind, name, full, oidHex)
    }

    // Takes the height the sections have no rows for (`groundPull`).
    Item {
        id: ground
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.verticalStretchFactor: sections.groundPull
    }
}
