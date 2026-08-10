import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// ToolButton wearing the theme's flat hover wash in place of the style's
// own panel (デザイン規約 §色: ホバーは bgHover の重ね色).
//
// Fusion's ButtonPanel is not a tint — it is a gradient face with a 2px
// radius, an outline and a second contrast line inside it, so a hovered
// tool button read as a raised box among rows, headers and rail cells
// that answer the same pointer with a flat wash and nothing else. A
// checked one kept that box up whether or not anyone was pointing at it,
// which is why the fold control in the sidebar's header band, and the
// eye that keeps tags out of the graph, both looked like a different
// kind of control from everything beside them.
//
// The wash is the background rather than a child so it stays behind the
// icon a caller hands `contentItem`, and it keeps the panel's implicit
// 20 so no button changes size by being flattened.
ToolButton {
    id: hoverToolButtonSelf
    /// The corner of the wash. Zero on the controls that fill a band or
    /// a cell edge to edge, where what washes beside them is square.
    property real washRadius: Theme.radiusSm

    // Said out loud rather than left to the platform. A Control with no
    // ancestor claiming hover falls back to the theme's `useHoverEffects`
    // hint, which the offscreen platform answers with false (measured
    // with qmltestrunner: the same scene washes for a `HoverHandler` and
    // not for a ToolButton). The rows and cells beside these buttons use
    // handlers, which never ask — so leaving the hint to decide is what
    // makes one control in a band answer the pointer and its neighbour
    // not.
    hoverEnabled: true

    background: Rectangle {
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        radius: hoverToolButtonSelf.washRadius
        // Pressed is a step up from hover rather than the style's darker
        // face: with no panel under it, a wash that lifted on press would
        // leave the button answering a held finger with nothing.
        color: !hoverToolButtonSelf.enabled ? "transparent"
             : hoverToolButtonSelf.down ? Theme.bgPressed
             : hoverToolButtonSelf.hovered
               || hoverToolButtonSelf.visualFocus ? Theme.bgHover
             : "transparent"
    }
}
