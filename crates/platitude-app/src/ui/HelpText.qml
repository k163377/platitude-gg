import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The sentence that says what a chapter is for, where its value is written, or what git is doing right now.
//
// **`fontMd`**. A settings screen is mostly these, so this text is most of the reading; a step down is "a note
// beside the thing being read". What tells it from the label above it is the ink, which is how VS
// Code's settings editor tells a setting's description from its name as well.
//
// **`textSecondary`** — `textMuted` is the disabled signal and the placeholder's
// (§テキスト / §無効), and a screen whose explanations wear it draws live text in the colour that means
// "this cannot be pressed" (observed).
//
// **It takes the width it is given.** Setting it narrower than the boxes above it — the reading IBM Carbon takes for
// a dialog, where body copy keeps a right margin and only form inputs expand the whole width — is not open here: a
// right margin read off the parent's width is a loop, since that width is what the parent computes from its children
// ("Detected recursive rearrange", measured). Capping the whole column is the way that holds, and the column
// is capped at `Theme.textWidth` where the chapters stand (`SettingsDialog`).
Label {
    id: help

    Layout.fillWidth: true
    wrapMode: Text.Wrap
    color: Theme.textSecondary
    font.pixelSize: Theme.fontMd
}
