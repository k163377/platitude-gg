// What the reader's hand is doing where the window's own hover and focus cannot say it.
pragma Singleton

import QtQuick

QtObject {
    /// No hand over this window, so a pointer lights nothing: a headless run's cursor rests on the window's top left
    /// and would light it for the whole run (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」). Set only by the
    /// harness; false in every window a person opens. Read wherever a real pointer would (`HoverToolButton.lit`).
    property bool away: false

    /// The scroll bar the hand is holding, or null. While one is held Qt hears no hover anywhere, so the rows keep what
    /// they had as they slide from under the hand: what hover opened goes, and nothing opens until the bar is let go
    /// (`TipSeat`, `HoverSeat`; rules-refs/app-ui.md「押下が掴んでいる間、窓に hover は届かない」). Written by
    /// the bars (`AutoScrollBar`).
    property Item heldBar: null

    /// A press the window holds — a button down outside every popup, `pressAt` where it is now — and whether it has
    /// travelled `Qt.styleHints.startDragDistance` from where it went down: a drag. Qt tells no hover while a press
    /// holds its grab, and at the release hands it to whatever stands under the hand, as if the hand had walked there
    /// (rules-refs/app-ui.md「押下が掴んでいる間、窓に hover は届かない」). Written by the window's press watcher
    /// (`FocusRelease.tellsHand`) through the functions below.
    property bool pressing: false
    property bool dragging: false
    property point pressAt: Qt.point(-1, -1)
    /// The press was let go at `letGoAt` and the hand has not moved from there since: the hover Qt hands at the release
    /// is told at that same place, and is no move. Lifted by the window's hand (`heard`).
    property bool letGo: false
    property point letGoAt: Qt.point(-1, -1)
    /// Hover is held back, from the press until the hand's own next move after the release (`HoverSeat.pressHolds`;
    /// デザイン規約 §hover のツールチップ「押している間は何も開かない」).
    readonly property bool holdsHover: pressing || letGo

    function pressBegan(at) {
        pressing = true
        dragging = false
        letGo = false
        pressAt = Qt.point(at.x, at.y)
    }
    /// The press was carried to `at`; `travelled` says it is past the drag distance from where it went down.
    function pressMoved(at, travelled) {
        pressAt = Qt.point(at.x, at.y)
        if (travelled)
            dragging = true
    }
    /// The hold passes from the press to the still hand before the press lets it go: `holdsHover` false between the
    /// two would answer the asks kept meanwhile, and one that opens at once would open.
    function pressEnded() {
        letGoAt = pressAt
        letGo = true
        pressing = false
        dragging = false
    }
    /// Where the window's hand is heard — (-1, -1) for gone from the window: anywhere but where the press let go is the
    /// hand's own move. A press still holding is no release yet.
    function heard(at) {
        if (!letGo || pressing)
            return
        if (at.x < 0 || Math.max(Math.abs(at.x - letGoAt.x), Math.abs(at.y - letGoAt.y)) >= 1)
            letGo = false
    }

    /// Something happened that a gesture of two clicks in a row must not span (`ReclickGesture`) — three ways in: a
    /// press anywhere in this window, on the window itself (`Main.pressLandedAnywhere`) or on a card, menu, dialog or
    /// tip over it (`AppCardFace`, `TipSeat`'s ground); the history landing on a commit
    /// (`RepoPage.activateRow` — the arrows, the search, a menu, a row's click — and `jumpToRef`); the window left
    /// (`Main`). The click a gesture is answering is among them, so a listener asks once the input is through whether
    /// its own target took it. The window hears a press before what it lands on does, a card after and some twice:
    /// ask whether, never how often (`tests/qml/tst_reclickaway.qml`).
    signal acted()
}
