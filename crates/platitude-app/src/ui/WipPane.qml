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

    readonly property string subjectText: wipSubject.text
    readonly property string bodyText: wipBody.text
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
            TextArea {
                id: wipSubject
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                wrapMode: TextArea.Wrap
                placeholderText: qsTr("Commit summary")
                font.pixelSize: Theme.fontLg
                font.weight: Font.DemiBold
                color: Theme.textPrimary
                background: null
                padding: 0
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
            Item { Layout.fillWidth: true }
            // Warned about, not forbidden: git allows it and the
            // confirmation says what it costs.
            Label {
                visible: wipPane.amending && wipPane.headPublished
                text: qsTr("already pushed")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                ToolTip.visible: amendPushedHover.containsMouse
                ToolTip.delay: 400
                ToolTip.text: qsTr("The last commit is on a remote. "
                                   + "Rewriting it would leave anyone "
                                   + "who already has it out of step.")
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
            ToolTip.delay: 300
            ToolTip.text: !wipPane.repoTab.identityReady
                          ? qsTr("git has no name or email to record on commits")
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
                        ? qsTr("Empty the staging area")
                        : qsTr("Stage every change, untracked files included")
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
            onFileClicked: (bucket, path, origPath) =>
                wipPane.fileActivated(bucket, path, origPath)
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
