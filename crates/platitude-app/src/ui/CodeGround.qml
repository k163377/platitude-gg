import QtQuick
import platitude.ui

// The ground git's spelling stands on — a command's, a name's (デザイン規約 §git 用語のコード表記): faint, rounded, as tall
// as the word's step and a gap — not its line box, which the code face makes different on every OS — and its middle on
// the middle of the code face's ink. Where it starts and ends is the caller's: half a gap off a word's ends, or from the
// head of a name's mark's seat to half a gap past the name's ink (`WordFace.groundStart` / `groundEnd`). The overhang
// eats into the air around the word and never moves the word: its head lines up with the other rows' words, and the
// next word stays an ordinary word space away. Never a whole gap: on a row behind a hold mark the ground would reach
// the mark, and mark and word would read as one run of ink. The words inside a chip close up by a gap
// (`wordSpacing: -spaceXs`, wherever a chip's words are drawn): a mono space is far wider than the air at the ground's
// ends, and `push -f`'s flag would stand off `push` as a second word on one ground.
//
// **The one ground**: `ActionButtonLabel` / `CodeChip` / `AppMenuItem` / `OpExitRow`'s command grounds and
// `CardText` / `AppMenuItem` / `NameRun`'s name grounds — the height, the place, the corners and the colour are all
// decided here from `size` and `wordBaseline`, the numbers nowhere else. Left to the line box, one word stands on a
// faint ground on Windows and a tag on Ubuntu. It draws and nothing more: the holder's `implicitHeight` stays its
// word's, so nothing around it moves.
//
// `wordBaseline` is `required`, so a caller that forgets it fails to load: where names and commands are drawn the
// baseline is passed — a chip a reader can drag over takes its `LineText`'s, a name in a sentence the sentence line's.
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
    /// Set by `ActionButtonLabel` (its ground and the phrase's chips, through `CodeChip.buttonWord`),
    /// `StashActionsBand` and `AskBar`'s answer pills.
    property bool buttonWord: false

    // Its middle on the middle of the code face's ink, not of the word's line box: a family leaves more room over its
    // ascenders than under its descenders, by an amount of its own (Cascadia Mono and Noto Sans Mono CJK JP: descent
    // about the descenders' depth, ascent past the letters), and a ground in the middle of the box has a `y`
    // reach its foot or hang out of it. A button's ground, short at its foot, keeps the top a whole one has. To a whole
    // pixel of the item it stands in.
    y: Math.round(ground.wordBaseline - WordFace.codeInkMiddle(ground.size)
                  - (ground.linePitch > 0 ? ground.height : ground.size + Theme.spaceXs) / 2)
    // A gap over the step, not half of one: a code face's letters stand about an em from the tallest to the deepest
    // (Noto Sans Mono CJK JP: 1.04 em with its `/`), and hinted at a name's size they can be drawn taller still, so
    // half a gap leaves the ground's top or foot under ink. In a text, no taller than leaves half a gap to the next
    // line's ground: Yu Gothic UI sets its lines 1.33 em apart, and a gap over the step would join two lines' grounds
    // (0–1px apart). Headless never loads Yu Gothic UI (it does not pick up a `.ttc`), so a real Windows line pitch is
    // not in a headless picture.
    height: ground.linePitch > 0
            ? Math.min(ground.size + Theme.spaceXs, Math.floor(ground.linePitch - Theme.spaceXs / 2))
            : ground.size + Theme.spaceXs / 2 + (ground.buttonWord ? Theme.spaceXs / 4 : Theme.spaceXs / 2)
    radius: Theme.radiusSm
    color: Theme.bgHover
}
