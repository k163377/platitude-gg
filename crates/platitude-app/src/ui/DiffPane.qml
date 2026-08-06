pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Center pane, diff mode: one file's unified diff with per-file,
// per-hunk and per-line staging affordances, plus image / binary
// previews. Which file is shown and what staging means is the owner's
// business — this pane reports intents.
Rectangle {
    id: diffPane

    required property var diffModel
    // Whether the shown diff is a working-tree file (stageable).
    property bool fromWorkTree: false
    // Whether it is the staged side (flips the affordance wording).
    property bool staged: false
    // A write is running: staging buttons disable.
    property bool busy: false

    signal closeRequested()
    /// Stage or unstage the whole file (direction follows `staged`).
    signal stageFileRequested()
    /// Stage or unstage one hunk (line < 0) or one line of it.
    signal stageSelectionRequested(int hunk, int line)
    /// Throw one hunk (line < 0) or one line of it away. Offered on the
    /// unstaged side only — the staged side unstages first.
    signal discardSelectionRequested(int hunk, int line)
    /// The question bar over these lines was answered / walked away from.
    signal askConfirmed()
    signal askCancelled()

    // ---- a standing question about part of this diff ----------------
    // The same bar the graph and the file list raise, over the lines it
    // is about (デザイン規約 §可否・警告の出し場所). What it concerns is
    // marked rather than worded: one hunk, or the single line inside it.
    property int askHunk: -1
    property int askLine: -1
    function startAsking(hunk, line, label, detail, accept) {
        diffPane.askHunk = hunk
        diffPane.askLine = line
        askBar.label = label
        askBar.detail = detail
        askBar.accept = accept
    }
    function stopAsking() {
        diffPane.askHunk = -1
        diffPane.askLine = -1
        askBar.label = ""
    }
    /// Automation: answer it by holding the pill to the end.
    function completeHold() {
        askBar.completeHold()
    }
    /// Any other click in here walks away from the question, the way one
    /// anywhere else does — every one of them stages, unstages or leaves.
    function leaveAsk() {
        if (askBar.open)
            diffPane.askCancelled()
    }

    color: Theme.bgSurface
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            color: Theme.bgElevated
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceSm
                spacing: Theme.spaceSm
                Label {
                    text: qsTr("DIFF · %1").arg(diffPane.diffModel.title)
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: Theme.textSecondary
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
                HoverToolButton {
                    visible: diffPane.fromWorkTree
                    text: diffPane.staged ? qsTr("Unstage file")
                                          : qsTr("Stage file")
                    font.pixelSize: Theme.fontSm
                    enabled: !diffPane.busy
                    ToolTip.visible: hovered
                    ToolTip.delay: 300
                    ToolTip.text: diffPane.staged
                        ? qsTr("Unstage the whole file at once")
                        : qsTr("Stage the whole file at once")
                    onClicked: {
                        diffPane.leaveAsk()
                        diffPane.stageFileRequested()
                    }
                }
                HoverToolButton {
                    text: "×"
                    implicitWidth: Theme.iconLg
                    implicitHeight: Theme.iconLg
                    padding: 0
                    onClicked: diffPane.closeRequested()
                }
            }
        }
        // Between the header and the lines it is about, the same way it
        // stands over the graph and over the file list.
        AskBar {
            id: askBar
            Layout.fillWidth: true
            danger: true
            hold: true
            onConfirmed: diffPane.askConfirmed()
            onCancelled: diffPane.askCancelled()
        }
        // -- content preview: binaries summarized by size, images
        //    rendered (added = After only, deleted = Before only,
        //    modified = both).
        Label {
            visible: diffPane.diffModel.previewKind === "binary"
                     || (diffPane.diffModel.isBinary
                         && diffPane.diffModel.previewKind === "")
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: {
                const oldS = diffPane.diffModel.previewOldSize
                const newS = diffPane.diffModel.previewNewSize
                if (oldS !== "" && newS !== "")
                    return qsTr("Binary file · %1 → %2").arg(oldS).arg(newS)
                if (newS !== "")
                    return qsTr("Binary file · %1").arg(newS)
                if (oldS !== "")
                    return qsTr("Binary file removed · was %1").arg(oldS)
                return qsTr("Binary file — no text diff")
            }
            color: Theme.textMuted
        }
        RowLayout {
            visible: diffPane.diffModel.previewKind === "image"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Theme.spaceSm
            spacing: Theme.spaceSm
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                label: qsTr("Before · %1").arg(diffPane.diffModel.previewOldSize)
                url: diffPane.diffModel.previewOldUrl
                sizeText: diffPane.diffModel.previewOldSize
            }
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                label: qsTr("After · %1").arg(diffPane.diffModel.previewNewSize)
                url: diffPane.diffModel.previewNewUrl
                sizeText: diffPane.diffModel.previewNewSize
            }
        }
        ListView {
            id: diffList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: diffPane.diffModel
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AutoScrollBar {}
            // An image with no text rows hands its space to the preview
            // (SVG edits keep both).
            visible: diffPane.diffModel.previewKind !== "image"
                     || count > 0
            delegate: Rectangle {
                id: diffRow
                required property string kind
                required property int old_no
                required property int new_no
                required property string text
                required property int hunk
                required property int line
                width: diffList.width
                height: Theme.rowHeight
                color: kind === "add" ? Theme.diffAddedBg
                       : kind === "del" ? Theme.diffRemovedBg
                       : kind === "hunk" ? Theme.diffHunkHeaderBg
                       : "transparent"
                // The standing question is about this row: the whole hunk
                // when no line was named, that one line otherwise. The
                // words are on the bar; the rows answer "which".
                readonly property bool marked:
                    diffPane.askHunk >= 0 && diffRow.hunk === diffPane.askHunk
                    && (diffPane.askLine < 0 ? diffRow.kind !== "meta"
                                             : diffRow.line === diffPane.askLine)
                Rectangle {
                    anchors.fill: parent
                    color: Theme.bgHover
                    visible: diffRow.marked
                    Rectangle {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: Metrics.laneStroke
                        color: Theme.danger
                    }
                }
                Row {
                    anchors.fill: parent
                    spacing: 0
                    Label {
                        width: 42
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.spaceXs
                        color: Theme.textMuted
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        width: 42
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.spaceXs
                        color: Theme.textMuted
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        width: parent.width - 84
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.text
                        elide: Text.ElideRight
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontMd
                        color: diffRow.kind === "add" ? Theme.diffAddedFg
                               : diffRow.kind === "del" ? Theme.diffRemovedFg
                               : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                               : diffRow.kind === "meta" ? Theme.textMuted
                               : Theme.textPrimary
                    }
                }
                MouseArea {
                    id: lineHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    enabled: diffPane.fromWorkTree
                }
                // Hunk-level staging. The row carries the hunk index the
                // patch builder needs, so what is staged is exactly what
                // is shown — and so is what is thrown away.
                Row {
                    visible: diffPane.fromWorkTree && diffRow.kind === "hunk"
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spaceSm
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.spaceXs
                    // Only the unstaged side has a piece to throw away: on
                    // the staged side the button beside this one puts the
                    // hunk back where it can be.
                    HoverToolButton {
                        visible: !diffPane.staged
                        text: qsTr("Discard hunk…")
                        font.pixelSize: Theme.fontSm
                        enabled: !diffPane.busy
                        onClicked: diffPane.discardSelectionRequested(diffRow.hunk, -1)
                    }
                    HoverToolButton {
                        text: diffPane.staged ? qsTr("Unstage hunk")
                                              : qsTr("Stage hunk")
                        font.pixelSize: Theme.fontSm
                        enabled: !diffPane.busy
                        onClicked: {
                            diffPane.leaveAsk()
                            diffPane.stageSelectionRequested(diffRow.hunk, -1)
                        }
                    }
                }
                // Line-level staging.
                Rectangle {
                    visible: diffPane.fromWorkTree && lineHover.containsMouse
                             && (diffRow.kind === "add"
                                 || diffRow.kind === "del")
                    x: Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.iconMd
                    height: Theme.iconMd
                    radius: Theme.radiusSm
                    color: Theme.bgElevated
                    border.color: Theme.borderStrong
                    border.width: Theme.borderWidth
                    ToolTip.visible: stageLineHover.containsMouse
                    ToolTip.delay: 300
                    ToolTip.text: diffPane.staged ? qsTr("Unstage this line")
                                                  : qsTr("Stage this line")
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: stageLineHover.containsMouse
                    }
                    NavIcon {
                        anchors.centerIn: parent
                        width: Theme.iconSm
                        height: Theme.iconSm
                        kind: diffRow.kind === "add" ? "plus" : "minus"
                        tint: Theme.textPrimary
                    }
                    MouseArea {
                        id: stageLineHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            diffPane.leaveAsk()
                            diffPane.stageSelectionRequested(diffRow.hunk,
                                                             diffRow.line)
                        }
                    }
                }
                // The same square again, for throwing that one line away.
                // Unstaged side only, for the same reason as the hunk's.
                Rectangle {
                    visible: diffPane.fromWorkTree && !diffPane.staged
                             && lineHover.containsMouse
                             && (diffRow.kind === "add"
                                 || diffRow.kind === "del")
                    x: Theme.spaceXs + Theme.iconMd + Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.iconMd
                    height: Theme.iconMd
                    radius: Theme.radiusSm
                    color: Theme.bgElevated
                    border.color: Theme.borderStrong
                    border.width: Theme.borderWidth
                    ToolTip.visible: discardLineHover.containsMouse
                    ToolTip.delay: 300
                    ToolTip.text: qsTr("Discard this line")
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: discardLineHover.containsMouse
                    }
                    Label {
                        anchors.centerIn: parent
                        text: "×"
                        font.pixelSize: Theme.fontSm
                        color: Theme.danger
                    }
                    MouseArea {
                        id: discardLineHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: diffPane.discardSelectionRequested(diffRow.hunk,
                                                                      diffRow.line)
                    }
                }
            }
        }
    }
}
