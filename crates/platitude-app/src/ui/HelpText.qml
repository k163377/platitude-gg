import QtQuick
import QtQuick.Layouts
import platitude.ui

// The sentence that says what a chapter is for, where its value is written, or what git is doing right now. A field
// (`CardText` — every word of the settings screen is one, `SettingsDialog`), in `fontMd` / `textSecondary`
// (デザイン規約 §タイポグラフィ). The one spelling of the settings screen's explanations. `fontMd`, a step above the help
// text elsewhere (`fontSm`), is this screen's own (user decision — ask before changing). It takes the width it is
// given: a right margin off the parent's width (`Layout.rightMargin: parent.width * n`) is a layout loop — the parent's
// width comes from its children ("Detected recursive rearrange") — so the column is capped instead (`Theme.textWidth`
// in `SettingsDialog`).
CardText {
    id: help

    Layout.fillWidth: true
    color: Theme.textSecondary
    pixelSize: Theme.fontMd
}
