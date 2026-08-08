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
    /// Throw one hunk away. Offered on the unstaged side only — the
    /// staged side unstages first. No question comes before it: the
    /// heading's own button is held down (デザイン規約 §その他の操作).
    signal discardHunkRequested(int hunk)

    /// Automation: hold the heading's discard button to its end. The
    /// hunk is the first one, which is the one every smoke run acts on.
    function completeHold() {
        const row = diffList.itemAtIndex(0)
        if (row && row.discardButton)
            row.discardButton.completeHold()
    }

    // ---- automation: the tools a line only shows under the pointer ----
    // Hover cannot be injected on Windows (CLAUDE.md), so the squares a
    // line puts out — stage this line, throw it away — have no headless
    // way to be seen. These name a row as though the pointer were on it,
    // the way `ref-list` enters where the hover timer would. Nothing in
    // the app writes them: the pointer is the only other way in.
    property int hoverHunk: -1
    property int hoverLine: -1
    function showLineTools(hunk, line) {
        diffPane.hoverHunk = hunk
        diffPane.hoverLine = line
    }
    /// The first line of a hunk that a partial write can act on. A hunk's
    /// lines are numbered through the context it carries, so line 0 is
    /// usually a line that is not part of the change at all — selecting
    /// one builds a patch with nothing in it, which core refuses ("the
    /// selected part is no longer in its diff"). A pointer never has this
    /// problem: it picks its line by being over it, and the squares only
    /// come out on the lines that changed. -1 when the hunk has none.
    function firstChangedLine(hunk) {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.hunk === hunk
                    && (row.kind === "add" || row.kind === "del"))
                return row.line
        }
        return -1
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
                    onClicked: diffPane.stageFileRequested()
                }
                HoverToolButton {
                    implicitWidth: Theme.iconLg
                    implicitHeight: Theme.iconLg
                    padding: 0
                    contentItem: Item {
                        NavIcon {
                            anchors.centerIn: parent
                            width: Theme.iconMd
                            height: Theme.iconMd
                            kind: "close"
                            tint: Theme.textPrimary
                        }
                    }
                    onClicked: diffPane.closeRequested()
                }
            }
        }
        // No question bar here: the only thing this pane throws away is a
        // hunk, and that is held down on the hunk's own heading
        // (デザイン規約 §その他の操作).
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
                // Nothing marks a row any more: what a hunk's discard
                // takes is the hunk its heading sits on, and the button is
                // in that heading. Reached from outside for the smoke run,
                // which holds it the way a hand does.
                readonly property alias discardButton: discardHunkButton
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
                // Under the pointer, or named as if it were (see above).
                readonly property bool underPointer:
                    lineHover.containsMouse
                    || (diffPane.hoverHunk === diffRow.hunk
                        && diffPane.hoverLine === diffRow.line)
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
                    //
                    // Held, not asked about (デザイン規約 §長押し): the
                    // button sits in the hunk's own heading, so what it
                    // takes is the thing it is standing on, and a bar
                    // coming down over the diff to say so is machinery a
                    // hunk does not need.
                    //
                    // Shaped like the held row of a right-click menu, not
                    // like the toolbar's framed button: this one sits in a
                    // line of other words rather than in a row of other
                    // buttons, and a frame around one word in a heading
                    // reads as a box that has come loose. The mark says it
                    // is held, and the hold fills the words' own ground
                    // edge to edge.
                    ActionButton {
                        id: discardHunkButton
                        visible: !diffPane.staged
                        text: qsTr("Discard hunk")
                        font.pixelSize: Theme.fontSm
                        tone: Theme.danger
                        holdTone: Theme.danger
                        holdMs: Metrics.holdMs
                        enabled: !diffPane.busy
                        onHeld: diffPane.discardHunkRequested(diffRow.hunk)
                    }
                    // The same pair of colours the file rows put on their
                    // own `+` and `−`: staging is the green half of the
                    // gesture and unstaging the red one, and the heading
                    // should not name them in a different voice than the
                    // list does.
                    ActionButton {
                        text: diffPane.staged ? qsTr("Unstage hunk")
                                              : qsTr("Stage hunk")
                        font.pixelSize: Theme.fontSm
                        tone: diffPane.staged ? Theme.diffRemovedFg
                                              : Theme.diffAddedFg
                        enabled: !diffPane.busy
                        onActivated: diffPane.stageSelectionRequested(diffRow.hunk, -1)
                    }
                }
                // Line-level staging.
                Rectangle {
                    visible: diffPane.fromWorkTree && diffRow.underPointer
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
                        onClicked: diffPane.stageSelectionRequested(diffRow.hunk,
                                                                   diffRow.line)
                    }
                }
                // No square for throwing one line away. A line can be
                // staged on its own because staging loses nothing — the
                // line stays on disk either way — but the smallest thing
                // that can be thrown away is a hunk (デザイン規約
                // §その他の操作): a bare `×` has no words to say what it
                // takes, and a control that must be held has to say it.
            }
        }
    }
}
