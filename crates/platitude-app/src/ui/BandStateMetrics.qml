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
    required property var stateWt
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
        font.weight: Font.DemiBold
        // On the font's change too: the initial evaluation still sees the default font.
        onFontChanged: metrics.settleMinW()
        Component.onCompleted: metrics.settleMinW()
    }

    component BadgeWord: Label {
        visible: false
        font.pixelSize: Theme.fontSm
        font.weight: Font.DemiBold
    }
    BadgeWord {
        id: mOpText
        text: metrics.stateWt !== null ? metrics.stateWt.opText : ""
    }
    BadgeWord {
        id: mOpAlso
        text: metrics.stateWt !== null ? metrics.stateWt.opAlso : ""
    }
    BadgeWord {
        id: mOpStep
        text: metrics.stateWt === null ? ""
              : qsTr("%1/%2").arg(metrics.stateWt.opStep).arg(metrics.stateWt.opSteps)
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
    DotMark {
        id: mDot
        visible: false
    }
}
