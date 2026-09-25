pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The diff pane's header band: which file is open, the one word that stages the whole of it, and the way out.
Rectangle {
    id: header

    /// What the pane is showing; empty when nothing is open.
    required property string title
    /// Whether the shown diff is a working-tree file (stageable), which side it is, whether git stopped on it, and
    /// whether a write is running (`DiffPane`).
    required property bool fromWorkTree
    required property bool staged
    /// Whether the file is this window's own to write (`DiffPane.writable`); the stage word stands only then
    /// (デザイン規約 §別の作業コピーを読む).
    required property bool writable
    /// The copy the file belongs to, where it is not this window's — said after the path, which leads
    /// (デザイン規約 §別の作業コピーを読む).
    required property string copyName
    required property bool conflicted
    required property bool busy
    /// Whether the rows are read as two columns (`DiffModel.split`) — what the toggle shows
    /// (デザイン規約 §diff を 2 列で読む).
    required property bool split

    /// The band's sweep hand, named so a run can enter it (as a card's is at `<card>.background.pad`).
    property alias pad: headerHand
    /// Automation: the toggle itself, so a run flips the view where a press does (`PGG_AUTO_ACT=diff-split`).
    readonly property alias viewToggle: viewToggle
    /// Whether the path is cut — the output side, which a picture of a wide pane cannot answer.
    readonly property alias cut: titleField.clipped
    /// Automation: whether the stage word is there to press, read off the button — `writable` would go green with
    /// the button unwired.
    readonly property bool stageOffered: stageFileButton.visible && stageFileButton.enabled

    signal stageFileRequested()
    signal closeRequested()
    signal splitChosen(bool split)

    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    // The row under it can be a hunk heading of the same colour (デザイン規約 §diff の中のステージ).
    BandRule { z: 1 }
    // The band's air hands presses to the path (規約 §右のペインの字は掴める). Under the row, so the buttons keep
    // their presses.
    SweepPad {
        id: headerHand
        anchors.fill: parent
        content: headerRow
    }
    RowLayout {
        id: headerRow
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceSm
        // Caption and path with no separator between them; the band's one separator is the dash before the copy.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("DIFF")
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            // The path, in a field the reader can copy from (規約 §右のペインの字は掴める), cut in the middle: both
            // ends tell a path apart, and the field keeps the whole path, so a copy never carries the `…`.
            LineText {
                id: titleField
                // Takes the slack but no more than the path, or the copy's name after it strands at the far edge.
                Layout.fillWidth: true
                Layout.maximumWidth: titleField.implicitWidth
                text: header.title
                pixelSize: Theme.fontMd
                weight: Font.DemiBold
                color: Theme.textSecondary
                cutAt: "middle"
                ground: Theme.bgElevated
            }
            // Whose file: a dash, then the mark and the name, as everywhere a copy is named (デザイン規約
            // §別の作業コピーを読む). Outside the field, so a copied path stays a path.
            Label {
                visible: header.copyName !== ""
                text: "—"
                font.pixelSize: Theme.fontMd
                color: Theme.textSecondary
            }
            // A quarter of the band at most, so a long copy name cannot push the path off; it cuts its own middle.
            RowLayout {
                visible: header.copyName !== ""
                // A nested layout fills by default; the slack is the path's.
                Layout.fillWidth: false
                Layout.maximumWidth: header.width / 4
                spacing: 0
                Item {
                    Layout.preferredWidth: copyMark.inkWidth
                    Layout.preferredHeight: Theme.iconSm
                    Layout.alignment: Qt.AlignVCenter
                    NavIcon {
                        id: copyMark
                        anchors.centerIn: parent
                        kind: "tree"
                        tint: Theme.textSecondary
                        width: Theme.iconSm
                        height: Theme.iconSm
                    }
                }
                CutName {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.rowHeight
                    text: header.copyName
                    pixelSize: Theme.fontMd
                    weight: Font.DemiBold
                    // The band's colour, not a step quieter: the name is part of the phrase, not an aside.
                    color: Theme.textSecondary
                }
            }
        }
        // The band's spare room goes to this spacer: a layout with room nothing takes spreads it between its items.
        // The three after it say `fillWidth: false`, since a nested layout fills by default. The toggle stands on every
        // diff (デザイン規約 §diff を 2 列で読む).
        Item { Layout.fillWidth: true }
        DiffViewToggle {
            id: viewToggle
            Layout.fillWidth: false
            split: header.split
            onChosen: split => header.splitChosen(split)
        }
        // The pane's one standing colour word (デザイン規約 §diff の中のステージ).
        ActionButton {
            id: stageFileButton
            Layout.fillWidth: false
            visible: header.fromWorkTree && header.writable
            // On a conflicted file the same `git add` marks it resolved (デザイン規約 §diff の中のステージ).
            text: header.conflicted ? qsTr("Mark resolved")
                  : header.staged ? qsTr("Unstage file") : qsTr("Stage file")
            tone: header.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
            enabled: !header.busy
            tip: header.conflicted ? qsTr("Takes the file as it stands now") : header.staged
                 ? qsTr("Unstage the whole file at once") : qsTr("Stage the whole file at once")
            onActivated: header.stageFileRequested()
        }
        // The third of the band's end trio: the toggle pair's `iconXl` seat, band-tall reach and mark box
        // (規約 §diff を 2 列で読む).
        CloseToolButton {
            Layout.fillWidth: false
            Layout.fillHeight: true
            seat: Theme.iconXl
            markSize: Theme.iconXl
            topInset: Math.round((height - Theme.iconLg) / 2)
            bottomInset: topInset
            onClicked: header.closeRequested()
        }
    }
}
