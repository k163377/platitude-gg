pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The refs a graph row could only stack: one chip and a "+N". Hovering
// that chip opens this under it, with every ref on a row of its own —
// branches first, then the tags the chip had no room for. The branches
// are what can be moved to; the tags are read here and nowhere else.
//
// Owned by the page, not by the delegate that raised it — delegates are
// recycled out from under an open popup, which is why the context menus
// live there too. It is a popup rather than an item in the row for the
// same reason a menu is: anything declared inside the list is clipped by
// it and painted under the row below.
Popup {
    id: refList

    /// Chip records (kind + flags + name, see encode.rs), as shown.
    property var records: []
    /// The branch the working tree is on; that one leads nowhere.
    property string currentBranch: ""
    /// One was chosen; the whole record, so its kind travels with it.
    signal picked(string record)

    padding: Theme.spaceXs
    // Nothing stands between the chip and this: the pointer has to be
    // able to walk down into it without leaving both.
    margins: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    /// Whether the pointer is over this. `HoverHandler` rather than a
    /// `MouseArea`: handlers are passive, so the rows' own hover does not
    /// take this one's away.
    readonly property alias pointerInside: insideHover.hovered

    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
    }

    contentItem: Column {
        id: rows
        // A Column takes its width from the widest child, and the rows
        // have to reach that width for their highlight to line up.
        readonly property real rowWidth: {
            let widest = 0
            for (let i = 0; i < rows.children.length; i++)
                widest = Math.max(widest, rows.children[i].implicitWidth)
            return widest
        }
        HoverHandler {
            id: insideHover
        }
        Repeater {
            model: refList.records
            delegate: Rectangle {
                id: refRow
                required property string modelData
                // The branch the working tree already stands on is not a
                // place to go, but it is not unavailable either — it is
                // where the reader is, and it says so in the colour the
                // sidebar says it in (§ref の種別). Only the hover and the
                // click come off. A tag leads nowhere for its own reason
                // (§タグでは detach しない) and keeps its colour too.
                readonly property bool current:
                    refRow.modelData[0] === "L"
                    && refRow.modelData.substring(5).split("\u001E")[0] === refList.currentBranch
                // The detached-HEAD marker is the one row that is only a
                // marker: nowhere to go and no ref to read, so it mutes.
                readonly property bool unavailable: refRow.modelData[0] === "H"
                readonly property bool leadsNowhere:
                    refRow.unavailable || refRow.current
                    || refRow.modelData[0] === "T"

                implicitWidth: rowChip.width + 2 * Theme.spaceSm
                               + (whose.visible
                                  ? whose.implicitWidth + Theme.spaceSm : 0)
                width: rows.rowWidth
                height: Theme.rowHeight
                radius: Theme.radiusSm
                color: rowHover.hovered && !refRow.leadsNowhere
                       ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    anchors.verticalCenter: parent.verticalCenter
                    x: Theme.spaceSm
                    records: [refRow.modelData]
                    muted: refRow.unavailable
                    // Unstacking is only worth it if the names read; the
                    // window the popup opens over is the only limit.
                    maxWidth: refList.parent ? refList.parent.width : 400
                }
                // Whose reading this is. A tag has no namespace to say it
                // in the way `origin/main` does, and a drifted one puts the
                // same bare name on two rows — this card is where the two
                // meet, so it is where the question gets answered. Meta
                // about the row rather than part of the name, so it is
                // written in the colour the `+N` is (§ref の種別).
                Label {
                    id: whose
                    visible: rowChip.recWhere !== ""
                    anchors.left: rowChip.right
                    anchors.leftMargin: Theme.spaceSm
                    anchors.verticalCenter: parent.verticalCenter
                    text: rowChip.recWhere
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
                HoverHandler {
                    id: rowHover
                }
                TapHandler {
                    enabled: !refRow.leadsNowhere
                    onTapped: {
                        refList.close()
                        refList.picked(refRow.modelData)
                    }
                }
            }
        }
    }
}
