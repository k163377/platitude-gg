---
name: platitude-claude-review
description: Review platitude-gg changes or investigations and prepare concise repair instructions for Claude.
---

# Claude へ渡すレビュー

レビュー観点の正本は [.claude/skills/review-points/SKILL.md](../../../.claude/skills/review-points/SKILL.md)。これを読み、対象の現在の SHA と、ユーザーの依頼・途中の訂正を照合する。

レビューのみの依頼は読み取りで進め、実装を含む依頼は共有規約に従って依頼範囲まで完了する。

## 指摘の根拠

- 指摘には、通常の操作で成立する条件と確認できる根拠を示し、事実・推論・未確認事項を区別する。
- 再レビューでは現在の実装を確認し、その時点で有効な指摘と修正方針を示す。

## 引き継ぐ内容

Claude 向けの修正指示書は、日本語の単一の plain text code block にする。ユーザーが形式を指定した時は、それに合わせる。

指示には指摘と修正方針に関係する内容のみ含める。

極端なエッジケースや意図的な破壊を要すると判断して除外した候補は、指示書とは別に、候補と除外理由を簡潔に報告する。

ユーザーによる指摘内容の確認・修正依頼を反映した結果は、指示書の全文で出力する。依頼に関係しない部分は、直前の指示書の文面をそのまま保持する。

指摘が無い時は、結論と確認範囲を簡潔に返す。
