pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, commit-details mode: message, author card, stash
// actions when the selected row is a stash, and the changed-file list.
// The message boxes are the editor for that commit's message — the
// same pair the working-tree pane commits with, so a message is
// written and rewritten in the same place.
ColumnLayout {
    id: detailsPane

    required property var details
    // Reflog selector when the selected row is a stash ("" otherwise).
    property string stashRef: ""
    // Whether this commit's message may be rewritten from here. The
    // page decides: only commits the working tree stands on can be
    // amended or replayed, and a stash is a commit but not one of those.
    property bool editable: false
    // A write is already running, so nothing new starts.
    property bool busy: false
    // HEAD's own commit: anything older is replayed instead of amended,
    // which the editor says out loud.
    property string headOid: ""
    // Why the boxes are read-only, in one line ("" when they are not).
    // A box that refuses typing without saying why reads as broken.
    property string editBlocked: ""
    // Something wants to move off this commit while the message is
    // half-written. The question belongs to the text, so it is asked
    // where the text is: the row under the boxes turns into it.
    property bool asking: false
    // A remote already has this commit. Rewriting is not asked about,
    // but 要望.md wants it said, so the save row carries the warning.
    property bool published: false

    signal fileActivated(string path, string origPath)
    signal parentClicked(string oidHex)
    signal copyRequested(string text)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    /// Save was pressed.
    signal messageSubmitted(string oidHex, string subject, string body)
    /// The leaving question was answered: true drops the edits and lets
    /// the move through, false stays on this commit.
    signal leaveResolved(bool discard)

    // ---- message editor state --------------------------------------
    // The boxes are filled by hand rather than bound: typing would
    // break a binding for good, and the next commit would arrive in a
    // box that no longer listens.
    property string baseOid: ""
    property string baseSubject: ""
    property string baseBody: ""
    readonly property bool messageDirty:
        detailsPane.editable
        && (subjectArea.text !== detailsPane.baseSubject
            || bodyArea.text !== detailsPane.baseBody)

    /// Adopt the model's message whenever it moves to another commit.
    /// Nothing else can change a message in place — a different message
    /// is a different commit — so an untouched box needs no other cue.
    function syncMessage() {
        if (detailsPane.details.shaHex === detailsPane.baseOid)
            return
        detailsPane.baseOid = detailsPane.details.shaHex
        detailsPane.baseSubject = detailsPane.details.messageSubject
        detailsPane.baseBody = detailsPane.details.messageBody
        subjectArea.text = detailsPane.baseSubject
        bodyArea.text = detailsPane.baseBody
    }
    /// Put the commit's own message back.
    function revertMessage() {
        subjectArea.text = detailsPane.baseSubject
        bodyArea.text = detailsPane.baseBody
    }
    /// git took the new message. What was written becomes the resting
    /// text: the commit it belonged to is gone under that hash, and the
    /// model still holds the old one until the selection follows.
    function noteMessageSaved() {
        detailsPane.baseSubject = subjectArea.text
        detailsPane.baseBody = bodyArea.text
    }
    function submitMessage() {
        if (!detailsPane.editable || subjectArea.text.trim() === "")
            return
        detailsPane.messageSubmitted(detailsPane.details.shaHex,
                                     subjectArea.text, bodyArea.text)
    }
    /// Smoke hook: type into the boxes the way a keystroke would —
    /// including not at all when they are read-only.
    function setMessageText(subject, body) {
        if (!detailsPane.editable)
            return
        subjectArea.text = subject
        bodyArea.text = body
    }

    Connections {
        target: detailsPane.details
        function onChanged() { detailsPane.syncMessage() }
    }
    Component.onCompleted: detailsPane.syncMessage()

    spacing: 0

    PaneHeader {
        text: qsTr("COMMIT")
    }
    // Stash actions when the selected row is a stash.
    Rectangle {
        visible: detailsPane.stashRef !== ""
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            NavIcon {
                kind: "stash"
                tint: Theme.textSecondary
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
            }
            Label {
                text: detailsPane.stashRef
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            HoverToolButton {
                text: qsTr("Apply")
                font.pixelSize: Theme.fontSm
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Apply this stash, keeping it")
                onClicked: detailsPane.applyStashRequested(detailsPane.stashRef)
            }
            HoverToolButton {
                text: qsTr("Pop")
                font.pixelSize: Theme.fontSm
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Apply this stash and drop it")
                onClicked: detailsPane.popStashRequested(detailsPane.stashRef)
            }
        }
    }
    // Inset on all four sides — the message box carries its own frame,
    // and flush against the header band the two borders read as one
    // welded block. Vertically the inset is the same step the rows
    // inside use, so band → summary → description → author → band is
    // one even rhythm; horizontally it is the pane inset, which puts
    // the card's edge under the header labels.
    ColumnLayout {
        Layout.fillWidth: true
        Layout.margins: Theme.spaceSm
        Layout.topMargin: Theme.spaceXs
        Layout.bottomMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        visible: detailsPane.details.shaHex !== ""

        // -- message first, like the commit editor: a prominent summary
        // box and a dimmer description box --
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: subjectArea.implicitHeight + Theme.spaceSm
            color: Theme.bgBase
            radius: Theme.radiusMd
            // While the question stands, the boxes it is about carry it:
            // the click that asked it happened over on the graph, and
            // nothing else would draw the eye back here.
            border.color: detailsPane.asking ? Theme.warning : Theme.borderDefault
            border.width: Theme.borderWidth
            SummaryArea {
                id: subjectArea
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                readOnly: !detailsPane.editable
                placeholderText: detailsPane.editable ? qsTr("Commit summary") : ""
                ToolTip.visible: hovered && detailsPane.editBlocked !== ""
                ToolTip.delay: 600
                ToolTip.text: detailsPane.editBlocked
            }
        }
        // Always shown, even empty, and two lines tall from the start —
        // the pair mirrors the commit editor's fields.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(Math.max(bodyArea.implicitHeight,
                                                      2 * Theme.fontMdLine)
                                             + Theme.spaceSm, 120)
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: detailsPane.asking ? Theme.warning : Theme.borderSubtle
            border.width: Theme.borderWidth
            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                // ScrollView keeps its Flickable private -- reach it
                // once it exists.
                Component.onCompleted:
                    contentItem.boundsBehavior = Flickable.StopAtBounds
                TextArea {
                    id: bodyArea
                    readOnly: !detailsPane.editable
                    wrapMode: TextArea.Wrap
                    placeholderText: detailsPane.editable ? qsTr("Description") : ""
                    font.pixelSize: Theme.fontMd
                    color: Theme.textSecondary
                    background: null
                    padding: 0
                }
            }
        }
        // Only once something is actually changed: until then the pane
        // keeps its resting shape and nothing invites a rewrite.
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            visible: detailsPane.messageDirty
            // The newest commit is amended in place and costs nothing;
            // an older one is replayed, and everything built on it
            // comes back as different commits. Only the second case is
            // worth a line.
            Label {
                Layout.fillWidth: true
                visible: !detailsPane.asking
                         && detailsPane.details.shaHex !== detailsPane.headOid
                wrapMode: Text.Wrap
                text: qsTr("Saving replays this commit, so every commit after "
                           + "it gets a new identity.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            // Said, not asked, like the amend editor's tag: the save
            // still goes ahead, and this line is the warning it gets.
            Label {
                Layout.fillWidth: true
                visible: !detailsPane.asking && detailsPane.published
                wrapMode: Text.Wrap
                text: qsTr("This commit is on a remote. Rewriting it leaves "
                           + "anyone who already has it out of step.")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
            }
            Label {
                Layout.fillWidth: true
                visible: detailsPane.asking
                wrapMode: Text.Wrap
                text: qsTr("Moving to another commit leaves this text behind.")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                Item { Layout.fillWidth: true }
                HoverToolButton {
                    visible: !detailsPane.asking
                    text: qsTr("Cancel")
                    font.pixelSize: Theme.fontSm
                    onClicked: detailsPane.revertMessage()
                }
                HoverButton {
                    visible: !detailsPane.asking
                    implicitHeight: Theme.controlHeight
                    highlighted: true
                    text: qsTr("Save message")
                    enabled: !detailsPane.busy && subjectArea.text.trim() !== ""
                    onClicked: detailsPane.submitMessage()
                }
                // The two ways out of the question. Staying is the
                // highlighted one: it is the answer that loses nothing.
                HoverToolButton {
                    visible: detailsPane.asking
                    text: qsTr("Discard edits")
                    font.pixelSize: Theme.fontSm
                    onClicked: detailsPane.leaveResolved(true)
                }
                HoverButton {
                    visible: detailsPane.asking
                    implicitHeight: Theme.controlHeight
                    highlighted: true
                    text: qsTr("Keep editing")
                    onClicked: detailsPane.leaveResolved(false)
                }
            }
        }
        // -- author card: avatar + name/date on the left, own hash over
        // parent hash on the right (rows aligned) --
        RowLayout {
            spacing: Theme.spaceSm
            IdentIcon {
                code: detailsPane.details.avatar
                width: Metrics.detailsAvatar
                height: Metrics.detailsAvatar
            }
            ColumnLayout {
                spacing: 0
                Layout.fillWidth: true
                Label {
                    id: authorLabel
                    text: detailsPane.details.authorName
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMd
                    font.weight: Font.DemiBold
                    ToolTip.visible: authorHover.containsMouse
                    ToolTip.delay: 400
                    ToolTip.text: qsTr("Author: %1 <%2>\nCommitter: %3")
                                  .arg(detailsPane.details.authorName)
                                  .arg(detailsPane.details.authorEmail)
                                  .arg(detailsPane.details.committer)
                    MouseArea {
                        id: authorHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }
                Label {
                    id: detailsDate
                    text: Qt.formatDateTime(new Date(detailsPane.details.authorTime * 1000),
                                            "yyyy-MM-dd HH:mm")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
            }
            ColumnLayout {
                spacing: 0
                Layout.alignment: Qt.AlignRight
                // The hash is the button, not just the icon beside it —
                // a 16px glyph was too small to aim at. Hovering
                // underlines the hash and lights the icon so the whole
                // plate reads as one control.
                // Not a HoverToolButton: the style's panel would make
                // the plate taller than one line and drop this hash out
                // of step with the author name beside it, so it draws
                // the same wash over its own flat face.
                ToolButton {
                    id: hashCopy
                    Layout.alignment: Qt.AlignRight
                    text: detailsPane.details.sha8
                    leftPadding: Theme.spaceXs
                    rightPadding: Theme.spaceXs
                    topPadding: 0
                    bottomPadding: 0
                    readonly property bool lit: hovered || visualFocus
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Copy full hash")
                    onClicked: detailsPane.copyRequested(detailsPane.details.shaHex)
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: hashCopy.down ? Theme.bgPressed
                             : hashCopy.lit ? Theme.bgHover
                             : "transparent"
                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.NoButton
                            cursorShape: Qt.PointingHandCursor
                        }
                        // Drawn here rather than as the label's font
                        // underline so the rule runs under the icon too —
                        // the hash and the icon are one target, so they
                        // get one line.
                        Rectangle {
                            visible: hashCopy.lit
                            color: Theme.textPrimary
                            height: Theme.borderWidth
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            anchors.leftMargin: hashCopy.leftPadding
                            anchors.rightMargin: hashCopy.rightPadding
                        }
                    }
                    contentItem: RowLayout {
                        spacing: Theme.spaceXs
                        Label {
                            text: hashCopy.text
                            font.family: Theme.monoFamily
                            font.pixelSize: Theme.fontMd
                            color: Theme.textPrimary
                            Layout.alignment: Qt.AlignVCenter
                        }
                        NavIcon {
                            kind: "copyicon"
                            tint: hashCopy.lit ? Theme.textPrimary
                                               : Theme.textSecondary
                            Layout.alignment: Qt.AlignVCenter
                        }
                    }
                }
                Label {
                    id: parentLink
                    visible: detailsPane.details.parentHex !== ""
                    Layout.alignment: Qt.AlignRight
                    text: "← " + detailsPane.details.parentHex.substring(0, 8)
                    font.family: Theme.monoFamily
                    color: Theme.textLink
                    font.pixelSize: Theme.fontSm
                    font.underline: parentHover.containsMouse
                    ToolTip.visible: parentHover.containsMouse
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Go to parent commit")
                    MouseArea {
                        id: parentHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: detailsPane.parentClicked(detailsPane.details.parentHex)
                    }
                }
            }
        }
    }
    // CHANGES header with the tree ⇄ path view toggle.
    Rectangle {
        visible: detailsPane.details.shaHex !== ""
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: qsTr("CHANGES (%1)").arg(detailsPane.details.fileTotal)
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
                onClicked: detailsPane.details.setTreeView(true)
                contentItem: NavIcon {
                    kind: "hier"
                    tint: detailsPane.details.treeView ? Theme.accent
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
                onClicked: detailsPane.details.setTreeView(false)
                contentItem: NavIcon {
                    kind: "list"
                    tint: detailsPane.details.treeView ? Theme.textMuted
                                                       : Theme.accent
                }
            }
        }
    }
    ListView {
        id: fileList
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: detailsPane.details
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {}
        delegate: FileRowDelegate {
            listWidth: fileList.width
            onActivated: (bucket, path, origPath) =>
                detailsPane.fileActivated(path, origPath)
            onFolderToggled: key => detailsPane.details.toggleFolder(key)
        }
    }
}
