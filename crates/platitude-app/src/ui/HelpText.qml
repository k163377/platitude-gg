import QtQuick
import QtQuick.Layouts
import platitude.ui

// The sentence that says what a chapter is for, where its value is written, or what git is doing right now. A field
// (`CardText` — rules-refs/app-ui.md「設定の画面の字は全部欄」), in `fontMd` / `textSecondary`
// (デザイン規約 §タイポグラフィ). It takes the width it is given: a right margin off the parent's width is a layout
// loop, so the column is capped instead (`Theme.textWidth` in `SettingsDialog`; rules-refs/app-ui.md「狭めるのは列ごと」).
CardText {
    id: help

    Layout.fillWidth: true
    color: Theme.textSecondary
    pixelSize: Theme.fontMd
}
