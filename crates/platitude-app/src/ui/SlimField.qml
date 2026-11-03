import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Slim single-line input shared by the toolbar search and the in-place name boxes: thin frame, compact height.
TextField {
    id: slim
    /// What is typed cannot be accepted: the frame says so, the reason waits in the tooltip
    /// (規約 §可否・警告の出し場所).
    property bool refused: false
    /// The frame's colour while the box has the keyboard: the focus ring, or the ref kind's own colour for a box naming
    /// a ref (規約 §ref の種別: 枠 = 種別). A refusal outranks it: the kind says what is typed, the refusal whether it
    /// can be, and the reader has to answer that first. Only two kinds override (`NavNameBox`, `GraphRowChips`): a
    /// tag's box (new or rename) is `refTag`, a new worktree's branch box that worktree's `success`. The rest keep the
    /// ring — a local branch's `accent` is the ring's own value, a remote's `textSecondary` is too dark for a focus
    /// frame, and a stash is no ref.
    property color focusTone: Theme.borderFocus
    /// The width at which what is typed stands without scrolling, insets included — for a layout that gives this box
    /// the room it needs (`PublishForm`).
    readonly property real wantedWidth: slim.contentWidth + slim.leftPadding + slim.rightPadding
    /// The kind of name the box takes, as a mark at its left edge that is no part of what is typed
    /// (`AppCombo.leadMark`, デザイン規約 §ref の種別「名前の印」). Empty for none.
    property string leadMark: ""
    /// The mark's seat's air on the frame's side, given back: the inset already parts the mark from the frame
    /// (`MarkSeat.airAt`, デザイン規約 §余白「枠の中の先頭に立つ印」).
    readonly property real frameAir: slim.leadMark !== "" ? MarkSeat.airAt(slim.font.pixelSize) : 0
    /// The room the words have between the insets.
    readonly property real room: slim.width - slim.leftPadding - slim.rightPadding
    /// A name too long for the box, with nobody typing in it, is shown cut — in its middle, a remote's at its end —
    /// and the whole under the hand; typing, it scrolls whole (user decision — ask before changing; デザイン規約
    /// §選ぶ欄と打つ欄 — the web's `text-overflow: ellipsis` on a field not being edited, `AppCombo.idleCut` alike).
    /// Only a box that holds a name (`leadMark`). Its right-click menu takes the focus while it stands, and the name is
    /// still being typed under it (`FieldMenuSeat.holding`).
    readonly property bool idleCut: slim.leadMark !== "" && !slim.activeFocus && !menuSeat.holding
                                    && slim.contentWidth > slim.room
    /// The name cut to fill the room (`WordFace.cutToFit`), worked out again whenever what it reads changes while it
    /// is drawn, as `AppCombo.cutValue` is.
    property string cutText: ""
    /// Automation: the cut name as drawn, to read where its line stands against the field's.
    readonly property alias idleLabel: idleWord
    /// The whole name under a hand on a cut one (デザイン規約 §寸法「名前の天井」— 全文は hover).
    readonly property TipAsk cutTip: TipAsk {
        target: slim
        under: true
        asked: slim.idleCut && fieldHover.hovered && !Hand.away
        words: slim.text
    }

    /// Cuts the name again (`cutText`) — only while it is drawn cut. The ruler takes the field's face here, as
    /// `AppCombo.recut`'s does.
    function recut() {
        if (!slim.idleCut)
            return
        cutRuler.font = slim.font
        slim.cutText = WordFace.cutToFit(word => {
            cutRuler.text = word
            return cutRuler.advanceWidth
        }, slim.text, slim.room, slim.leadMark === "remote")
    }

    implicitHeight: Theme.iconLg
    font.pixelSize: Theme.fontMd
    // The style's own colour, clear while the cut stands in for it (`idleCut`).
    color: slim.idleCut ? "transparent" : slim.palette.text
    // Every box in the app holds its words `spaceXs` off its frame on both sides — the pane's own inset, which the
    // message frames also stand their text on (デザイン規約 §余白) — and the kind's mark after it (`leadMark`).
    leftPadding: Theme.spaceXs + (slim.leadMark !== "" ? leadIcon.wordCell - slim.frameAir : 0)
    rightPadding: Theme.spaceXs
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: slim.refused ? Theme.warning
                    : slim.activeFocus || menuSeat.holding ? slim.focusTone : Theme.borderDefault
        border.width: Theme.borderWidth
    }
    onIdleCutChanged: slim.recut()
    onTextChanged: slim.recut()
    onRoomChanged: slim.recut()
    onFontChanged: slim.recut()
    onLeadMarkChanged: slim.recut()
    Component.onCompleted: slim.recut()

    // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    Keys.onMenuPressed: event => event.accepted = menuSeat.offer()
    // What is typed past the box scrolls to the caret; unclipped, its head is drawn out past the frame.
    clip: true
    FieldMenuSeat {
        id: menuSeat
        editor: slim
    }
    TextMetrics {
        id: cutRuler
    }
    HoverHandler {
        id: fieldHover
    }
    // The inset's own ground inside the frame, clear of its rounded corners, so the scrolled words' head passes under
    // the mark, not through it.
    Rectangle {
        visible: slim.leadMark !== ""
        x: Theme.borderWidth
        y: Theme.radiusSm
        width: slim.leftPadding - Theme.borderWidth
        height: slim.height - 2 * Theme.radiusSm
        color: Theme.bgBase
    }
    // The name, cut, while nobody is typing in it (`idleCut`), on the field's own line; the field's own text goes
    // clear meanwhile, so no scrolled piece of it is drawn beside.
    PlainTextLabel {
        id: idleWord
        visible: slim.idleCut
        x: slim.leftPadding
        width: slim.room
        height: slim.height
        text: slim.cutText
        color: slim.palette.text
        font: slim.font
        verticalAlignment: Text.AlignVCenter
    }
    // In the inset, on the words' line: drawn, not typed.
    NavIcon {
        id: leadIcon
        visible: slim.leadMark !== ""
        x: Theme.spaceXs - slim.frameAir
        y: slim.topPadding
        width: leadIcon.wordCell
        height: slim.height - slim.topPadding - slim.bottomPadding
        kind: slim.leadMark !== "" ? slim.leadMark : "branch"
        set: "sign"
        size: slim.font.pixelSize
        wordBase: slim.baselineOffset - slim.topPadding
        nameAir: WordFace.nameAir(slim, slim.text)
        // Its kind's colour, as a name's mark is anywhere (`NavIcon.inkBeside`); grey with the box.
        tint: leadIcon.inkBeside(slim.enabled ? Theme.textPrimary : Theme.textMuted)
    }
}
