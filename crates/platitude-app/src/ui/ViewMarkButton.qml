import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One half of a view toggle: the mark is lit while its view shows, and a step down in the same hue otherwise — not
// the muted colour, which reads as unavailable (デザイン規約 §暗く落とした段 / §無効).
//
// Square inside the band's depth by padding and inset: a `Control` stretches its `contentItem` over what the
// padding leaves, so a width written on the mark is lost on the next layout.
HoverToolButton {
    id: viewButton

    /// Which of `NavIcon`'s kinds this half is drawn as.
    required property string mark
    /// Whether this half's view is the one showing.
    required property bool lit

    implicitWidth: Theme.iconXl
    implicitHeight: Theme.iconXl
    padding: 0
    topPadding: Math.round((viewButton.height - Theme.iconXl) / 2)
    bottomPadding: viewButton.topPadding
    topInset: viewButton.topPadding
    bottomInset: viewButton.topPadding
    contentItem: NavIcon {
        kind: viewButton.mark
        tint: viewButton.lit ? Theme.accent : Theme.accentDim
    }
}
