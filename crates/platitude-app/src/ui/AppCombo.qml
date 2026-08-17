pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// A form field that also offers a list: the name is typed, and the list is only ever a shortcut past typing it.
// Editable on purpose — what git will accept is wider than anything this app can enumerate.
//
// Drawn from tokens rather than left to Fusion, for the reason `AppMenu` is: the built-in popup's ground is
// `palette.base`, the same colour as the pane behind it, with no frame to read the edge by.
ComboBox {
    id: combo

    /// Stands in for the value while the field is empty. `TextInput` has no placeholder of its own, and the frame alone
    /// cannot say whether empty means "unset" or "not read yet".
    property string placeholder: ""

    /// The list is the whole set of answers rather than a shortcut past typing one: nothing outside it means anything
    /// (a branch goes to a remote this repository has, or to one the same question makes). The field stops taking text
    /// and becomes the value it shows, and the owner keeps `wanted` — picking a row reports the choice instead of
    /// writing it, so a binding on `wanted` is not overwritten from here.
    property bool pickOnly: false

    /// A read is out for the list. The seat the arrow sits in turns instead — the list can take seconds to arrive, and
    /// an arrow over an empty list says "nothing" where "not yet" is the truth.
    property bool loading: false

    /// The last row is not one of the answers but the way to one this list does not hold yet. Set off by the line a
    /// menu separates its groups with (デザイン規約 §メニュー): a row that acts and a row that answers cannot be told apart while
    /// both are plain text on one ground, and the list is read before it is clicked.
    property bool lastRowActs: false

    /// What the field should say, and the value to read back.
    ///
    /// **Not `editText`.** A model arriving makes ComboBox snap its `currentIndex` to the first row and drag the text
    /// along with it, so a list that lands seconds later would quietly replace a configured name — or one half typed —
    /// with whatever happens to sort first. Here the list is only ever a set of suggestions, so the text is held apart
    /// from it and put back once the swap has settled. Only two things move it, and both are a person: a row picked
    /// from the popup, and `textEdited` on the field below — which, unlike `editText` changing, is not emitted when the
    /// text is set in code. Nothing weaker works: ComboBox resets `currentIndex` *before* the QML `onModelChanged`
    /// runs, and it holds focus the whole time its popup is up, so neither focus nor a flag set here can tell that
    /// reset from typing.
    property string wanted: ""
    onWantedChanged: if (!combo.pickOnly && combo.editText !== combo.wanted) combo.editText = combo.wanted
    onActivated: index => {
        if (!combo.pickOnly)
            combo.wanted = combo.textAt(index)
    }
    // Undo the reset the swap caused, once it has finished happening.
    onModelChanged: Qt.callLater(() => {
        if (!combo.pickOnly)
            combo.editText = combo.wanted
    })

    editable: !combo.pickOnly
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd

    // The ground says whether there is anything to type here. `bgBase` is the inside of an input (デザイン規約 §色 背景), and a
    // chooser has no inside — it carries the value it shows and nothing can be put into it. Left bare it reads as the
    // button it is, so where a chooser and a name box stand side by side the recessed ground belongs to exactly one of
    // them (デザイン規約 §選ぶ欄と打つ欄).
    background: Rectangle {
        color: combo.pickOnly ? "transparent" : Theme.bgBase
        radius: Theme.radiusSm
        border.color: combo.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }

    // Own contentItem, so it does not go through palette (§無効).
    contentItem: TextInput {
        id: input
        // Picking only, the value is the owner's and never `editText`: ComboBox does not maintain that outside an
        // editable field.
        text: combo.pickOnly ? combo.wanted : combo.editText
        color: combo.enabled ? Theme.textPrimary : Theme.textMuted
        font: combo.font
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceSm
        verticalAlignment: Text.AlignVCenter
        readOnly: combo.pickOnly
        selectByMouse: !combo.pickOnly
        selectionColor: Theme.bgSelected
        selectedTextColor: Theme.textPrimary
        onTextChanged: if (!combo.pickOnly) combo.editText = text
        onTextEdited: combo.wanted = text
        // A read-only input still takes the press, so the control it sits in would never see the click that opens its
        // list.
        MouseArea {
            anchors.fill: parent
            enabled: combo.pickOnly
            onClicked: {
                if (combo.popup.opened)
                    combo.popup.close()
                else
                    combo.offer()
            }
        }
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
            id: seatMark
            anchors.fill: parent
            // Nothing to open, nothing to point at (see `hasList`).
            visible: combo.hasList
            // The ring turns in one place at a time: here while the card is shut, inside the card once it is up — two
            // of them a row apart would be one waiting said twice.
            readonly property bool waits: combo.loading && !combo.popup.opened
            kind: waits ? "spinner" : "chevron"
            tint: Theme.textSecondary
            // Only the turning one is animated; the arrow keeps the angle it was drawn at (an animator leaves it where
            // it stopped).
            rotation: waits ? 0 : 90
            // On the render thread, so it keeps turning while the GUI thread drains models.
            RotationAnimator on rotation {
                running: seatMark.waits && AppBackend.shotDir === ""
                loops: Animation.Infinite
                from: 0
                to: 360
                duration: Metrics.spinMs
            }
        }
    }

    /// Opens the list. A read still out counts as something to open — the card says so with its own ring (see
    /// `hasList`).
    function offer() {
        if (combo.hasList)
            combo.popup.open()
    }

    /// Whether there is anything to open at all — rows, or a read that may still bring some.
    ///
    /// A read that is still out is not the same as an empty list, and the card is where that difference gets said: it
    /// opens, and the ring inside it says the rows are not here yet (デザイン規約 §進行中・長押しの定数 — 進行中を言うのはリングの 仕事). Refusing to
    /// open while the read is out would leave the press with no answer at all, which reads as broken.
    ///
    /// With neither, the field is just a box to type in and says so by dropping the seat's mark: a chevron over a list
    /// that can never open is a lie.
    readonly property bool hasList: combo.count > 0 || combo.loading
    // The read came back with nothing while the card was up. Nothing is an answer, and the card has no way to say it —
    // so the card goes, and the mark goes with it.
    onHasListChanged: if (!combo.hasList) combo.popup.close()

    popup: Popup {
        y: combo.height
        width: combo.width
        padding: Theme.spaceXs
        // The other way in is the control's own press, which Qt takes before anything here sees it.
        //
        // The arrows start from the answer as well, so the wash that says where they are does not land on a row nobody
        // picked while the picked one sits beside it wearing another ground. Only where the list is the whole set of
        // answers: moving `currentIndex` on an editable one drags the text with it (see `wanted`).
        onAboutToShow: {
            if (!combo.hasList) {
                combo.popup.close()
                return
            }
            if (combo.pickOnly)
                combo.currentIndex = combo.find(combo.wanted)
        }
        // The padding is the popup's, not the list's, so it has to be added on — a height of just the rows leaves the
        // last one cut.
        implicitHeight: Math.min(contentItem.implicitHeight, Theme.rowHeight * 8) + topPadding + bottomPadding
        background: AppCardFace {}
        // A card with a ring in it while the rows are still being read, and the rows themselves once they land. The
        // height is a row's worth for the ring to stand in, so the card that opens on a press is the same size as the
        // card that answers it.
        contentItem: Item {
            implicitHeight: rows.count > 0 ? rows.contentHeight : Theme.rowHeight
            SpinnerIcon {
                anchors.centerIn: parent
                width: Theme.iconMd
                height: Theme.iconMd
                spinning: rows.count === 0 && combo.loading
            }
            ListView {
                id: rows
                anchors.fill: parent
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                implicitHeight: contentHeight
                model: combo.delegateModel
                currentIndex: combo.highlightedIndex
                ScrollBar.vertical: AutoScrollBar {}
            }
        }
    }

    delegate: ItemDelegate {
        id: row
        required property string modelData
        required property int index
        /// This row acts rather than answers — see `lastRowActs`.
        readonly property bool acts: combo.lastRowActs && row.index === combo.count - 1
        /// What the line above it costs: the separator and the air the menu gives one on each side (デザイン規約 §メニュー).
        readonly property int lead: row.acts ? 2 * Theme.spaceXs + Theme.borderWidth : 0
        /// The answer this list is already carrying. Without it the list opens with a row washed that has nothing to do
        /// with the value — Qt puts its highlight on whatever the keyboard would move from — so the one row the reader
        /// came to find is the one row nothing points at (デザイン規約 §選ぶ欄と打つ欄).
        readonly property bool current:
            !row.acts && row.modelData !== "" && row.modelData === (combo.pickOnly ? combo.wanted : combo.editText)
        width: combo.width - 2 * Theme.spaceXs
        height: Theme.rowHeight + row.lead
        topPadding: row.lead
        highlighted: combo.highlightedIndex === row.index
        // Two rectangles rather than one: the wash belongs to the row, and a wash drawn over the whole item would
        // swallow the line that is there to keep the two apart.
        background: Item {
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                y: Theme.spaceXs
                height: Theme.borderWidth
                color: Theme.borderSubtle
                visible: row.acts
            }
            // Two grounds, because they answer two questions: which row is the value (§色 bgSelected = 選択行) and which
            // row the hand is on. The wash is an overlay colour, so it lies over the selected ground without either one
            // being lost.
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.rowHeight
                radius: Theme.radiusSm
                color: row.current ? Theme.bgSelected : "transparent"
            }
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.rowHeight
                radius: Theme.radiusSm
                color: row.highlighted ? Theme.bgHover : "transparent"
            }
        }
        contentItem: Label {
            text: row.modelData
            color: Theme.textPrimary
            font.pixelSize: Theme.fontMd
            leftPadding: Theme.spaceXs
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
    }
}
