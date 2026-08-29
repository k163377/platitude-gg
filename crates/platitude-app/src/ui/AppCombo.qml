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

    /// A row that wears the push mark, empty for a list with nothing to mark. The wash says which row is picked now
    /// and the mark says which one the repository sends pushes to: two questions, two answers, one row (デザイン規約
    /// §リモートを書き留める).
    property string markedRow: ""

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
        // The same frame-to-word inset the plain boxes keep (`SlimField`) — this field stands beside one of them in
        // the question bar, and two boxes a row apart holding their text at different distances read as two different
        // kinds of box (デザイン規約 §選ぶ欄と打つ欄 already has the ground saying which is which).
        leftPadding: Theme.spaceXs
        rightPadding: Theme.spaceXs
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
            onClicked: combo.pressField()
        }
        // Where the name can also be typed, Qt hands the press straight to the input and only the small mark at the
        // right edge opens anything — so the answers this field already knows stay hidden behind a target the width of
        // an icon. A handler rather than a MouseArea, because the input still needs that press to put the caret where
        // it was aimed; and a tap rather than a press, so dragging a selection out of the text does not drop the list
        // over what is being selected.
        TapHandler {
            enabled: !combo.pickOnly
            onTapped: combo.pressField()
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
        id: seat
        x: combo.width - width - Theme.spaceSm
        y: (combo.height - height) / 2
        width: Theme.iconMd
        height: Theme.iconMd
        // Nothing to open, nothing to point at (see `hasList`).
        visible: combo.hasList
        // The ring turns in one place at a time: here while the card is shut, inside the card once it is up — two of
        // them a row apart would be one waiting said twice.
        readonly property bool waits: combo.loading && !combo.popup.opened
        // Two marks in one seat rather than one mark that changes kind, because an animator **takes** the property it
        // turns: the ring's first frame kills the binding that stands the arrow on end, and nothing puts it back when
        // the ring stops — so the seat kept whatever angle the last frame left, and the arrow that came back pointed
        // sideways at a list that comes **down** (observed — the merge editor's seat, after its
        // `--tool-help` read landed). Invisible in headless, where the ring is held still for the camera and the
        // binding therefore survives. Split, the arrow is never animated and has no angle to lose.
        SpinnerIcon {
            anchors.fill: parent
            spinning: seat.waits
        }
        NavIcon {
            anchors.fill: parent
            visible: !seat.waits
            kind: "chevron"
            tint: Theme.textSecondary
            // Drawn pointing right, stood on end here: down is where the list comes from.
            rotation: 90
        }
    }

    /// What a press on the field means, wherever the press came from.
    ///
    /// Picking only, the field is a button and one press is the whole errand, so it toggles. Where the name can also be
    /// typed, the press is asking two things at once — "let me type here" and "show me what you already know" — and
    /// they are not in conflict: the caret lands where it was pressed and the list comes down beside it, so nobody has
    /// to find the mark at the right edge to learn there were answers. A second press inside the text is someone moving
    /// that caret, not putting the list away; the mark at the edge and Escape are what shut it.
    function pressField() {
        if (combo.pickOnly) {
            if (combo.popup.opened)
                combo.popup.close()
            else
                combo.offer()
            return
        }
        // Redundant under a real press — the input takes the caret itself — and the whole of what "was pressed" can
        // mean to the automation verb, which has no pointer to put anywhere.
        input.forceActiveFocus()
        // After the release, not inside it. The input owns the exclusive grab of a press this handler is only a passive
        // witness to, and a list opened while that grab is still being unwound is taken straight back down: it reads
        // `opened` true on the next line and false a frame later, so the field answers a press by flickering and
        // staying shut. One turn of the loop later it stands (observed, measured with an injected click —
        // qmltestrunner, since a press cannot be put into the app itself). The chooser above wants none of this: its
        // MouseArea holds the grab itself, so there is nothing to unwind under the list.
        Qt.callLater(combo.offer)
    }

    /// Whether a letter typed right now would land in the field. The list coming down must not take the caret with it,
    /// and that is not a thing a photograph can answer.
    readonly property bool typing: !combo.pickOnly && input.activeFocus

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
        // Outside the *field*, not outside the card: the field's own press is what opens the list now, and under the
        // default policy that same press closes it first — leaving a press that flickers the list instead of dropping
        // it, and a chooser whose second press could never close it at all.
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
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
                // The style's see-through thumb rather than the panels' flat slab (デザイン規約 §色 スクロールバー):
                // this bar stands inside a card and the rows run under it, which is the frame's answer rather than a
                // pane's edge. Over the card's own ground the same ink reads `#303F54`.
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
        contentItem: Item {
            Label {
                anchors.fill: parent
                anchors.rightMargin: mark.visible ? Theme.iconSm + Theme.spaceXs : 0
                text: row.modelData
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMd
                leftPadding: Theme.spaceXs
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
            // The same mark the left menu puts on that remote's own row, at the same step and in the same colour.
            NavIcon {
                id: mark
                visible: !row.acts && row.modelData !== "" && row.modelData === combo.markedRow
                kind: "push"
                tint: Theme.accent
                width: Theme.iconSm
                height: Theme.iconSm
                anchors.right: parent.right
                anchors.rightMargin: Theme.spaceXs
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }
}
