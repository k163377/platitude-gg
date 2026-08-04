pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The refs a graph row could only stack: one chip and a "+N". Hovering
// that chip opens this under it, with every ref on a row of its own so
// one of them can be moved to.
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
                // Where the working tree already is, and the detached-HEAD
                // marker, are not places to go.
                readonly property bool dead:
                    refRow.modelData[0] === "H"
                    || (refRow.modelData[0] === "L"
                        && refRow.modelData.substring(4) === refList.currentBranch)

                implicitWidth: rowChip.width + 2 * Theme.spaceSm
                width: rows.rowWidth
                height: Theme.rowHeight
                radius: Theme.radiusSm
                color: rowHover.hovered && !refRow.dead ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    anchors.verticalCenter: parent.verticalCenter
                    x: Theme.spaceSm
                    records: [refRow.modelData]
                    tagStyle: refRow.modelData[0] === "T"
                    muted: refRow.dead
                    // Unstacking is only worth it if the names read; the
                    // window the popup opens over is the only limit.
                    maxWidth: refList.parent ? refList.parent.width : 400
                }
                HoverHandler {
                    id: rowHover
                }
                TapHandler {
                    enabled: !refRow.dead
                    onTapped: {
                        refList.close()
                        refList.picked(refRow.modelData)
                    }
                }
            }
        }
    }
}
