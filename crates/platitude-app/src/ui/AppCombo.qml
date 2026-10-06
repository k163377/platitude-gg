pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// A form field that also offers a list, only ever a shortcut past typing: what git accepts is wider than anything this
// app can enumerate. Drawn from tokens: the built-in popup's ground is `palette.base` with no frame, lost on the pane.
ComboBox {
    id: combo

    /// Shown while the field is empty (`TextInput` has no placeholder of its own).
    property string placeholder: ""

    /// The list is the whole set of answers: the field takes no text, and the owner keeps `wanted` — picking a row
    /// reports the choice without overwriting a binding on `wanted`.
    property bool pickOnly: false

    /// A read is out for the list; the arrow's seat turns instead, since an arrow over an empty list says "nothing".
    property bool loading: false

    /// The width at which the value stands uncut — the word, the input's insets, the control's left one, and the width
    /// the control reserves for the arrow whether drawn or not (`PublishForm`). Rounded up and one over: a field
    /// exactly its name's width comes back elided by the rounding.
    readonly property real wantedWidth: Math.ceil(input.fitWidth) + 1
                                        + input.leftPadding + input.rightPadding
                                        + combo.leftPadding + seat.width

    /// The last row acts (the way to an answer the list lacks), set off by a menu's separator (デザイン規約 §メニュー).
    property bool lastRowActs: false

    /// The row that wears the push mark; empty marks none (デザイン規約 §リモートを書き留める).
    property string markedRow: ""

    /// The value; `editText` follows it. A model arriving snaps `currentIndex` to the first row and drags the text
    /// along, so the text is held here and put back once the swap settles. Only a person moves it — a picked row, or
    /// `textEdited` (not emitted for text set in code) (rules-refs/app-ui.md「マージツール候補の `AppCombo`」).
    property string wanted: ""
    onWantedChanged: if (!combo.pickOnly && combo.editText !== combo.wanted) combo.editText = combo.wanted
    onActivated: index => {
        if (!combo.pickOnly)
            combo.wanted = combo.textAt(index)
    }
    // Undo the swap's reset once it has finished.
    onModelChanged: Qt.callLater(() => {
        if (!combo.pickOnly)
            combo.editText = combo.wanted
    })

    editable: !combo.pickOnly
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd

    // The ground says whether there is anything to type here (デザイン規約 §選ぶ欄と打つ欄).
    background: Rectangle {
        color: combo.pickOnly ? "transparent" : Theme.bgBase
        radius: Theme.radiusSm
        border.color: combo.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }

    // Own contentItem, so it does not go through palette (§無効).
    contentItem: TextInput {
        id: input
        /// The word's room and the whole value's width. Whether to cut is decided from these two, not from the elide's
        /// answer, which cuts a name that fits by a fraction too; `wantedWidth` reads the same pair.
        readonly property real room: input.width - input.leftPadding - input.rightPadding
        readonly property real fitWidth: whole.advanceWidth
        // Picking only, the value is the owner's (ComboBox keeps `editText` only when editable), cut in the middle: a
        // read-only `TextInput` too narrow scrolls to the caret and loses the name's head unmarked. The elide's own
        // arithmetic, not `CutName`: a lone field has no column edge to line up.
        text: combo.pickOnly ? (input.fitWidth > input.room ? fit.elidedText : combo.wanted) : combo.editText
        color: combo.enabled ? Theme.textPrimary : Theme.textMuted
        font: combo.font
        // The plain boxes' inset (`SlimField`), which this field stands beside in the question bar.
        leftPadding: Theme.spaceXs
        // Stops the word short of the arrow; no list, the plain inset (rules-refs/app-ui.md「打つ欄の右で字を止めるのは山」).
        rightPadding: combo.hasList ? Theme.spaceSm + Theme.spaceXs : Theme.spaceXs
        verticalAlignment: Text.AlignVCenter
        readOnly: combo.pickOnly
        selectByMouse: !combo.pickOnly
        selectionColor: Theme.bgSelected
        selectedTextColor: Theme.textPrimary
        onTextChanged: if (!combo.pickOnly) combo.editText = text
        onTextEdited: combo.wanted = text
        TextMetrics {
            id: fit
            font: input.font
            text: combo.wanted
            elide: Text.ElideMiddle
            elideWidth: Math.max(0, input.width - input.leftPadding - input.rightPadding)
        }
        // The whole name's width, from a ruler with no cut: `fit` answers about what it drew, so a field sized from it
        // stays cut.
        TextMetrics {
            id: whole
            font: input.font
            text: combo.wanted
        }
        // A read-only input still takes the press, so the control would never see the click that opens its list.
        MouseArea {
            anchors.fill: parent
            enabled: combo.pickOnly
            onClicked: combo.pressField()
        }
        // Editable, Qt hands the press to the input and only the arrow would open the list. A handler, so the input
        // still places the caret; a tap, so a drag-select does not drop the list over the selection.
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
        // One ring at a time: here while the card is shut, inside it once it is up.
        readonly property bool waits: combo.loading && !combo.popup.opened
        // Two marks, not one: an animator takes the property it turns, killing the arrow's `rotation` binding for good,
        // so a shared mark comes back pointing sideways. Invisible in headless, where the ring is held still.
        SpinnerIcon {
            anchors.fill: parent
            spinning: seat.waits
        }
        NavIcon {
            anchors.fill: parent
            visible: !seat.waits
            kind: "chevron"
            tint: Theme.textSecondary
            // Drawn pointing right; down is where the list comes from.
            rotation: 90
        }
    }

    /// A press on the field, wherever it came from. Picking only, it toggles; editable, the caret lands and the list
    /// comes down, and a second press only moves the caret (rules-refs/app-ui.md「`AppCombo` は押された所から開く」).
    function pressField() {
        if (combo.pickOnly) {
            if (combo.popup.opened)
                combo.popup.close()
            else
                combo.offer()
            return
        }
        // For the automation verb, which has no pointer; a real press already placed the caret.
        input.forceActiveFocus()
        // After the release: a list opened while the input's exclusive grab is still unwinding is taken straight back
        // down a frame later. `pickOnly`'s MouseArea holds the grab itself, so it needs no wait.
        Qt.callLater(combo.offer)
    }

    /// A letter typed now would land in the field — for runs, since a photograph cannot answer it.
    readonly property bool typing: !combo.pickOnly && input.activeFocus

    /// Enter with nothing standing over the box. Not `accepted`: Qt raises it on the key press, before the release
    /// picks the highlighted row, so it carries the name that row is about to replace (`tst_appcombo`). Under the list,
    /// Enter picks the row (デザイン規約 §立っている質問は 1 か所で聞く).
    signal submitted()
    onAccepted: if (!combo.popup.visible) combo.submitted()

    /// Opens the list, if `hasList`.
    function offer() {
        if (combo.hasList)
            combo.popup.open()
    }

    /// Rows, or a read that may still bring some: the card opens on that with a ring, since refusing leaves the press
    /// unanswered (デザイン規約 §進行中・長押しの定数「進行中を言うのはリングの仕事」). With neither, the arrow goes.
    readonly property bool hasList: combo.count > 0 || combo.loading
    /// Automation only: the card's middle-button hand, and how far it has sent the rows (a middle button cannot be
    /// injected).
    readonly property alias listHand: listHand
    readonly property real listAt: rows.contentY
    // The read came back empty while the card was up: the card cannot say nothing, so it goes.
    onHasListChanged: if (!combo.hasList) combo.popup.close()

    popup: Popup {
        y: combo.height
        width: combo.width
        padding: Theme.spaceXs
        // The default policy reads the field's own press as outside, so a second press could never close the list.
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        // As every popup base: a popup inherits no tip policy from the window (`Main.qml`).
        ToolTip.policy: ToolTip.Manual
        // Also reached by the control's own press. Picking only, the arrows start from the value; on an editable list
        // moving `currentIndex` drags the text (see `wanted`).
        onAboutToShow: {
            if (!combo.hasList) {
                combo.popup.close()
                return
            }
            if (combo.pickOnly)
                combo.currentIndex = combo.find(combo.wanted)
        }
        // Plus the popup's own padding, or the last row is cut.
        implicitHeight: Math.min(contentItem.implicitHeight, Theme.rowHeight * 8) + topPadding + bottomPadding
        background: AppCardFace {}
        // A ring in a row's height while the rows are read, then the rows.
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
                // The see-through thumb: the rows run under it (デザイン規約 §色 スクロールバー).
                ScrollBar.vertical: AutoScrollBar {}
                // The hand an `AppListView` carries, since this list is not one.
                MiddleAutoScroll {
                    id: listHand
                    parent: rows
                    anchors.fill: parent
                    visible: rows.ScrollBar.vertical.visible
                    onDrifted: dy => rows.contentY =
                        Math.max(0, Math.min(rows.contentY + dy, rows.contentHeight - rows.height))
                }
            }
        }
    }

    delegate: ItemDelegate {
        id: row
        required property string modelData
        required property int index
        readonly property bool acts: combo.lastRowActs && row.index === combo.count - 1
        /// The separator above and its air (デザイン規約 §メニュー).
        readonly property int lead: row.acts ? 2 * Theme.spaceXs + Theme.borderWidth : 0
        /// The row holding the value; Qt's highlight is only where the keyboard is (デザイン規約 §選ぶ欄と打つ欄).
        readonly property bool current:
            !row.acts && row.modelData !== "" && row.modelData === (combo.pickOnly ? combo.wanted : combo.editText)
        width: combo.width - 2 * Theme.spaceXs
        height: Theme.rowHeight + row.lead
        topPadding: row.lead
        // A menu row's air either side of its word (`AppMenuItem.padding`, デザイン規約 §選ぶ欄と打つ欄), not the
        // style's wider one.
        leftPadding: Theme.spaceXs
        rightPadding: Theme.spaceXs
        highlighted: combo.highlightedIndex === row.index
        // The line and the row's grounds apart: a wash over the whole item would swallow the line.
        background: Item {
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                y: Theme.spaceXs
                height: Theme.borderWidth
                color: Theme.borderSubtle
                visible: row.acts
            }
            // The value's ground under the hand's wash, an overlay colour, so both read (デザイン規約 §選ぶ欄と打つ欄).
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
                anchors.rightMargin: mark.visible ? mark.width + mark.anchors.rightMargin : 0
                text: row.modelData
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMd
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
            // The left menu's push mark for that remote, at its step and colour.
            NavIcon {
                id: mark
                visible: !row.acts && row.modelData !== "" && row.modelData === combo.markedRow
                kind: "push"
                tint: Theme.accent
                width: Theme.iconSm
                height: Theme.iconSm
                anchors.right: parent.right
                // Clear of the list's floating thumb: right-set ink on that side would stand under it whenever the
                // list scrolls (デザイン規約 §ペインのスクロールバー「その辺に右揃えのインクが立つ」).
                anchors.rightMargin: rows.ScrollBar.vertical.width - row.rightPadding
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }
}
