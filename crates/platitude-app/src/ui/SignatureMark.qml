import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A tick for a signature, a bang for a bad one. The seat is the caller's: beside the committer's name, or on the corner
// of the commit editor's face, which has no name to hang it off (規約 §署名の表示).
Item {
    id: signatureMark

    /// What git makes of the signature: "" (unsigned, or not answered yet), "verified", "signed" or "bad". `code` is
    /// git's own `%G?` letter, which the tooltip needs: the mark says one of three things, the reasons are more.
    property string kind: ""
    property string code: ""
    property string signer: ""
    /// Stands in for the pointer, so the tooltip can be photographed (PGG_AUTO_ACT=signature-tip).
    property bool pointedAt: false
    /// The ink of the mark itself. A smaller face hands in a smaller ink (デザイン規約 §アバターを与える).
    property int ink: Theme.iconSm
    /// Line weight: the seat beside a name hands in the letters' weight (`NavIcon.stroke`, 規約 §署名の表示); the
    /// badge on a face keeps the default.
    property real stroke: Metrics.iconStroke

    /// The tooltip is up, read off the ToolTip's own visible — the output side, so a cut binding cannot read as green.
    readonly property bool tipShown: signatureMark.ToolTip.visible
    /// Whether the pointer is on the badge itself, which the face under it reads to keep its own tooltip out of the way
    /// (規約 §hover のツールチップ「1 つのポインタが開けるものは 1 つ」).
    readonly property bool pointed: signatureHover.hovered || signatureMark.pointedAt

    /// The air this mark's square holds past its ink on each side, for the caller to take off its seat — the seat is
    /// the ink (規約 §余白). Per kind (`NavIcon.inkGrid` / `inkRightGrid` / `inkTopGrid`). No stroke is added, unlike
    /// `inkWidth`: no stroke here runs along the axis being measured.
    readonly property real inkAirLeft: (mark.inkRightGrid - mark.inkGrid) / 16 * signatureMark.ink
    readonly property real inkAirRight: (16 - mark.inkRightGrid) / 16 * signatureMark.ink
    readonly property real inkAirTop: mark.inkTopGrid / 16 * signatureMark.ink

    readonly property bool broken: signatureMark.kind === "bad"
    readonly property color tone: signatureMark.kind === "verified" ? Theme.success
        : signatureMark.broken ? Theme.danger : Theme.textSecondary
    /// One line, conclusion first (デザイン規約 §hover のツールチップ). git's verdict by default; the commit editor's
    /// badge is a setting and hands its own sentence in.
    property string tip:
        signatureMark.code === "G"
        ? (signatureMark.signer !== ""
           ? qsTr("Signed by %1").arg(signatureMark.signer)
           : qsTr("Signed by a key you trust"))
        : signatureMark.code === "B"
          ? qsTr("The content changed after it was signed")
        : signatureMark.code === "U"
          // git names no signer for this one: it cannot put a name to the key.
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

    // Bare, unlike the pen: it never stands on a face, so a disc would only be a second small frame (規約 §署名の表示).
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
    // A handler, being passive: the face under this keeps the hover that raises its own badge (規約 §hover のツールチップ).
    HoverHandler { id: signatureHover }
}
