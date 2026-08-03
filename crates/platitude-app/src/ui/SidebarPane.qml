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
    signal refMenuRequested(string name, string oidHex, bool isRemote)
    signal worktreeActivated(string path)

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
        // Current branch pinned under the header (it stays in the list
        // too, highlighted). Replaces the old top-left branch display.
        Rectangle {
            visible: (sidebar.expBranches || refFilter.text !== "")
                     && (sidebar.branchesModel.headName !== ""
                         || sidebar.workTree.detached)
            Layout.fillWidth: true
            implicitHeight: Theme.rowHeight
            color: Theme.bgElevated
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
                // "You are here" marker, sharing the fold arrows'
                // column so the sidebar lines up.
                NavIcon {
                    kind: "check"
                    tint: Theme.accent
                    width: Theme.iconSm + 2
                    height: Theme.iconSm + 2
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
            MouseArea {
                id: headRowMouse
                anchors.fill: parent
                hoverEnabled: true
                enabled: sidebar.branchesModel.headOid !== ""
                onClicked: sidebar.refActivated(sidebar.branchesModel.headOid)
            }
        }
        NavList {
            sectionModel: sidebar.branchesModel
            expanded: sidebar.expBranches || refFilter.text !== ""
            kindHint: "branch"
            stretch: sidebar.lastOpen === "branches"
            headTrack: sidebar.workTree.upstream !== ""
                       ? "↑" + sidebar.workTree.ahead + " ↓" + sidebar.workTree.behind : ""
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (name, oidHex) =>
                sidebar.refMenuRequested(name, oidHex, false)
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
            stretch: sidebar.lastOpen === "remotes"
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (name, oidHex) =>
                sidebar.refMenuRequested(name, oidHex, true)
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
            stretch: sidebar.lastOpen === "worktree"
            onFileActivated: (bucket, path, origPath) =>
                sidebar.worktreeActivated(path)
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
            stretch: sidebar.lastOpen === "stashes"
            // A stash is a commit: clicking shows its stashed changes
            // in the details pane.
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
        }

        NavHeader {
            caption: qsTr("TAGS")
            iconKind: "tag"
            iconTint: Theme.warning
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
            stretch: sidebar.lastOpen === "tags"
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
        }
    }
}
