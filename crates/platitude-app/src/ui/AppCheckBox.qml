import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The one check box (デザイン規約 §QML 実装ルール). Drawn from tokens: Fusion's indicator is `palette.base` — the
// pane behind it — framed darker still, so on this theme the box vanishes (unchecked reads as a blank before the
// words, checked as a mark floating in the air). Unchecked wears an input field's own frame (`bgBase` face,
// `borderDefault` 1px, `borderFocus` for focus) so it reads as something to press; checked is an `accent` face with
// the `check` mark in `textOnAccent`; under the pointer the frame goes `borderStrong`. Disabled moves §無効's palette
// table onto the face (`highlight` → `accentMuted` face, `highlightedText` → `textMuted` mark).
// Headless: `amend-author` / `remote-url origin:marked` / `commands`.
CheckBox {
    id: appCheck

    /// The least a holder that lets the words give way hands the box: its seat, the cut mark, so words cut to nothing
    /// still say they were cut (`CutName.markWidth`), and the style's right padding past it, which the words' box
    /// stands inside as it does past whole words.
    readonly property real leastWidth:
        appCheck.indicator.width + appCheck.spacing + Math.ceil(boxWords.markWidth) + appCheck.rightPadding

    font.pixelSize: Theme.fontMd
    // Users keep this default; only the band's `Background reads` (`CommandsPane`) drops to `iconLg`.
    implicitHeight: Theme.controlHeight
    // The box's seat: whatever holds the box keeps its step on the left, and the same step comes before the words —
    // the style's own 6 on the left would stack on it (デザイン規約 §アイコンの席「④ の席」— the box's seat is
    // `spaceXs` | mark | `spaceXs`).
    leftPadding: 0
    spacing: Theme.spaceXs
    indicator: Rectangle {
        implicitWidth: Theme.iconMd
        implicitHeight: Theme.iconMd
        x: appCheck.leftPadding
        y: appCheck.topPadding + (appCheck.availableHeight - height) / 2
        radius: Theme.radiusSm
        color: !appCheck.checked ? Theme.bgBase
             : appCheck.enabled ? Theme.accent : Theme.accentMuted
        border.width: Theme.borderWidth
        border.color: appCheck.visualFocus ? Theme.borderFocus
                    : appCheck.checked ? color
                    : appCheck.hovered ? Theme.borderStrong : Theme.borderDefault
        NavIcon {
            anchors.fill: parent
            kind: "check"
            visible: appCheck.checked
            tint: appCheck.enabled ? Theme.textOnAccent : Theme.textMuted
        }
    }
    // The words after the box's seat, as the style's label lays them, but cut at their end with the cut mark when a
    // holder hands the box less than they take (`CutName`, 規約 §寸法「省略の印は描く」): the style's label would type
    // the font's `…`. A line in no column (`closed`).
    contentItem: Item {
        implicitWidth: appCheck.indicator.width + appCheck.spacing + boxWords.implicitWidth
        implicitHeight: boxWords.implicitHeight
        CutName {
            id: boxWords
            x: appCheck.indicator.width + appCheck.spacing
            width: Math.max(0, parent.width - x)
            anchors.verticalCenter: parent.verticalCenter
            text: appCheck.text
            cutAt: "end"
            closed: true
            pixelSize: appCheck.font.pixelSize
            weight: appCheck.font.weight
            color: appCheck.palette.windowText
        }
    }
}
