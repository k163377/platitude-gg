import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The sentence that says what a chapter is for, where its value is written, or what git is doing right now.
//
// **`fontMd`, not a step down** (2026-08-30 ユーザー指示). A settings screen is mostly these, so a step down is not
// "a note beside the thing being read" here — it is most of the reading. What tells it from the label above it is
// the ink, which is how VS Code's settings editor tells a setting's description from its name as well.
//
// **`textSecondary`, never `textMuted`** — that ink is the disabled signal and the placeholder's
// (§テキスト / §無効), and a screen whose explanations wore it was drawing live text in the colour that means
// "this cannot be pressed" (2026-08-30 ユーザー報告).
//
// **It takes the width it is given.** Setting it narrower than the boxes above it — the reading IBM Carbon takes for
// a dialog, where body copy keeps a right margin and only form inputs expand the whole width — was tried and taken
// back out: a right margin read off the parent's width is a loop, since that width is what the parent computes from
// its children ("Detected recursive rearrange", 実測). Capping the whole column instead is the way that holds, and it
// needs a number nobody has agreed to yet (P3-確認事項).
Label {
    id: help

    Layout.fillWidth: true
    wrapMode: Text.Wrap
    color: Theme.textSecondary
    font.pixelSize: Theme.fontMd
}
