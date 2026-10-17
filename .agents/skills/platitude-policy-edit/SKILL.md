---
name: platitude-policy-edit
description: Edit platitude-gg agent rules, skills, and harness guidance while preserving their scope and sources of truth.
---

# 規約とハーネス文書の編集

配置と追加の条件は [CLAUDE.md の「規約の置き場所と本ファイルの運用」](../../../CLAUDE.md)、レビュー観点は [.claude/skills/review-points/SKILL.md](../../../.claude/skills/review-points/SKILL.md) が正本。今回の依頼に必要な変更を、その正本と参照側へ揃える。

- 依頼の語句に対応する正本と参照側を探し、既存の項目の編集・集約・リンクで依頼を満たす。
- 指示の対象・条件・例外を保ち、差分を依頼に必要な意味の変更に絞る。
- 規則は望ましい行動・成果・完了条件を直接書き、権限と適用範囲の境界を明確にする。
- 常時読む文書には判断に必要な不変条件と参照先を置く。制約・罠・決定事項は現在形で書き、分析記録は役割別の資料へまとめる。
- 値・コマンド・生成物は既存の管理方式で更新する。

変更後は正本と直接の参照側を読み、現在の実装・依頼と文書の意味が一致することを確認する。差分全体を、必要な変更だけで目的を満たす形に整える。

文書検査は <!--cmd:docs.check-->`cargo xtask docs`。生成 span は既存の同期手順で更新し、差分を確認する。新規・変更した Codex skill は `skill-creator` の validator と、代表的な依頼に対する適用範囲・行動・出力の照合で確認する。検査結果と規則の意味をそれぞれ確認し、gate の時機は共有の gate skill に従う。
