import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One half of a view toggle: a mark that is lit while its view is the one showing, and a step down in the same hue
// while it is the way to the other (デザイン規約 §暗く落とした段 / §無効 — the muted colour would read as unavailable).
//
// The depth comes from the band the toggle stands in; the mark and the wash stay square inside it. Both are written
// as padding and inset — a `Control` stretches its `contentItem` over whatever the
// padding leaves, so a width written on the mark is gone on the next layout.
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
