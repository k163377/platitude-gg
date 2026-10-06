pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// What each of the band's state badges would like to be drawn at, measured off labels nothing lays out: the badge
/// row stops being laid out when the group folds, and a width read from it then would never let the words back.
Item {
    id: metrics

    visible: false

    /// The working tree of the tab in front, which is where the operation's own words come from. Null while no tab is.
    required property var stateWorkingTree
    /// Whether the operation also has a second word and a step count, both drawn inside its badge.
    required property bool hasAlso
    required property bool hasStep
    /// The words' floor, in characters (規約 §ウィンドウの縁).
    required property int minChars

    /// Whole pixels: boxes are laid out on whole ones, and a ceiling summed from the fractions comes out under what
    /// the rounded widths add up to — the share-out then finds itself short at exactly the natural width, and every
    /// word elides in a band with room to spare.
    readonly property int opW: Math.ceil(mOpText.implicitWidth
                                          + (metrics.hasAlso
                                             ? 2 * Theme.spaceXs + mDot.implicitWidth + mOpAlso.implicitWidth : 0)
                                          + (metrics.hasStep ? Theme.spaceXs + mOpStep.implicitWidth : 0))
        + 2 * Theme.spaceXs
    readonly property int conflictW: Math.ceil(mConflict.implicitWidth) + 2 * Theme.spaceXs
    readonly property int identityW: Math.ceil(mIdentity.implicitWidth) + 2 * Theme.spaceXs
    readonly property int oldGitW: Math.ceil(mOldGit.implicitWidth) + 2 * Theme.spaceXs
    readonly property int staleW: Math.ceil(mStale.implicitWidth) + 2 * Theme.spaceXs
    readonly property int lfsW: Math.ceil(mLfs.implicitWidth) + 2 * Theme.spaceXs

    /// The badge box, the band's and the card's alike (規約 §ウィンドウの縁「バッジの箱」). The words are capitals only,
    /// so the box is cut round the capitals' ink — the line's descent is air no letter uses: as much air over the cap
    /// height as under the baseline, the most of it that fits in the band's box step (`iconXl`, the `+`'s wash). Whole
    /// pixels, so the frame's lines stay sharp and the two airs come out equal.
    readonly property int capRows: Math.round(stateFont.capitalHeight)
    readonly property int air: Math.floor((Theme.iconXl - 2 * Theme.borderWidth - metrics.capRows) / 2)
    readonly property int depth: metrics.capRows + 2 * metrics.air + 2 * Theme.borderWidth
    /// Where the words' baseline stands, down from the box's top.
    readonly property int wordBase: Theme.borderWidth + metrics.air + metrics.capRows
    /// Where a word's line goes in the box so its baseline lands on `wordBase`, for words laid out by a row (a row
    /// cannot be anchored by a baseline).
    readonly property real wordTop: metrics.wordBase - stateFont.ascent
    /// Where the capitals' middle stands, down from the top of a word's line: the drawn dot between two words sits on
    /// it (`DotMark`).
    readonly property real capMiddle: stateFont.ascent - metrics.capRows / 2

    /// The narrowest a badge is drawn with words in it. Settled by `settleMinW`: `advanceWidth` is a method, so a
    /// binding on it holds the default font's answer (rules-refs/app-ui.md「`FontMetrics.advanceWidth()` も同じ側」).
    /// Counted in `n`s, not `averageCharacterWidth` (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」).
    property real minW: 0
    function settleMinW() {
        metrics.minW = Math.ceil(stateFont.advanceWidth("…") + metrics.minChars * stateFont.advanceWidth("n"))
            + 2 * Theme.spaceXs
    }

    FontMetrics {
        id: stateFont
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        font.weight: Theme.fontWeightStrong
        // On the font's change too: the initial evaluation still sees the default font.
        onFontChanged: metrics.settleMinW()
        Component.onCompleted: metrics.settleMinW()
    }

    component BadgeWord: Label {
        visible: false
        font.pixelSize: Theme.fontSm
        font.weight: Theme.fontWeightStrong
    }
    BadgeWord {
        id: mOpText
        text: metrics.stateWorkingTree !== null ? metrics.stateWorkingTree.opText : ""
    }
    BadgeWord {
        id: mOpAlso
        text: metrics.stateWorkingTree !== null ? metrics.stateWorkingTree.opAlso : ""
    }
    BadgeWord {
        id: mOpStep
        text: metrics.stateWorkingTree === null ? ""
              : qsTr("%1/%2").arg(metrics.stateWorkingTree.opStep).arg(metrics.stateWorkingTree.opSteps)
    }
    BadgeWord {
        id: mConflict
        text: Words.badgeConflicts
    }
    BadgeWord {
        id: mIdentity
        text: Words.badgeSetIdentity
    }
    BadgeWord {
        id: mOldGit
        text: Words.badgeOldGit
    }
    BadgeWord {
        id: mStale
        text: Words.badgeStaleGraph
    }
    BadgeWord {
        id: mLfs
        text: Words.badgeNoLfs
    }
    DotMark {
        id: mDot
        visible: false
    }
}
