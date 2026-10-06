pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The cards and tooltips put out under a pointer — signature, stash, path, author, co-authors. Hover cannot be
/// injected, so each verb names its target. Built by `AutoActDriver` only when a verb was given.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`, because naming the driver's type would be a circle: it is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var workingTree: driver.workingTree
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var carriedPane: driver.carriedPane

    /// Runs `act` if it is this family's and says whether it was; each verb belongs to one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "signature") {
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTimer.start()
        } else if (act === "signature-tip") {
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTipTimer.start()
        } else if (act === "stash-tip") {
            // Why the box refuses the caret — the row decides which refusal. `older` is the row under HEAD's.
            page.activateRow(graphModel.oidAt(
                arg === "older" ? graphModel.rowOf(workingTree.headOid) + 1 : Number(arg)))
            stashTipTimer.start()
        } else if (act === "path-tip") {
            // Row 0 is the elided leaf, or the folder chain with `-tree`. The argument picks the pane as `corner`
            // does, plus `carried:<copy>`, named because its rows share the all-zero id (`driver.rowOfCopy`). A copy
            // whose folder ends in `-tree` would be misread; the presets hold none. `@<row>` points at a later row, one
            // with rows on both sides.
            const rowAt = ("" + arg).indexOf("@")
            pathTipTimer.row = rowAt >= 0 ? Number(("" + arg).slice(rowAt + 1)) : 0
            const named = rowAt >= 0 ? ("" + arg).slice(0, rowAt) : "" + arg
            const wantsTree = named.endsWith("-tree")
            const pane = wantsTree ? named.slice(0, -5) : named
            pathTipTimer.carried = pane.startsWith("carried:") ? pane.slice(8) : ""
            pathTipTimer.wipSide = pane === "" || pane === "wip"
            if (pathTipTimer.carried !== "") {
                // The copy is stood on by the timer's first tick; its list does not exist before that.
                page.setWipTreeView(wantsTree)
            } else if (pathTipTimer.wipSide) {
                page.showWip()
                unstagedModel.setTreeView(wantsTree)
            } else {
                page.activateRow(graphModel.oidAt(Number(pane)))
                detailsModel.setTreeView(wantsTree)
            }
            pathTipTimer.start()
        } else if (act === "tip-copy" || act === "tip-sweep") {
            // `path-tip`'s plain row: the flattened view's whole path is the longest tooltip (the tree's is one word).
            page.showWip()
            unstagedModel.setTreeView(false)
            if (act === "tip-sweep")
                tipSweepTimer.start()
            else
                tipCopyTimer.start()
        } else if (act === "author-card" || act === "author-card-open") {
            // `-open` writes what the hover handler writes; read the pair together — "stayed shut" means something
            // only beside a run where it opened.
            page.activateRow(graphModel.oidAt(Number(arg)))
            authorCardTimer.start()
        } else if (act === "co-authors" || act === "co-authors-open") {
            // Same pairing as author-card. `<row>[:<width>]`, as `details-parents`: `min` is the pane's floor, where
            // the credit folds (規約 §co-author の表示).
            const widthAt = ("" + arg).indexOf(":")
            page.activateRow(graphModel.oidAt(Number(widthAt > 0 ? arg.substring(0, widthAt) : arg)))
            if (widthAt > 0) {
                const width = arg.substring(widthAt + 1)
                page.setDetailsWidth(width === "min" ? 0 : Number(width))
            }
            coAuthorTimer.start()
        } else {
            return false
        }
        return true
    }
    // Waits for gpg / ssh-keygen's verdict, which decides the mark.
    SampleTimer {
        id: signatureTimer
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTimer.stop()
            Harness.report("signature kind=" + page.selectedSignatureKind
                              + " code=" + page.selectedSignatureCode
                              + " signer=" + page.selectedSignatureSigner)
            driver.complete()
        }
    }
    // signature-tip / stash-tip: pointed at only once the state has landed, reported once the tip is shown.
    SampleTimer {
        id: signatureTipTimer
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTipTimer.stop()
            detailsPane.signaturePointedAt = true
            signatureTipReport.start()
        }
    }
    SampleTimer {
        id: signatureTipReport
        onTriggered: {
            if (!detailsPane.signatureTipShown)
                return
            signatureTipReport.stop()
            Harness.report("signature_tip code=" + page.selectedSignatureCode
                              + " tip=" + detailsPane.signatureTipShown)
            driver.complete()
        }
    }
    SampleTimer {
        id: stashTipTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            stashTipTimer.stop()
            detailsPane.summaryPointedAt = true
            stashTipReport.start()
        }
    }
    SampleTimer {
        id: stashTipReport
        onTriggered: {
            if (!detailsPane.summaryTipShown)
                return
            stashTipReport.stop()
            Harness.report(
            "stash_tip blocked=" + (detailsPane.editBlocked !== "")
            + " tip=" + detailsPane.summaryTipShown + " why=" + detailsPane.editBlocked)
            driver.complete()
        }
    }
    // path-tip: pointed at once the list has landed. The report reads the shared tooltip itself, the one thing that
    // also says its words.
    SampleTimer {
        id: pathTipTimer
        property bool wipSide: true
        /// The copy this run is pointing into, empty for the two panes of this window's own tree.
        property string carried: ""
        /// The row pointed at.
        property int row: 0
        property bool stood: false
        onTriggered: {
            if (pathTipTimer.carried !== "") {
                if (!pathTipTimer.standOnCopy())
                    return
                // Until the copy's own files arrive the pane holds this window's.
                const files = page.wipUnstaged
                if (page.carriedPath === "" || files.carriedAt !== page.carriedPath || files.total === 0)
                    return
                if (carriedPane.view.count <= pathTipTimer.row)
                    return
                pathTipTimer.stop()
                carriedPane.pointedTipRow = pathTipTimer.row
                pathTipReport.start()
                return
            }
            if (pathTipTimer.wipSide && unstagedModel.total === 0)
                return
            if (!pathTipTimer.wipSide && !driver.cardSettled)
                return
            pathTipTimer.stop()
            if (pathTipTimer.wipSide)
                wipPane.pointedTipRow = pathTipTimer.row
            else
                detailsPane.pointedTipRow = pathTipTimer.row
            pathTipReport.start()
        }
        /// Presses the copy's graph row once, so the row's own decision is taken (verify-ui §壊れない動詞の実装と反復).
        /// Answers whether the pane is standing on it.
        function standOnCopy() {
            if (pathTipTimer.stood)
                return true
            if (graphModel.finishCount === 0 || !workingTree.loaded)
                return false
            const row = driver.rowOfCopy(pathTipTimer.carried)
            if (row < 0)
                return false
            const item = graphPane.view.itemAtIndex(row)
            if (item === null)
                return false
            item.leftClick(Qt.NoModifier)
            pathTipTimer.stood = true
            return false
        }
    }
    SampleTimer {
        id: pathTipReport
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            pathTipReport.stop()
            Harness.report("path_tip pane="
                + (pathTipTimer.carried !== "" ? "carried"
                   : pathTipTimer.wipSide ? "wip" : "details")
                + " tree=" + (pathTipTimer.carried !== "" ? page.wipUnstaged.treeView
                              : pathTipTimer.wipSide ? unstagedModel.treeView : detailsModel.treeView)
                + " tip=" + tip.visible
                + " aside=" + driver.tipAside(tip)
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // A tooltip's words are a field (`CardText`): all selected and compared with the tip's text (scope: verbs.md
    // `tip-copy`). Asked of the contentItem as-is — a tip back on the style's `Text` has no `selectAll` and dies.
    SampleTimer {
        id: tipCopyTimer
        onTriggered: {
            if (unstagedModel.total === 0)
                return
            wipPane.pointedTipRow = 0
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            tipCopyTimer.stop()
            tip.contentItem.selectAll()
            Harness.report("tip_copy tip=" + tip.visible
                + " copied=" + (tip.contentItem.selected === tip.text)
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // ...and the same words grabbed from the tip's padding (規約 §hover のツールチップ), sampled over the air only
    // (`SweepPad.airPoints` — verbs.md `tip-sweep`). Reached off the tip's own parts as `tip-copy` is: a ground that
    // lost its pad dies here.
    SampleTimer {
        id: tipSweepTimer
        onTriggered: {
            if (unstagedModel.total === 0)
                return
            wipPane.pointedTipRow = 0
            const tip = page.ToolTip.toolTip
            if (!tip.visible || tip.width <= 0)
                return
            tipSweepTimer.stop()
            Harness.report("tip_sweep " + tip.background.pad.sweepAir(7))
            driver.complete()
        }
    }
    // Waits for the details. `cut=` leads, so must_say reads it as one run of words (verbs.md).
    SampleTimer {
        id: authorCardTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (Harness.autoAct === "author-card-open")
                detailsPane.messageBlock.showAuthor(true)
            if (Harness.autoAct === "author-card-open" && !detailsPane.authorCards.authorCardOpen)
                return
            authorCardTimer.stop()
            Harness.report(
                "author_card cut=" + detailsPane.messageBlock.nameClipped
                + " open=" + detailsPane.authorCards.authorCardOpen
                + " author=" + detailsPane.details.authorEmail
                + " committer=" + detailsPane.details.committerEmail
                + " other=" + detailsPane.details.committerDiffers
                + " later=" + detailsPane.details.commitTimeDiffers)
            driver.complete()
        }
    }
    // Waits for the details, which carry the credit line, and for the row to stop moving: a width asked for arrives a
    // frame after the row it resizes.
    SampleTimer {
        id: coAuthorTimer
        property string lastGeom: ""
        onTriggered: {
            if (!driver.cardSettled || detailsPane.width <= 0)
                return
            const geom = Math.round(detailsPane.width) + "/" + Math.round(detailsPane.valueRow.width)
                + "/" + detailsPane.messageBlock.matesFolded
            if (geom !== coAuthorTimer.lastGeom) {
                coAuthorTimer.lastGeom = geom
                return
            }
            if (Harness.autoAct === "co-authors-open")
                detailsPane.messageBlock.showCoAuthors(true)
            if (Harness.autoAct === "co-authors-open" && !detailsPane.authorCards.matesCardOpen)
                return
            coAuthorTimer.stop()
            // `open=` is the card's own visibility; the input that asked for it would go green with the binding cut.
            // `folded=` / `said=` are the credit as drawn: the name and the count of the rest, or folded, the face and
            // that same count (`-` for one, whose face stands alone).
            Harness.report(
                "co_authors count=" + detailsPane.messageBlock.coAuthorRecords.length
                + " cut=" + detailsPane.messageBlock.matesClipped
                + " first=" + detailsPane.messageBlock.coAuthorName(0)
                + " open=" + detailsPane.authorCards.matesCardOpen
                + " folded=" + detailsPane.messageBlock.matesFolded
                + " said=" + detailsPane.messageBlock.matesSaid
                // Unjudged: which step the date beside the credit came out at is the face's answer (`whole` on one
                // desk, `day` under a wider face at the same pane).
                + " date=" + detailsPane.messageBlock.valueRow.dateSaid
                + " pane=" + Math.round(detailsPane.width))
            driver.complete()
        }
    }
}
