// What the hand is doing where the window's own hover cannot say it.
pragma Singleton

import QtQuick

QtObject {
    /// No hand over this window, so a pointer lights nothing: a headless run's cursor rests on the window's top left
    /// and would light it for the whole run (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」). Set only by the
    /// harness; false in every window a person opens. Read wherever a real pointer would (`HoverToolButton.lit`).
    property bool away: false

    /// The scroll bar the hand is holding, or null. While one is held Qt hears no hover anywhere, so the rows keep what
    /// they had as they slide from under the hand: what hover opened goes, and nothing opens until the bar is let go
    /// (`SharedToolTip`, `HoverCardHost`; rules-refs/app-ui.md「バーを掴んでいる間、窓に hover は届かない」). Written by
    /// the bars (`AutoScrollBar`).
    property Item heldBar: null
}
