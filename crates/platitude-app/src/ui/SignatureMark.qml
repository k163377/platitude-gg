import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A signature that holds is a tick and nothing more; only one that
// contradicts the content spends words (規約 §署名の表示).
//
// The mark rides a corner of the face it is about, the way the pen rides
// the other one: both are things said about the person under them, and a
// corner apiece is what keeps the pair from ever standing on each other.
// The commit editor's badge is the same shape over the same face, so the
// two panes say "signature" in one vocabulary.
//
// Green stays with the signatures git actually vouched for. One it
// could read but not judge gets the same tick in textSecondary: the
// shape says a signature is there, the colour says nobody here
// checked it — and that is also what the editor's badge means, where
// nothing has been signed yet and the setting is all there is to say.
Item {
    id: signatureMark

    /// What git makes of the signature: "" (unsigned, or not answered
    /// yet), "verified", "signed" or "bad". The letter behind it is
    /// git's own `%G?` code, which is what the tooltip needs — the mark
    /// says one of three things, but the reason a signature could not
    /// be judged is one of five.
    property string kind: ""
    property string code: ""
    property string signer: ""
    /// Stands in for the pointer, so the tooltip can be photographed
    /// (PG_AUTO_ACT=signature-tip) — hover cannot be injected.
    property bool pointedAt: false
    /// The ink of the mark itself; the disc under it is that plus its
    /// own ring, the same arithmetic the pen badge does. A smaller face
    /// hands in a smaller ink rather than wearing the same badge twice
    /// over (デザイン規約 §アバターを与える).
    property int ink: Theme.iconSm

    /// The tooltip is up. Reported through the ToolTip's own visible —
    /// the output side, so a cut binding cannot read as green.
    readonly property bool tipShown: signatureMark.ToolTip.visible
    /// Whether the pointer is on the badge itself, which the face under
    /// it reads to keep its own tooltip out of the way (規約 §hover
    /// 「1 つのポインタが開けるものは 1 つ」).
    readonly property bool pointed: signatureHover.hovered || signatureMark.pointedAt

    /// The air this mark's own square holds past its ink, on the two
    /// sides that are set against something. **The seat is the ink, not
    /// the box** (規約 §余白): the mark is drawn well inside its square
    /// on both of those axes, so one placed a pixel off a frame would
    /// leave several between the frame and anything actually drawn.
    /// Whoever places this adds these back, and each kind answers for
    /// its own drawing (`NavIcon.inkRightGrid` / `inkTopGrid`).
    ///
    /// The grid air as drawn, with nothing taken off for the stroke:
    /// `inkWidth` adds a line to a span because there a stroke straddles
    /// a path running along the axis being measured, and neither of
    /// these does.
    readonly property real inkAirRight: (16 - mark.inkRightGrid) / 16 * signatureMark.ink
    readonly property real inkAirTop: mark.inkTopGrid / 16 * signatureMark.ink

    readonly property bool broken: signatureMark.kind === "bad"
    readonly property color tone: signatureMark.kind === "verified" ? Theme.success
        : signatureMark.broken ? Theme.danger : Theme.textSecondary
    /// Conclusion first, one line (デザイン規約 §hover のツールチップ).
    /// git's verdict by default; the commit editor's badge is a setting
    /// rather than a verdict, so it hands its own sentence in.
    property string tip:
        signatureMark.code === "G"
        ? (signatureMark.signer !== ""
           ? qsTr("Signed by %1").arg(signatureMark.signer)
           : qsTr("Signed by a key you trust"))
        : signatureMark.code === "B"
          ? qsTr("The content changed after it was signed")
        : signatureMark.code === "U"
          // git names no signer for this one: the key it read is not one
          // it can put a name to.
          ? qsTr("The key is not one you have vouched for")
        : signatureMark.code === "X"
          ? qsTr("The signature has expired")
        : signatureMark.code === "Y"
          ? qsTr("The signing key has expired")
        : signatureMark.code === "R"
          ? qsTr("The signing key was revoked")
        : qsTr("Cannot be checked — the key is not here")

    visible: signatureMark.kind !== ""
    implicitWidth: signatureMark.ink
    implicitHeight: signatureMark.ink
    // Anchored rather than laid out by a layout, so the implicit sizes
    // above have to be taken by hand.
    width: signatureMark.implicitWidth
    height: signatureMark.implicitHeight

    // **No disc under it.** The pen's is what makes the pen legible on a
    // face of any colour, but this mark is placed to sit past the corner
    // of the round face rather than on it — so it already stands on the
    // pane's own ground, and a ring there would only be a second small
    // frame beside whatever frame it is set against.
    NavIcon {
        id: mark
        anchors.fill: parent
        kind: signatureMark.broken ? "bang" : "check"
        tint: signatureMark.tone
    }
    ToolTip.visible: signatureMark.pointed
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: signatureMark.tip
    // A handler, not a MouseArea: handlers are passive, so the face
    // under this one keeps the hover that raises its own badge (規約
    // §hover のツールチップ).
    HoverHandler { id: signatureHover }
}
