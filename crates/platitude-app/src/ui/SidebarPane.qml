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
    /// What is typed cannot be accepted. The rules are git's own, asked of
    /// core (a stash's label is free text, not a ref name).
    readonly property bool editRefused: sidebar.editKey !== ""
        && !(sidebar.editKind === "stash"
             ? sidebar.repoTab.validStashMessage(sidebar.editText)
             : sidebar.repoTab.validRefName(sidebar.editText))
    readonly property string editRefusedWhy: !sidebar.editRefused ? ""
        : sidebar.editText.trim() === ""
          ? qsTr("A name is needed")
          : sidebar.editKind === "stash"
            ? qsTr("One line, and nothing invisible in it")
            : qsTr("git will not take this as a name")

    function startEdit(kind, key, mode, id, oid, text) {
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
        sidebar.stopEdit()
        if (mode === "branch")
            sidebar.branchAtRequested(oid, text.trim())
        else if (text.trim() !== id)
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

    /// Smoke hook (PG_SCROLL_TO=nav-bottom): jump the branches list to
    /// its end. The current branch's sticky row only changes edges
    /// under scroll, which a headless run cannot produce otherwise.
    function scrollBranchesToEnd() {
        branchList.contentY = Math.max(
            0, branchList.contentHeight - branchList.height)
    }

    SplitView.preferredWidth: 260
    SplitView.minimumWidth: 180
    color: Theme.bgSurface

    // Section expansion (filter reveals collapsed sections).
    property bool expBranches: true
    property bool expRemotes: true
    property bool expWorktree: true
    property bool expStashes: true
    property bool expTags: true

    // The last expanded section absorbs the leftover height so the
    // sidebar packs top to bottom.
    readonly property string lastOpen: refFilter.text !== "" ? "tags"
        : expTags ? "tags"
        : expStashes ? "stashes"
        : expWorktree ? "worktree"
        : expRemotes ? "remotes"
        : expBranches ? "branches" : ""

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // Frameless, full-width filter: the sidebar is already enclosed
        // by dividers, so the input only keeps a hairline underline
        // (accent on focus). It is this pane's header band, so it takes
        // the header height — the other panes' headers and the first
        // row under each of them line up with it.
        TextField {
            id: refFilter
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            font.pixelSize: Theme.fontMd
            leftPadding: Theme.spaceSm
            rightPadding: Theme.spaceSm
            topPadding: 0
            bottomPadding: 0
            placeholderText: qsTr("Filter")
            background: Rectangle {
                color: "transparent"
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: Theme.borderWidth
                    color: refFilter.activeFocus ? Theme.borderFocus
                                                 : Theme.borderSubtle
                }
            }
            onTextChanged: {
                sidebar.branchesModel.setFilter(text)
                sidebar.remotesModel.setFilter(text)
                sidebar.worktreesModel.setFilter(text)
                sidebar.stashesModel.setFilter(text)
                sidebar.tagsModel.setFilter(text)
            }
        }

        NavHeader {
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
            stretch: sidebar.lastOpen === "branches"
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
                        Layout.preferredWidth: Theme.iconSm + 2
                        Layout.preferredHeight: Theme.iconSm + 2
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
                        width: Theme.iconSm + 2
                        height: Theme.iconSm + 2
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
            stretch: sidebar.lastOpen === "remotes"
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        // git worktrees (checkouts), GitKraken-style; the changed-file
        // lists live in the right pane's WIP view. Clicking one opens
        // it as a new tab.
        NavHeader {
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
            stretch: sidebar.lastOpen === "worktree"
        }

        NavHeader {
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
            stretch: sidebar.lastOpen === "stashes"
            // A stash is a commit: clicking shows its stashed changes
            // in the details pane.
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        NavHeader {
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
            stretch: sidebar.lastOpen === "tags"
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) =>
                sidebar.refMenuRequested(kind, name, full, oidHex)
        }
    }
}
