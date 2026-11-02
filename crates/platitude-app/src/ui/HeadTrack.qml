import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

/// A branch's ahead / behind against its upstream, arrows drawn rather than set as `↑` / `↓` (East Asian Ambiguous,
/// full-width in the CJK families — デザイン規約 §寸法「印は描いて出す」).
///
/// On a branch row of the left panel it is said at the far end, **only when there is something to say, and only what
/// there is** (user decision — ask before changing): `↑2`, `↓1`, `↑1 ↓1`. Only a nonzero leg is drawn, as git spells
/// `[ahead 2]` and never `[ahead 2, behind 0]`, and as other products drop the 0 beside an inline name; a 0 belongs
/// only in a table where the count has a column of its own. Whoever seats this leaves it empty when both are zero, and
/// with no upstream or a gone one. Level and "no upstream" look alike on the row — accepted (git's short `trackshort`
/// has `=` for level; here it is empty). Every local branch with an upstream draws it, a branch behind too.
RowLayout {
    id: track
    property int ahead: 0
    property int behind: 0
    property color tint: Theme.textSecondary

    /// One colour for the pair: `warning` once anything is behind — something came in from outside, to act on before a
    /// push: the "diverged" a standing state colour is allowed for (デザイン規約 §状態), the same colour on the same
    /// condition as the toolbar's `push -f` (§リモートへ送る). Ahead alone is plain: a push goes through as it is, the
    /// ordinary state no standing colour is for.
    readonly property color hue: track.behind > 0 ? Theme.warning : track.tint

    /// The gap between the legs; inside a leg, mark and number are one thing.
    spacing: Theme.spaceXs
    /// The numbers', so a line of words can stand the pair on its baseline (`NavFactLine`). The legs stand alike, so
    /// whichever is drawn answers.
    baselineOffset: track.ahead > 0 ? aheadLeg.y + aheadLeg.baselineOffset : behindLeg.y + behindLeg.baselineOffset

    /// One arrow and its count, cut like the graph row's tally (`WipTallyRow.Tally`): no step between them.
    component Leg: RowLayout {
        id: leg
        required property int turn
        required property int count
        required property color hue
        spacing: 0
        baselineOffset: countWord.y + countWord.baselineOffset
        // The `↑` / `↓` before the count, drawn in their place (デザイン規約 §アイコンの席 の字の代替): its box is the
        // count's line, its cell the face's own room round it, as the character's would be.
        NavIcon {
            id: legMark
            Layout.preferredWidth: legMark.wordCell
            Layout.preferredHeight: countWord.implicitHeight
            kind: "arrow"
            set: "sign"
            characterStandIn: true
            size: countWord.font.pixelSize
            wordBase: countWord.baselineOffset
            turn: leg.turn
            tint: leg.hue
            width: legMark.wordCell
            height: countWord.implicitHeight
        }
        PlainTextLabel {
            id: countWord
            text: leg.count
            color: leg.hue
            font.pixelSize: Theme.fontSm
        }
    }

    Leg {
        id: aheadLeg
        turn: -90
        count: track.ahead
        hue: track.hue
        visible: track.ahead > 0
        Layout.alignment: Qt.AlignVCenter
    }
    Leg {
        id: behindLeg
        turn: 90
        count: track.behind
        hue: track.hue
        visible: track.behind > 0
        Layout.alignment: Qt.AlignVCenter
    }
}
