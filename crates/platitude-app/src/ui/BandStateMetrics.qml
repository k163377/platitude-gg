pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// What each of the band's state badges would like to be drawn at, measured off labels that are never drawn.
///
/// **The badges in the band cannot also be what the cap is measured from.** A `RowLayout` that is not being laid out
/// reports the width it had when it last was, and the row of badges goes away the moment the group folds — so a cap
/// read from there makes the fold one that nothing comes back from (measured, `cap=32` with a 1440-wide window,
/// and no width would bring the words back).
///
/// One of the measuring components the window is settled with (`TabMetrics` / `BandWidest` / `DiffTextMetrics`),
/// and for the same reason: a width read off the thing being laid out is a width that has already given way.
Item {
    id: metrics

    visible: false

    /// The working tree of the tab in front, which is where the operation's own words come from. Null while no tab is.
    required property var stateWt
    /// Whether the operation has a second word and a step count beside its own — both are drawn inside the one badge,
    /// so both are counted into its width.
    required property bool hasAlso
    required property bool hasStep
    /// The narrowest a badge is drawn before the group gives up on words, in characters (規約 §ウィンドウの縁) — the
    /// same count costs a different number of pixels in each platform's UI font.
    required property int minChars

    /// Whole pixels, for the reason the tab names are settled in them (規約 §ウィンドウの縁): a word asks for a
    /// fractional width, a box is laid out on a whole one, and a ceiling summed from the fractions is a few pixels
    /// under what the same widths add up to when each is rounded — so the group is handed exactly its natural width
    /// and the share-out still finds itself short, and every word elides in a band with room to spare (measured, on
    /// Linux: `cap=103` with `groupW=270`, which was the natural width).
    readonly property int opW: Math.ceil(mOpText.implicitWidth
                                          + (metrics.hasAlso
                                             ? 2 * Theme.spaceXs + mDot.implicitWidth + mOpAlso.implicitWidth : 0)
                                          + (metrics.hasStep ? Theme.spaceXs + mOpStep.implicitWidth : 0))
        + 2 * Theme.spaceXs
    readonly property int conflictW: Math.ceil(mConflict.implicitWidth) + 2 * Theme.spaceXs
    readonly property int identityW: Math.ceil(mIdentity.implicitWidth) + 2 * Theme.spaceXs
    readonly property int oldGitW: Math.ceil(mOldGit.implicitWidth) + 2 * Theme.spaceXs
    readonly property int staleW: Math.ceil(mStale.implicitWidth) + 2 * Theme.spaceXs

    /// The narrowest a badge is drawn with words in it. Settled by `settleMinW` rather than bound: `advanceWidth` is a
    /// method and takes no binding dependency, so a binding on it holds whatever the *default* font measured
    /// (app-ui.md 「FontMetrics.advanceWidth も同じ側」).
    ///
    /// Counted in `n`s, never in `averageCharacterWidth` (`TabMetrics.titleMinW` and `GraphColumnMetrics.chipNameMinW`
    /// for the same reason): that is the **font's** average, and every family named for this UI carries Japanese, so it
    /// answers with a full-width figure no operation is spelled in — and a different one per platform
    /// (rules-refs/app-ui.md carries the measurement). A floor read off it moves with the font rather than with the
    /// letters, and where it runs wide it folds the words away in a band that still had room for them.
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
        // On the metrics' own change signal, so the ellipsis is measured
        // in the settled font — the initial evaluation still sees the
        // default one.
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
