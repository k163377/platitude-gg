import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The tree ⇄ paths view toggle both file lists carry in their header band. The one not in use is still the way to
// switch, so it keeps the hue and drops a step rather than falling to the muted colour, which would read as unavailable
// (デザイン規約 §暗く落とした段 / §無効).
//
// **The pair is the whole of its seat.** No word stands beside it, so each mark is its cell's own content and takes the
// step the cell's height leaves rather than the one a mark standing beside a caption would (デザイン規約 §寸法「印が帯ではなく
// セルの中身そのものである時」— the folded rail's sections are the other place this holds). The seat runs the band top to bottom
// as well, so a hand coming down the header lands on the switch anywhere in its depth — **and only the hit area reaches
// that far**: the wash keeps the mark's own box, since paint carried to a band's edge reads as a different kind of
// control from the caption it shares the band with (§当たり判定「広げるのは判定だけ」).
RowLayout {
    id: toggle

    /// Which half of the pair the list is showing.
    property bool treeView: false
    signal chosen(bool tree)

    Layout.fillHeight: true
    // Nothing between the two: `hier` and `list` are drawn inside a 16-grid that leaves air either side of the ink, and
    // that air is what the eye measures the gap by (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    spacing: 0

    // The depth comes from the band the toggle stands in; the mark and the wash stay square inside it. Both are written
    // as padding and inset rather than as a size on the icon — a `Control` stretches its `contentItem` over whatever the
    // padding leaves, so a width written on the mark is gone on the next layout.
    component ViewButton: HoverToolButton {
        id: viewButton
        required property string mark
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

    ViewButton {
        Layout.fillHeight: true
        mark: "hier"
        lit: toggle.treeView
        tip: qsTr("Tree view")
        onClicked: toggle.chosen(true)
    }
    ViewButton {
        Layout.fillHeight: true
        mark: "list"
        lit: !toggle.treeView
        tip: qsTr("Paths view")
        onClicked: toggle.chosen(false)
    }
}
