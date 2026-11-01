import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// ToolButton wearing the flat hover wash as its `background`, in place of Fusion's ButtonPanel (デザイン規約
// §QML 実装ルール): Fusion's `ToolButton` puts the panel up on hover / down / checked, and `bgHover` laid over it is no
// flat wash; laid as a child, the wash would sit over the `contentItem`'s mark. The background keeps the panel's
// implicit 20, so no button changes size by being flattened. `checked` (the eye's two states) is told by the tint
// alone (規約 §暗く落とした段).
//
// Disabled, it still takes hover (it says `hoverEnabled: true`) and `TipAsk.asked` reads only `tip` and `lit`: a
// button that cannot be pressed is silenced by an empty `tip` (規約 §無効「ボタンや入力欄の無効はツールチップを持たない」)
// — `TopBar`'s fetch is that shape (headless: the `fetch-tip` pair).
ToolButton {
    id: hoverToolButtonSelf
    /// Zero on controls that fill a band or cell edge to edge, beside square washes.
    property real washRadius: Theme.radiusSm
    /// The wash, for a caller whose own `background` replaces this one whole and must paint it itself — one that does
    /// not is a button that answers the pointer with nothing. Pressed has its own step: with no panel under it,
    /// dropping the wash on press would answer a held finger with nothing.
    readonly property color washColor:
        !hoverToolButtonSelf.enabled ? "transparent"
        : hoverToolButtonSelf.down ? Theme.bgPressed
        : hoverToolButtonSelf.lit || hoverToolButtonSelf.visualFocus || hoverToolButtonSelf.standing ? Theme.bgHover
        : "transparent"
    /// What this button opened is standing: the wash stays, so the card still hangs off something lit (as
    /// `AppMenuButton`'s does).
    property bool standing: false

    /// Headless stand-in for the pointer; read only through `lit` (by `washColor` and the tip's `asked`, where
    /// `hovered` would be read), so a run lights what a hand lights. The one way a tooltip's words can be photographed:
    /// `fetch-hover <failures>`, `push-hover`; `publish-tip` the same through `AskBar.pointedAt`.
    property bool pointedAt: false
    /// A hand is on this button: a real hover (not the offscreen cursor parked in the corner — `Hand.away`) or the
    /// stand-in.
    readonly property bool lit:
        (hoverToolButtonSelf.hovered && !Hand.away) || hoverToolButtonSelf.pointedAt
    /// Automation: the tip is actually up (after `tipDelayMs`), not merely asked for — the window's box standing on
    /// this button (`TipAsk.shown`), not a tip string being there (`TopBar.fetchTipShown`).
    readonly property bool tipShown: buttonTip.shown

    // Said out loud: a `Control`'s default (the theme's `useHoverEffects` hint) is false offscreen, while the rows' and
    // cells' `HoverHandler`s ask no hint — unsaid, only one of the two in a band answers the hand.
    hoverEnabled: true

    /// The one thing the face had no room to say (デザイン規約 §hover のツールチップ); empty shows none.
    property string tip: ""
    /// A word inside `tip` that is a place to go, and the name the page answers for it (`RepoPage.tipLinkAsked`).
    /// `tip` stays plain text (git-printed names can hold `<`); the window's tip draws this word as the anchor.
    property string tipPlace: ""
    property string tipHref: ""

    readonly property TipAsk buttonTip: TipAsk {
        id: buttonTip
        target: hoverToolButtonSelf
        asked: hoverToolButtonSelf.tip !== "" && hoverToolButtonSelf.lit
        words: hoverToolButtonSelf.tip
        place: hoverToolButtonSelf.tipPlace
        href: hoverToolButtonSelf.tipHref
    }

    background: Rectangle {
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        radius: hoverToolButtonSelf.washRadius
        color: hoverToolButtonSelf.washColor
    }
}
