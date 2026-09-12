import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// ToolButton wearing the theme's flat hover wash in place of the style's own panel (デザイン規約 §色: ホバーは bgHover の重ね色).
//
// Fusion's ButtonPanel is not a tint — it is a gradient face with a 2px radius, an outline and a second contrast line
// inside it, so a hovered tool button read as a raised box among rows, headers and rail cells that answer the same
// pointer with a flat wash and nothing else. A checked one kept that box up whether or not anyone was pointing at it,
// which is why the fold control in the sidebar's header band, and the eye that keeps tags out of the graph, both looked
// like a different kind of control from everything beside them.
//
// The wash is the background rather than a child so it stays behind the icon a caller hands `contentItem`, and it keeps
// the panel's implicit 20 so no button changes size by being flattened.
ToolButton {
    id: hoverToolButtonSelf
    /// The corner of the wash. Zero on the controls that fill a band or a cell edge to edge, where what washes beside
    /// them is square.
    property real washRadius: Theme.radiusSm
    /// What the pointer is answered with, for a caller that draws its own background and would otherwise answer with
    /// nothing.
    ///
    /// A `background` handed in replaces this one whole. While the wash was a child of the content it survived that —
    /// every `ActionButton` in the app went on answering the pointer with its own frame drawn underneath — and moving
    /// the paint here took it from all of them in one go (observed: fetch, push, the commit button, the hunk
    /// heading's two and every dialog's pair stopped lighting at all). So the rule lives in one place and the two
    /// backgrounds read it.
    ///
    /// Pressed is a step up from hover rather than the style's darker face: with no panel under it, a wash that lifted
    /// on press would leave the button answering a held finger with nothing.
    readonly property color washColor:
        !hoverToolButtonSelf.enabled ? "transparent"
        : hoverToolButtonSelf.down ? Theme.bgPressed
        : hoverToolButtonSelf.lit || hoverToolButtonSelf.visualFocus ? Theme.bgHover
        : "transparent"

    /// Stands in for the pointer, which headless cannot inject (`NavItemDelegate.tipPointedAt` and its kin). Read
    /// where `hovered` is read and nowhere else, so a run lights what a hand lights — wash and tip together, never one
    /// without the other.
    property bool pointedAt: false
    /// A hand is on this button, whichever of the two put it there.
    readonly property bool lit: hoverToolButtonSelf.hovered || hoverToolButtonSelf.pointedAt
    /// Automation: the tip this button raised is up. The attached tooltip waits out `tipDelayMs` before it stands, so
    /// this is the one thing that says the words are on screen rather than merely asked for.
    readonly property bool tipShown: hoverToolButtonSelf.ToolTip.visible

    // Said out loud rather than left to the platform. A Control with no ancestor claiming hover falls back to the
    // theme's `useHoverEffects` hint, which the offscreen platform answers with false (measured with qmltestrunner: the
    // same scene washes for a `HoverHandler` and not for a ToolButton). The rows and cells beside these buttons use
    // handlers, which never ask — so leaving the hint to decide is what makes one control in a band answer the pointer
    // and its neighbour not.
    hoverEnabled: true

    /// The one thing the button's face had no room to say (規約 §hover のツールチップ). Empty says nothing at all.
    ///
    /// A button whose tip comes out under some other condition — only while the thing is refused, or while a hand is
    /// resting on another item entirely — binds `ToolTip.visible` itself and leaves this empty; a use-site binding
    /// replaces the one below.
    property string tip: ""
    /// A word inside `tip` that is a place to go, and the name the page answers for it (`RepoPage.tipLinkAsked`).
    /// Both empty — the ordinary case — leaves the tip the plain sentence it is.
    ///
    /// **The sentence itself is never spelled twice.** `tip` stays plain: it is what is measured, what a run reads
    /// back (`TopBar.fetchTipShown`), and what can hold a ref name or a path git printed without a `<` in it eating
    /// the rest of the line. The tooltip finds this word inside it and draws that run as an anchor, which is also
    /// where the anchor's colour has to be decided (`SharedToolTip`).
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
