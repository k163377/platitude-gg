import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The word half of a toolbar action: the wording itself, a command's flag drawn rather than typed, the chip a command
// wears, and the mark that says the last go did not work. The box it is measured into is the widest wording the button
// ever says, so the toolbar does not move when the state does.
Item {
    id: btnLabel

    /// What this state says.
    property string text: ""
    /// The label is a git command said in git's own spelling, and wears the chip that says so (デザイン規約 §git 用語のコード表記).
    property bool code: false
    /// Text the box is measured for, and whether that wording is a command (the two families measure differently, so
    /// the box has to be told which one it is holding).
    property string widestText: ""
    property bool widestCode: false
    /// The colour the word is drawn in — the button's own `fg`.
    property color tint: Theme.textPrimary
    /// The last go at what this button does did not work.
    property bool alert: false
    property color alertTone: btnLabel.tint
    /// Pull the mark back to a letter's distance from the word.
    property bool alertTight: false
    property int fontSize: Theme.fontMd

    // The box is the widest wording's ink and nothing else. What stands between that ink and the frame is the button's
    // own air, shared out by one rule in every state (`ActionButton.slack`) — the widest included, whose slack is
    // nothing and whose air is therefore the padding itself. No extra gap charged to one family and not the other: that
    // makes the box jump whenever the two wordings cross in width.
    //
    // Measured from the font even where the flag is drawn (see below) — the box is what holds the toolbar still, and it
    // must not move when a shorter rule is chosen for the flag.
    readonly property real box: widest.implicitWidth
    /// A command's flag, set apart from the command itself so its dashes can be drawn rather than typed. Every dash the
    /// mono family carries is the same 7px rule in an 8px cell (measured over U+002D / 2010 / 2011 / 2212), and on the
    /// wording the shared box was measured for that is what leaves the mark no room past the word. Drawn, the rule's
    /// length and the air either side are ours to pick (デザイン規約 §git 用語のコード表記).
    readonly property int flagAt: btnLabel.code ? btnLabel.text.indexOf(" -") : -1
    readonly property bool splitFlag: btnLabel.flagAt > 0
    readonly property string head: btnLabel.splitFlag
        ? btnLabel.text.substring(0, btnLabel.flagAt) : btnLabel.text
    /// The flag with its leading dashes taken off, and how many of them there were.
    readonly property string flagRest: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).replace(/^-+/, "") : ""
    readonly property int dashCount: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).length - btnLabel.flagRest.length : 0

    /// The label is a phrase with a command at each end rather than a single word: a chip, the words between them, and
    /// a second chip in a colour of its own. The commit button is the one button that says one — it names the command
    /// *and* where the command will land, and neither half is a phrase about the other (デザイン規約 §git 用語のコード
    /// 表記). Empty leaves the ordinary single-wording path exactly as it was.
    property string phraseHead: ""
    /// The one number in the phrase, kept apart from the words around it so the words can give way without taking it
    /// (デザイン規約 §git 用語のコード表記).
    property string phraseCount: ""
    property string phraseTail: ""
    property color phraseTailTint: btnLabel.tint
    /// The hold's mark, when the phrase is on a button that is held rather than clicked.
    ///
    /// **Inside the phrase rather than in the button's own seat.** The seat stands at the row's left edge, and a
    /// phrase is centred in a cell that fills the row — so a mark left out there ends up alone against the far edge,
    /// with the words it is meant to introduce adrift in the middle. §長押し puts the mark immediately ahead of the
    /// word; here that means ahead of the first chip (2026-08-22 実測: the stranded mark).
    property int phraseHoldMs: 0
    property real phraseHoldProgress: 0
    /// Whom the phrase's action will be attributed to, at its end (`… by <face>`). -1 draws nothing at all — this is
    /// the commit button's own ending, and no other button has one (デザイン規約 §アバターを与える).
    property int phraseFace: -1
    property string phraseFaceUrl: ""
    /// How many more people the same action credits, as the `+N` the graph's chips and the co-author line already use.
    /// Zero says nothing: the ordinary case carries no mark.
    property int phraseMates: 0
    /// What is said about that person's signature, in the mark's own vocabulary (`SignatureMark.kind`), and the one
    /// line its tooltip carries. Empty puts nothing on the face.
    property string phraseSignature: ""
    property string phraseSignatureTip: ""
    /// Stands in for the pointer on that tick, so its one line can be photographed — hover cannot be injected.
    property bool phraseSignaturePointedAt: false
    /// Whether that line is on screen — the tooltip's own visible, so a cut binding cannot read as green.
    readonly property bool phraseSignatureTipShown: phraseAvatar.signatureTipShown
    readonly property bool phrased: btnLabel.phraseHead !== ""
    /// How wide the phrase may be: **the width this cell was given**, never the width it would like. A phrased button
    /// fills a pane rather than being measured to its own content, and a request for the phrase's full width is a
    /// floor the pane cannot go under — the column would stand as wide as the longest branch name anyone has checked
    /// out, and every box sized to fill it paints past the window's edge (the accident `details-fit` catches, on this
    /// side of the app). Reading the given width closes no ring: nothing here feeds the request back.
    readonly property real phraseRoom: btnLabel.phrased ? btnLabel.width : 0
    // ---- what gives, and in what order ---------------------------------
    //
    // **The ref is what the reader can be wrong about, so it is the last thing to give** (2026-08-18 ユーザー判断).
    // The parts that never give are the ones with no length of their own: the mark, the command, the count, the face.
    // What gives, weakest first: the `+N`, then the app's own words, then — only if there is still nothing left — the
    // ref itself.
    /// What the parts that never give are holding, each with the gap that follows it. Counted per part rather than as
    /// a lump: a `Row` charges a gap for every visible child, so a part that is not there does not owe one.
    readonly property real phraseFixed:
        (btnLabel.phraseHoldMs > 0 ? Theme.iconSm + phraseRow.spacing : 0)
        + headChip.implicitWidth + phraseRow.spacing
        + (countWord.visible ? countWord.implicitWidth + phraseRow.spacing : 0)
        + (btnLabel.phraseFace >= 0 ? byWord.implicitWidth + phraseAvatar.width + 2 * phraseRow.spacing : 0)
    /// What is left for the three that do.
    readonly property real phraseFree: Math.max(0, btnLabel.phraseRoom - btnLabel.phraseFixed)
    /// The ref, given everything the weaker two do not need. Never below a floor: elided past this a branch name says
    /// nothing at all, and at that point the words may as well go too.
    readonly property real tailCap:
        btnLabel.phraseRoom <= 0 ? 0 : Math.max(Theme.buttonMinWidth,
                                                btnLabel.phraseFree - btnLabel.midCap - btnLabel.mateRoom)
    /// **What gives leaves a `…` behind.** Giving is not the same as never having been said: a phrase that drops its
    /// words silently reads as a complete sentence that happens to be terse, and the reader has no way to know a word
    /// was taken out. So the floor for anything that gives is the mark itself (2026-08-18 ユーザー判断).
    readonly property real cutMark: ellipsis.implicitWidth
    /// The app's own words: what the ref left them, down to the mark.
    readonly property real midCap:
        btnLabel.phraseRoom <= 0 || btnLabel.text === "" ? 0
        : Math.max(btnLabel.cutMark, Math.min(midWords.implicitWidth,
                                              btnLabel.phraseFree - tailWord.wantWidth - btnLabel.mateRoom
                                              - phraseRow.spacing))
    /// The `+N` goes first of all — it is the one part whose absence changes nothing about where the commit lands.
    /// Gone, it too leaves the mark: `by <face> …` still says there are more names here than the one drawn.
    readonly property bool mateFits:
        btnLabel.phraseFree - tailWord.wantWidth - midWords.implicitWidth
        >= mateFull.implicitWidth + phraseRow.spacing
    readonly property string mateWord: qsTr("+%1").arg(btnLabel.phraseMates)
    readonly property real mateRoom:
        btnLabel.phraseMates <= 0 ? 0
        : (btnLabel.mateFits ? mateFull.implicitWidth : btnLabel.cutMark) + phraseRow.spacing

    implicitWidth: btnLabel.phrased ? phraseRow.implicitWidth
                   : headText.width + (btnLabel.splitFlag ? flagRow.width : 0)
    implicitHeight: btnLabel.phrased ? phraseRow.implicitHeight : headText.implicitHeight
    Layout.maximumWidth: btnLabel.phrased ? Number.POSITIVE_INFINITY : 240
    // Its own width: what the shared box asks for past this wording is held by the button's padding (`slack`), so the
    // chip and the mark, both measured off this cell, keep sitting on the word.
    //
    // A phrase asks for nothing and takes what the row has: it is the only wording with a part that can give (the
    // ref), and a request for its full width is a floor the pane cannot go under (see `phraseRoom`).
    Layout.fillWidth: btnLabel.phrased
    Layout.preferredWidth: btnLabel.phrased ? 0 : btnLabel.implicitWidth
    Layout.alignment: Qt.AlignVCenter

    // Measured, never drawn: a hidden item is left out of the layout, and a Label measures the way the visible one does
    // — TextMetrics reports a few pixels tighter, which is enough of a difference to shift the toolbar it is here to
    // hold still.
    Label {
        id: widest
        visible: false
        text: btnLabel.widestText
        font.family: btnLabel.widestCode ? Theme.monoFamily : Theme.uiFamily
        font.wordSpacing: btnLabel.widestCode ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
    }
    // Also measured, never drawn: the mark a cut leaves, and the `+N` at the length it would like to be. Both are read
    // while working out who gives what, so neither can be the item whose width the answer sets.
    Label {
        id: ellipsis
        visible: false
        text: "…"
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    Label {
        id: mateFull
        visible: false
        text: btnLabel.mateWord
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    // The phrase, when there is one: chip, words, chip. A row rather than anchors, because what has to line up here is
    // three baselines and two gaps — and the chips hang their own ground half a gap past their glyphs, so the gaps are
    // the ordinary word spacing the sentence would have had bare.
    Row {
        id: phraseRow
        visible: btnLabel.phrased
        // Centred in the cell rather than packed against its left edge: the cell fills the button's row (see
        // `Layout.fillWidth` above), and the two spacers `centred` puts either side cannot centre a cell that fills.
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceXs
        HoldIcon {
            visible: btnLabel.phraseHoldMs > 0
            progress: btnLabel.phraseHoldProgress
            tint: btnLabel.tint
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: Metrics.opticalDrop
        }
        CodeChip {
            id: headChip
            word: btnLabel.phraseHead
            tint: btnLabel.tint
            size: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The count never gives: it is the one number in the phrase, and a `5` costs nothing to keep. Out entirely
        // when there is none, rather than standing empty — a `Row` charges a gap for a child of no width, and the
        // command would sit two gaps off the word after it.
        Label {
            id: countWord
            visible: btnLabel.phraseCount !== ""
            text: btnLabel.phraseCount
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The app's own words, which give before the ref does.
        Label {
            id: midWords
            visible: btnLabel.midCap > 0
            width: Math.min(implicitWidth, btnLabel.midCap)
            elide: Text.ElideRight
            text: btnLabel.text
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // **The ref is the one part that gives.** Everything else in this phrase is bounded by what it says — a
        // command, a count, a face, a `+N` — but **a branch name has no length git will not take**, so this is the one
        // part that has to be capped: what is left for it is the room the button has, less what the fixed parts hold.
        CodeChip {
            id: tailWord
            word: btnLabel.phraseTail
            tint: btnLabel.phraseTailTint
            size: btnLabel.fontSize
            cap: btnLabel.tailCap
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            id: byWord
            visible: btnLabel.phraseFace >= 0
            text: qsTr("by")
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The face, wearing the same corner mark it wears anywhere else (`AvatarButton`) — no address, so nothing here
        // is pressable: the phrase is the button, and a target inside a target is two things to aim at.
        //
        // `iconLg`, which is the step the graph draws a node at beside a `fontMd` subject: a face is read as a picture
        // rather than as a glyph, so it wants the proportion the graph's rows already give it, not the one a mark
        // beside a word gets (2026-08-18 報告).
        AvatarButton {
            id: phraseAvatar
            visible: btnLabel.phraseFace >= 0
            face: btnLabel.phraseFace
            faceUrl: btnLabel.phraseFaceUrl
            signatureKind: btnLabel.phraseSignature
            signatureTip: btnLabel.phraseSignatureTip
            signaturePointedAt: btnLabel.phraseSignaturePointedAt
            badgeInk: Theme.iconXs
            // Clear of the round face's corner by a step, one pixel short of the frame beside it, so what the eye
            // reads is a tick beside a face rather than a tick on one (§署名の表示).
            badgeTopOut: Theme.spaceXs - Theme.borderWidth
            badgeRightOut: Theme.spaceXs - Theme.borderWidth
            width: Theme.iconLg
            height: Theme.iconLg
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            id: mateCount
            visible: btnLabel.phraseMates > 0
            text: btnLabel.mateFits ? btnLabel.mateWord : ellipsis.text
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
    }
    Label {
        id: headText
        visible: !btnLabel.phrased
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // Bounded by the cell's own ceiling rather than by its width: the width comes from this, so reading it back
        // would close a loop.
        width: Math.min(implicitWidth, btnLabel.Layout.maximumWidth)
        text: btnLabel.head
        color: btnLabel.tint
        font.family: btnLabel.code ? Theme.monoFamily : Theme.uiFamily
        // A command and its flag are one thing said, and a mono space is far wider than the air the chip keeps at its
        // own ends — left alone, `-f` drifts away from the `push` it belongs to and the chip reads as two words on one
        // ground (デザイン規約 §git 用語のコード表記).
        font.wordSpacing: btnLabel.code ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
        elide: Text.ElideRight
    }
    // The flag: the air the mono space held, a drawn rule for each dash, and the letters after them in the font. The
    // rule is `borderWidth` thick because that is what the font's own dash measures at this size, and a hair of air
    // follows it so the letter does not touch.
    Row {
        id: flagRow
        visible: btnLabel.splitFlag
        anchors.left: headText.right
        anchors.verticalCenter: headText.verticalCenter
        spacing: 0
        Item {
            width: Theme.spaceXs
            height: headText.height
        }
        Repeater {
            model: btnLabel.dashCount
            Item {
                width: Theme.spaceXs + Theme.borderWidth
                height: headText.height
                Rectangle {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.spaceXs
                    height: Theme.borderWidth
                    color: btnLabel.tint
                }
            }
        }
        Label {
            text: btnLabel.flagRest
            color: btnLabel.tint
            font.family: Theme.monoFamily
            font.pixelSize: btnLabel.fontSize
        }
    }
    // The chip a command wears, behind the glyphs and only as wide as they are — the box around them is measured for
    // the longest wording of the pair, and a ground stretched to that would draw a chip the word does not fill. Half a
    // gap of tint hangs off either end, the same as a menu row's (デザイン規約 §git 用語のコード表記).
    Rectangle {
        z: -1
        visible: btnLabel.code
        x: -Theme.spaceXs / 2
        width: btnLabel.implicitWidth + Theme.spaceXs
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    // Past the word's end — past the chip's edge where there is one (a mark crossing that edge reads as stuck to the
    // chip rather than said after the word). Closer still after a flag: that is the longest thing the button says and
    // the one wording whose right-hand side is short of room (デザイン規約 §リモートへ送る).
    //
    // **A phrase that ends in a face puts the mark at its head instead.** That corner of the face is the signature's
    // (§署名の表示), and a yellow `!` standing there would read as something said about the signature, which it is
    // never about. The head is the one end of this phrase that carries nothing.
    NavIcon {
        visible: btnLabel.alert
        kind: "bang"
        tint: btnLabel.alertTone
        width: Theme.iconSm
        height: Theme.iconSm
        // Raised, and a half-gap off the command — the seat every other `!` in the app has (the toolbar's `push -f`),
        // so the mark reads the same wherever it is met. On a phrase it stands at the head instead of the tail, for
        // the reason above.
        x: btnLabel.phrased
           ? phraseRow.x - width + Theme.spaceXs / 2
           : btnLabel.implicitWidth - (btnLabel.splitFlag ? Theme.spaceXs / 2 : 0)
             - (btnLabel.alertTight ? Theme.spaceXs : 0)
        y: btnLabel.phrased
           ? phraseRow.y - Theme.spaceXs
           : -Theme.spaceXs
    }
}
