import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// ToolButton wearing the flat hover wash as its `background`, in place of Fusion's ButtonPanel
// (rules-refs/app-ui.md「hover の重ね色は `HoverToolButton` が `background` として自分で描く」). The background keeps
// the panel's implicit 20, so no button changes size by being flattened.
ToolButton {
    id: hoverToolButtonSelf
    /// Zero on controls that fill a band or cell edge to edge, beside square washes.
    property real washRadius: Theme.radiusSm
    /// The wash, for a caller whose own `background` replaces this one whole and must paint it itself
    /// (rules-refs/app-ui.md「`background` は渡すと丸ごと差し替わる」). Pressed has its own step: with no panel under
    /// it, dropping the wash on press would answer a held finger with nothing.
    readonly property color washColor:
        !hoverToolButtonSelf.enabled ? "transparent"
        : hoverToolButtonSelf.down ? Theme.bgPressed
        : hoverToolButtonSelf.lit || hoverToolButtonSelf.visualFocus || hoverToolButtonSelf.standing ? Theme.bgHover
        : "transparent"
    /// What this button opened is standing: the wash stays, so the card still hangs off something lit (as
    /// `AppMenuButton`'s does).
    property bool standing: false

    /// Headless stand-in for the pointer; read only through `lit`, so a run lights what a hand lights.
    property bool pointedAt: false
    /// A hand is on this button: a real hover (not the offscreen cursor parked in the corner — `Hand.away`) or the
    /// stand-in.
    readonly property bool lit:
        (hoverToolButtonSelf.hovered && !Hand.away) || hoverToolButtonSelf.pointedAt
    /// Automation: the tip is actually up (after `tipDelayMs`), not merely asked for.
    readonly property bool tipShown: hoverToolButtonSelf.ToolTip.visible

    // Said out loud: the default hint is false offscreen
    // (rules-refs/app-ui.md「`hoverEnabled: true` は `HoverToolButton` が名乗る」).
    hoverEnabled: true

    /// The one thing the face had no room to say (デザイン規約 §hover のツールチップ); empty shows none. A button whose
    /// tip shows under another condition leaves this empty and binds `ToolTip.visible` itself.
    property string tip: ""
    /// A word inside `tip` that is a place to go, and the name the page answers for it (`RepoPage.tipLinkAsked`).
    /// `tip` stays plain text (git-printed names can hold `<`); `SharedToolTip` draws this word as the anchor.
    property string tipPlace: ""
    property string tipHref: ""

    ToolTip.visible: hoverToolButtonSelf.tip !== "" && hoverToolButtonSelf.lit
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: hoverToolButtonSelf.tip

    background: Rectangle {
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        radius: hoverToolButtonSelf.washRadius
        color: hoverToolButtonSelf.washColor
    }
}
