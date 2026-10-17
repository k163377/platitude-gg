---
name: platitude-seat
description: Claim, resume, or release a platitude-gg worktree seat from Codex with the current task identity and Git metadata permissions.
---

# Codex の席取得

席の保持・返却・乗っ取り・land の権限は [CLAUDE.md の「ビルド・テスト」「Git 運用」](../../../CLAUDE.md)、会話をまたぐ保持と作業状態は [AGENTS.md](../../../AGENTS.md) に従う。

## 席の取得と継続

この会話が席を保持している間は、その席で作業を続ける。割り当てが必要な時は次の手順で取得する。

1. 実行環境の `CODEX_THREAD_ID` が現在のタスク ID であることを確認する。`CLAUDE_CODE_SESSION_ID`・`CLAUDE_PID`・`CLAUDECODE` は未設定の状態を使う。
2. `git rev-parse --path-format=absolute --git-common-dir` で共有 Git 管理領域を確認し、書き込める権限で取得コマンドを実行する。管理領域が読み取り専用の sandbox では、最初から `require_escalated` を使う。
3. <!--cmd:seat.take-->`cargo xtask seat` で割り当てを受け、席の文字・ブランチ・絶対パスを報告する。以後の読み書き・検査は、返された worktree で行う。

席の選択・所有権・他の会話との競合は、取得コマンドが判定する。

割り当てを得られなかった場合は、コマンドの結果と作業を妨げる条件をユーザーへ報告する。

## 返却

ユーザーが返却を指示した時は、同じタスク ID と書き込み権限で <!--cmd:seat.release-->`cargo xtask seat release` を実行する。解除結果と worktree のロック状態を照合してから報告する。land は共有の手順で返却する。
