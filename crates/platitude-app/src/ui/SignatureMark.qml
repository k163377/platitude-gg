import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A signature that holds is a tick and nothing more; only one that
// contradicts the content spends words (規約 §署名の表示).
//
// Where it stands is the caller's to say, and there are two seats for it.
// In the details pane it rides the committer's name, on the shoulder every
// other `!` in this app stands on (規約 §署名の表示). In the commit editor
// there is no name to hang it off — the phrase names a branch
// — so it rides the corner of the face the phrase already carries, the way
// the pen rides the other one. One shape either way, so both panes say
// "signature" in one vocabulary.
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
    /// (PGG_AUTO_ACT=signature-tip) — hover cannot be injected.
    property bool pointedAt: false
    /// The ink of the mark itself; the disc under it is that plus its
    /// own ring, the same arithmetic the pen badge does. A smaller face
    /// hands in a smaller ink
    /// (デザイン規約 §アバターを与える).
    property int ink: Theme.iconSm
    /// Line weight. A mark standing at `iconSm` beside a word carries 4/3
    /// the weight of the letters unless the caller hands the grid ratio in,
    /// and weight is what the eye reads as size (`NavIcon.stroke`) — so the
    /// seat on a name asks for it and the badge on a face does not.
    property real stroke: Metrics.iconStroke

    /// The tooltip is up. Reported through the ToolTip's own visible —
    /// the output side, so a cut binding cannot read as green.
    readonly property bool tipShown: signatureMark.ToolTip.visible
    /// Whether the pointer is on the badge itself, which the face under
    /// it reads to keep its own tooltip out of the way (規約 §hover
    /// 「1 つのポインタが開けるものは 1 つ」).
    readonly property bool pointed: signatureHover.hovered || signatureMark.pointedAt

    /// The air this mark's own square holds past its ink, on whichever
    /// side is set against something. **The seat is the ink**
    /// (規約 §余白): the mark is drawn well inside its square, so one
    /// placed a pixel off a frame — or half a gap off a name — would leave
    /// several between that and anything actually drawn. Whoever places
    /// this adds the air back, and each kind answers for its own drawing
    /// (`NavIcon.inkGrid` / `inkRightGrid` / `inkTopGrid`). **The two kinds
    /// disagree by more than a hair**: the tick's ink starts 3.5 of the
    /// grid in and the bang's 7, so a seat written to one of them stands
    /// the other somewhere else.
    ///
    /// The grid air as drawn, with nothing taken off for the stroke:
    /// `inkWidth` adds a line to a span because there a stroke straddles
    /// a path running along the axis being measured, and none of these
    /// does.
    readonly property real inkAirLeft: (mark.inkRightGrid - mark.inkGrid) / 16 * signatureMark.ink
    readonly property real inkAirRight: (16 - mark.inkRightGrid) / 16 * signatureMark.ink
    readonly property real inkAirTop: mark.inkTopGrid / 16 * signatureMark.ink

    readonly property bool broken: signatureMark.kind === "bad"
    readonly property color tone: signatureMark.kind === "verified" ? Theme.success
        : signatureMark.broken ? Theme.danger : Theme.textSecondary
    /// Conclusion first, one line (デザイン規約 §hover のツールチップ).
    /// git's verdict by default; the commit editor's badge is a setting,
    /// so it hands its own sentence in.
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

    // **The mark is bare.** The pen's disc makes the pen legible on a
    // face of any colour, but this mark is never on one: beside a name it
    // stands on the pane's own ground, and on a face it is placed past the
    // round corner. A ring either way would only be a
    // second small frame beside whatever frame it is set against.
    NavIcon {
        id: mark
        anchors.fill: parent
        kind: signatureMark.broken ? "bang" : "check"
        tint: signatureMark.tone
        stroke: signatureMark.stroke
    }
    ToolTip.visible: signatureMark.pointed
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: signatureMark.tip
    // A handler: handlers are passive, so the face
    // under this one keeps the hover that raises its own badge (規約
    // §hover のツールチップ).
    HoverHandler { id: signatureHover }
}
