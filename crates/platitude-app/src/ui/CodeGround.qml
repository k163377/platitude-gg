import QtQuick
import platitude.ui

// The ground git's spelling stands on — a command's, a name's (デザイン規約 §git 用語のコード表記): faint, rounded, as tall
// as the word's step and a gap — not its line box, which the code face makes different on every OS — and its middle on
// the middle of the code face's ink. Where it starts and ends is the caller's: half a gap off a word's ends, or from the
// head of a name's mark's seat to half a gap past the name's ink (`WordFace.groundStart` / `groundEnd`).
Rectangle {
    id: ground

    /// The pixel size of the word it is under.
    property real size: Theme.fontMd
    /// That word's baseline, in the coordinates of the item this stands in.
    required property real wordBaseline
    /// How far apart the lines of the text the word is set in stand, where it can take several (`CardText`); 0 for a
    /// word on a line of its own.
    property real linePitch: 0
    /// The word is a button's: the ground keeps a quarter of a gap under it, not half of one. Most of a word's letters
    /// stand on the baseline, so a foot as deep as the top leaves the ground empty under the word (`fetch`), and the air
    /// under a descender looks wider than the same air over an ascender (`push`). Not read in a text (`linePitch`).
    property bool buttonWord: false

    // Its middle on the middle of the code face's ink, not of the word's line box: a family leaves more room over its
    // ascenders than under its descenders, by an amount of its own, and a ground in the middle of the box has a `y`
    // reach its foot or hang out of it. A button's ground, short at its foot, keeps the top a whole one has. To a whole
    // pixel of the item it stands in.
    y: Math.round(ground.wordBaseline - WordFace.codeInkMiddle(ground.size)
                  - (ground.linePitch > 0 ? ground.height : ground.size + Theme.spaceXs) / 2)
    // A gap over the step, not half of one: a code face's letters stand about an em from the tallest to the deepest
    // (Noto Sans Mono CJK JP: 1.04 em with its `/`), and hinted at a name's size they can be drawn taller still, so
    // half a gap leaves the ground's top or foot under ink. In a text, no taller than leaves half a gap to the next
    // line's ground: Yu Gothic UI sets its lines 1.33 em apart, and a gap over the step would join two lines' grounds.
    height: ground.linePitch > 0
            ? Math.min(ground.size + Theme.spaceXs, Math.floor(ground.linePitch - Theme.spaceXs / 2))
            : ground.size + Theme.spaceXs / 2 + (ground.buttonWord ? Theme.spaceXs / 4 : Theme.spaceXs / 2)
    radius: Theme.radiusSm
    color: Theme.bgHover
}
