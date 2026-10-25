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
    /// (`TipSeat`, `HoverSeat`; rules-refs/app-ui.md「バーを掴んでいる間、窓に hover は届かない」). Written by
    /// the bars (`AutoScrollBar`).
    property Item heldBar: null

    /// Something happened that a gesture of two clicks in a row must not span (`ReclickGesture`) — three ways in: a
    /// press anywhere in this window, on the window itself (`Main.pressLandedAnywhere`) or on a card, menu, dialog or
    /// tip over it (`AppCardFace`, `TipSeat`'s ground); the history landing on a commit
    /// (`RepoPage.activateRow` — the arrows, the search, a menu, a row's click — and `jumpToRef`); the window left
    /// (`Main`). The click a gesture is answering is among them, so a listener asks once the input is through whether
    /// its own target took it. The window hears a press before what it lands on does, a card after and some twice:
    /// ask whether, never how often (`tests/qml/tst_reclickaway.qml`).
    signal acted()
}
