pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// A form field that also offers a list: the name is typed, and the list is
// only ever a shortcut past typing it. Editable on purpose — what git will
// accept is wider than anything this app can enumerate.
//
// Drawn from tokens rather than left to Fusion, for the reason `AppMenu`
// is: the built-in popup's ground is `palette.base`, the same colour as
// the pane behind it, with no frame to read the edge by.
ComboBox {
    id: combo

    /// Stands in for the value while the field is empty. `TextInput` has
    /// no placeholder of its own, and the frame alone cannot say whether
    /// empty means "unset" or "not read yet".
    property string placeholder: ""

    /// A read is out for the list. The seat the arrow sits in turns
    /// instead — the list can take seconds to arrive, and an arrow over an
    /// empty list says "nothing" where "not yet" is the truth.
    property bool loading: false

    /// What the field should say, and the value to read back.
    ///
    /// **Not `editText`.** A model arriving makes ComboBox snap its
    /// `currentIndex` to the first row and drag the text along with it, so
    /// a list that lands seconds later would quietly replace a configured
    /// name — or one half typed — with whatever happens to sort first.
    /// Here the list is only ever a set of suggestions, so the text is
    /// held apart from it and put back once the swap has settled.
    /// Only two things move it, and both are a person: a row picked from
    /// the popup, and `textEdited` on the field below — which, unlike
    /// `editText` changing, is not emitted when the text is set in code.
    /// Nothing weaker works: ComboBox resets `currentIndex` *before* the
    /// QML `onModelChanged` runs, and it holds focus the whole time its
    /// popup is up, so neither focus nor a flag set here can tell that
    /// reset from typing.
    property string wanted: ""
    onWantedChanged: if (combo.editText !== combo.wanted) combo.editText = combo.wanted
    onActivated: index => combo.wanted = combo.textAt(index)
    // Undo the reset the swap caused, once it has finished happening.
    onModelChanged: Qt.callLater(() => {
        combo.editText = combo.wanted
    })

    editable: true
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd

    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: combo.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }

    // Own contentItem, so it does not go through palette (§無効).
    contentItem: TextInput {
        id: input
        text: combo.editText
        color: combo.enabled ? Theme.textPrimary : Theme.textMuted
        font: combo.font
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceSm
        verticalAlignment: Text.AlignVCenter
        selectByMouse: true
        selectionColor: Theme.bgSelected
        selectedTextColor: Theme.textPrimary
        onTextChanged: combo.editText = text
        onTextEdited: combo.wanted = text
        Label {
            anchors.fill: parent
            visible: input.text === ""
            text: combo.placeholder
            color: Theme.textMuted
            font: combo.font
            leftPadding: input.leftPadding
            verticalAlignment: Text.AlignVCenter
        }
    }

    indicator: Item {
        x: combo.width - width - Theme.spaceSm
        y: (combo.height - height) / 2
        width: Theme.iconMd
        height: Theme.iconMd
        NavIcon {
            anchors.fill: parent
            kind: combo.loading ? "spinner" : "chevron"
            tint: Theme.textSecondary
            // Only the turning one is animated; the arrow keeps the angle
            // it was drawn at (an animator leaves it where it stopped).
            rotation: combo.loading ? 0 : 90
            // On the render thread, so it keeps turning while the GUI
            // thread drains models.
            RotationAnimator on rotation {
                running: combo.loading && AppBackend.shotDir === ""
                loops: Animation.Infinite
                from: 0
                to: 360
                duration: Metrics.spinMs
            }
        }
    }

    popup: Popup {
        y: combo.height
        width: combo.width
        padding: Theme.spaceXs
        // The padding is the popup's, not the list's, so it has to be
        // added on — a height of just the rows leaves the last one cut.
        implicitHeight: Math.min(contentItem.implicitHeight, Theme.rowHeight * 8)
                        + topPadding + bottomPadding
        background: Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }
        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: combo.delegateModel
            currentIndex: combo.highlightedIndex
            ScrollBar.vertical: AutoScrollBar {}
        }
    }

    delegate: ItemDelegate {
        required property string modelData
        required property int index
        width: combo.width - 2 * Theme.spaceXs
        height: Theme.rowHeight
        highlighted: combo.highlightedIndex === index
        background: Rectangle {
            radius: Theme.radiusSm
            color: parent.highlighted ? Theme.bgHover : "transparent"
        }
        contentItem: Label {
            text: parent.modelData
            color: Theme.textPrimary
            font.pixelSize: Theme.fontMd
            leftPadding: Theme.spaceXs
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
    }
}
