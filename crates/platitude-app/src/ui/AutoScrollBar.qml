import QtQuick
import QtQuick.Controls.Fusion

// A scroll bar pinned visible while its view overflows. The default AsNeeded policy re-derives visibility from
// transient view state and has been seen dropping the bar entirely around model swaps, so the policy is computed from
// content size instead. Attached to a Flickable, the bar's parent is the view itself. The comparison carries a pixel of
// slack. Text heights are fractional -- a font whose line box is 23.5 makes a two-line item 47.0 inside a frame laid
// out at 47 -- and a strict `>` turns a rounding remainder no eye can see into a bar down the side of a view that has
// nothing to scroll (2026-08-15 ユーザー報告). A real overflow is a line of text, never a fraction of one, so nothing that
// should scroll is lost by this.
//
// **Above anything else pinned to the view's frame.** An overlay laid over the rows (`GraphHeadPin`'s band, the
// sidebar's `HeadPinRow`) is a child of the view like this bar is, and is built later, so at equal z it lands on top
// and takes the bar with it — which is where the reader is in two thousand rows (2026-08-22 ユーザー報告). One step up
// is all it takes, and it is the right way round on its own terms: nothing in a view stands over its scroll bar.
ScrollBar {
    /// The view this bar answers for. A bar put on a `Flickable` is parented to it, which is what the default reads;
    /// one put on a `ScrollView` is parented to the view instead, and the flickable is that view's `contentItem` — so
    /// those name it (`DescriptionBox` / `MessageEditor`). **A `ScrollView`'s bar has to be told**: left to the style it
    /// stands only while the pointer is on its own 10px strip, so a box scrolled by the wheel never shows one at all
    /// (qmltestrunner 実測 2026-08-27: `active` false at rest, false after `contentY` is assigned, true only on hover of
    /// the bar).
    property Flickable view: parent as Flickable
    z: 1
    policy: view && view.contentHeight + view.topMargin + view.bottomMargin > view.height + 1
            ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
    /// **No fade: a bar with nowhere to go is not drawn at all.** The style keeps the thumb painted for 450ms and then
    /// takes 200ms over it, so a view that had somewhere to go for one frame of layout went on showing a bar for two
    /// thirds of a second after it stopped having anywhere — a full-height bar standing over a box that fits (2026-08-27
    /// ユーザー報告 / 実測: the summary box settles at `policy=AsNeeded size=1 active=false` with the thumb still at 0.75).
    /// Going away costs nothing on the way back in: entering the style's own "active" state is instant, and only
    /// leaving it is animated.
    ///
    /// The policy above is the whole question, which is why the style's second term (`active && size < 1`) is not
    /// repeated here: that term is what an `AsNeeded` bar has instead of an answer, and here there is one. It would
    /// also be circular — `active` is raised by the pointer, and a bar that is not drawn cannot be pointed at.
    visible: policy === ScrollBar.AlwaysOn
}
