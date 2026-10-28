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
    readonly property var worktree: driver.worktree
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
                arg === "older" ? graphModel.rowOf(worktree.headOid) + 1 : Number(arg)))
            stashTipTimer.start()
        } else if (act === "path-tip") {
            // Row 0 is the elided leaf, or the folder chain with `-tree`. The argument picks the pane as `corner`
            // does, plus `carried:<worktree>`, named because its rows share the all-zero id (`driver.rowOfWorktree`).
            // A worktree whose folder ends in `-tree` would be misread; the presets hold none. `@<row>` points at a
            // later row, one with rows on both sides.
            const rowAt = ("" + arg).indexOf("@")
            pathTipTimer.row = rowAt >= 0 ? Number(("" + arg).slice(rowAt + 1)) : 0
            const named = rowAt >= 0 ? ("" + arg).slice(0, rowAt) : "" + arg
            const wantsTree = named.endsWith("-tree")
            const pane = wantsTree ? named.slice(0, -5) : named
            pathTipTimer.carried = pane.startsWith("carried:") ? pane.slice(8) : ""
            pathTipTimer.wipSide = pane === "" || pane === "wip"
            pathTipTimer.order = pathTipTimer.carried !== "" ? ["stood", "pointed"] : ["pointed"]
            if (pathTipTimer.carried !== "") {
                // The worktree is stood on by the timer's first stage; its list does not exist before that.
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
            authorCardTimer.order = act === "author-card-open" ? ["shown"] : []
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
            coAuthorTimer.order = act === "co-authors-open" ? ["shown"] : []
            coAuthorTimer.start()
        } else {
            return false
        }
        return true
    }
    // Waits for gpg / ssh-keygen's verdict, which decides the mark.
    PageStage {
        id: signatureTimer
        name: "signature"
        onTriggered: {
            if (!Awaited.all("signature_ready", { "ready": page.signatureIsForSelection }))
                return
            signatureTimer.complete(() => "signature kind=" + page.selectedSignatureKind
                                           + " code=" + page.selectedSignatureCode
                                           + " signer=" + page.selectedSignatureSigner)
        }
    }
    // signature-tip / stash-tip: pointed at only once the state has landed, reported once the tip is shown.
    PageStage {
        id: signatureTipTimer
        name: "signature_tip"
        order: ["pointed"]
        onTriggered: {
            if (signatureTipTimer.at("start")) {
                if (!Awaited.all("signature_tip_ready", { "signature": page.signatureIsForSelection }))
                    return
                signatureTipTimer.input("pointed", () => {
                    detailsPane.signaturePointedAt = true
                })
                return
            }
            if (!Awaited.all("signature_tip_report_ready", { "ready": detailsPane.signatureTipShown }))
                return
            signatureTipTimer.complete(() => "signature_tip code=" + page.selectedSignatureCode
                                              + " tip=" + detailsPane.signatureTipShown)
        }
    }
    PageStage {
        id: stashTipTimer
        name: "stash_tip"
        order: ["pointed"]
        onTriggered: {
            if (stashTipTimer.at("start")) {
                if (!Awaited.all("stash_tip_ready", { "card": driver.cardSettled }))
                    return
                stashTipTimer.input("pointed", () => {
                    detailsPane.summaryPointedAt = true
                })
                return
            }
            if (!Awaited.all("stash_tip_report_ready", { "ready": detailsPane.summaryTipShown }))
                return
            stashTipTimer.complete(() => "stash_tip blocked=" + (detailsPane.editBlocked !== "")
                                          + " tip=" + detailsPane.summaryTipShown + " why=" + detailsPane.editBlocked)
        }
    }
    // path-tip: pointed at once the list has landed. The report reads the shared tooltip itself, the one thing that
    // also says its words.
    PageStage {
        id: pathTipTimer
        name: "path_tip"
        property bool wipSide: true
        /// The worktree this run is pointing into, empty for the two panes of this window's own tree.
        property string carried: ""
        /// The row pointed at.
        property int row: 0
        onTriggered: {
            const op = pathTipTimer
            if (op.carried !== "" && op.at("start")) {
                // Presses the worktree's graph row once, so the row's own decision is taken (verify-ui
                // §壊れない動詞の実装と反復).
                const row = driver.rowOfWorktree(op.carried)
                const item = row < 0 ? null : graphPane.view.itemAtIndex(row)
                if (!Awaited.all("path_tip_worktree", { "graph": graphModel.finishCount !== 0,
                                                        "loaded": worktree.loaded,
                                                        "row": row >= 0,
                                                        "built": item !== null }))
                    return
                op.input("stood", () => {
                    item.leftClick(Qt.NoModifier)
                })
                return
            }
            if (!op.at("pointed")) {
                if (op.carried !== "") {
                    // Until the worktree's own files arrive the pane holds this window's.
                    const files = page.wipUnstaged
                    if (!Awaited.all("path_tip_carried", { "path": page.carriedPath !== "",
                                                           "files": files.carriedAt === page.carriedPath,
                                                           "total": files.total !== 0,
                                                           "rows": carriedPane.view.count > op.row }))
                        return
                    op.input("pointed", () => {
                        carriedPane.pointedTipRow = op.row
                    })
                    return
                }
                if (!Awaited.all("path_tip_list", { "files": !op.wipSide || unstagedModel.total !== 0,
                                                    "card": op.wipSide || driver.cardSettled }))
                    return
                op.input("pointed", () => {
                    if (op.wipSide)
                        wipPane.pointedTipRow = op.row
                    else
                        detailsPane.pointedTipRow = op.row
                })
                return
            }
            if (!Awaited.all("path_tip_report_ready", { "ready": Tips.box.visible }))
                return
            op.complete(() => {
                const tip = Tips.box
                return "path_tip pane="
                       + (op.carried !== "" ? "carried"
                          : op.wipSide ? "wip" : "details")
                       + " tree=" + (op.carried !== "" ? page.wipUnstaged.treeView
                                     : op.wipSide ? unstagedModel.treeView : detailsModel.treeView)
                       + " tip=" + tip.visible
                       + " aside=" + driver.tipAside(tip)
                       + " text=" + tip.text
            })
        }
    }
    // A tooltip's words are a field (`CardText`): all selected and compared with the tip's text (scope: verbs.md
    // `tip-copy`). Asked of the contentItem as-is — a tip back on the style's `Text` has no `selectAll` and dies.
    PageStage {
        id: tipCopyTimer
        name: "tip_copy"
        order: ["pointed", "selected"]
        onTriggered: {
            const op = tipCopyTimer
            if (!Awaited.all("tip_copy_ready", { "ready": !(unstagedModel.total === 0) }))
                return
            if (op.at("start")) {
                op.input("pointed", () => {
                    wipPane.pointedTipRow = 0
                })
            }
            if (!Awaited.all("tip_copy_shown", { "tip": Tips.box.visible }))
                return
            op.input("selected", () => {
                Tips.box.contentItem.selectAll()
            })
            op.complete(() => {
                const tip = Tips.box
                return "tip_copy tip=" + tip.visible
                       + " copied=" + (tip.contentItem.selected === tip.text)
                       + " text=" + tip.text
            })
        }
    }
    // ...and the same words grabbed from the tip's padding (規約 §hover のツールチップ), sampled over the air only
    // (`SweepPad.airPoints` — verbs.md `tip-sweep`). Reached off the tip's own parts as `tip-copy` is: a ground that
    // lost its pad dies here.
    PageStage {
        id: tipSweepTimer
        name: "tip_sweep"
        order: ["pointed", "swept"]
        /// What the sweep answered, read as it went over the air.
        property string answer: ""
        onTriggered: {
            const op = tipSweepTimer
            if (!Awaited.all("tip_sweep_ready", { "ready": !(unstagedModel.total === 0) }))
                return
            if (op.at("start")) {
                op.input("pointed", () => {
                    wipPane.pointedTipRow = 0
                })
            }
            if (!Awaited.all("tip_sweep_shown", { "tip": Tips.box.visible, "wide": Tips.box.width > 0 }))
                return
            op.input("swept", () => {
                op.answer = Tips.box.background.pad.sweepAir(7)
            })
            op.complete(() => "tip_sweep " + op.answer)
        }
    }
    // Waits for the details. `cut=` leads, so must_say reads it as one run of words (verbs.md).
    PageStage {
        id: authorCardTimer
        name: "author_card"
        onTriggered: {
            const op = authorCardTimer
            if (!Awaited.all("author_card_ready", { "ready": driver.cardSettled }))
                return
            const opens = Harness.autoAct === "author-card-open"
            if (opens && op.at("start")) {
                op.input("shown", () => {
                    detailsPane.messageBlock.showAuthor(true)
                })
            }
            if (!Awaited.all("author_card_open", { "open": !opens || detailsPane.authorCards.authorCardOpen }))
                return
            op.complete(() => "author_card cut=" + detailsPane.messageBlock.nameClipped
                              + " open=" + detailsPane.authorCards.authorCardOpen
                              + " author=" + detailsPane.details.authorEmail
                              + " committer=" + detailsPane.details.committerEmail
                              + " other=" + detailsPane.details.committerDiffers
                              + " later=" + detailsPane.details.commitTimeDiffers)
        }
    }
    // Waits for the details, which carry the credit line, and for the row to stop moving: a width asked for arrives a
    // frame after the row it resizes.
    PageStage {
        id: coAuthorTimer
        name: "co_author"
        onTriggered: {
            const op = coAuthorTimer
            if (!Awaited.all("co_author_ready", { "ready": !(!driver.cardSettled || detailsPane.width <= 0) }))
                return
            const geom = Math.round(detailsPane.width) + "/" + Math.round(detailsPane.valueRow.width)
                + "/" + detailsPane.messageBlock.matesFolded
            if (!op.still("co_author_geom", geom))
                return
            const opens = Harness.autoAct === "co-authors-open"
            if (opens && op.at("start")) {
                op.input("shown", () => {
                    detailsPane.messageBlock.showCoAuthors(true)
                })
            }
            if (!Awaited.all("co_author_open", { "open": !opens || detailsPane.authorCards.matesCardOpen }))
                return
            // `open=` is the card's own visibility; the input that asked for it would go green with the binding cut.
            // `folded=` / `said=` are the credit as drawn: the name and the count of the rest, or folded, the face and
            // that same count (`-` for one, whose face stands alone).
            op.complete(() => "co_authors count=" + detailsPane.messageBlock.coAuthorRecords.length
                              + " cut=" + detailsPane.messageBlock.matesClipped
                              + " first=" + detailsPane.messageBlock.coAuthorName(0)
                              + " open=" + detailsPane.authorCards.matesCardOpen
                              + " folded=" + detailsPane.messageBlock.matesFolded
                              + " said=" + detailsPane.messageBlock.matesSaid
                              // Unjudged: which step the date beside the credit came out at is the face's answer
                              // (`whole` on one desk, `day` under a wider face at the same pane).
                              + " date=" + detailsPane.messageBlock.valueRow.dateSaid
                              + " pane=" + Math.round(detailsPane.width))
        }
    }
}
