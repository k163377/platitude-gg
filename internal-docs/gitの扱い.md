# git の扱い

**git に関する判断の正本(SSoT)。** 対応範囲(版)、異常時の扱いの基準と出し先、発行前の釘付け(穴塞ぎ)の一覧、
分類しない物、日常性能への代金、サポート外、判断待ちをここに集約する。
操作ごとの類型(git の答え・場面と頻度・出し先)と、各釘付けの手段・無いと何が起きるかは、その操作のモジュールの
コメントが持つ(置き場は §類型 と §発行前の釘付け の索引)。
1 つの呼び出し箇所の罠と実測の記録はその箇所のコメント、複数の呼び出し箇所に効く罠だけ
[rules-refs/core.md](../.claude/rules-refs/core.md)(1 項目 1 行)、画面の配置・文言は
[デザイン規約.md](デザイン規約.md)(§答えの要らない報せ / §git が言ったことを読む場所 /
§git が無い時・古い時)、検証の動詞は verify-ui スキル。コードの正本は `platitude-core` の
`report.rs`(`ReportKind` / `RemoteWhy` / `WriteReport`)と各操作のモジュール。
**操作をまたぐ判断を実装する時は先にここへ行を足し、1 つの操作に閉じる判断はその操作のコメントへ書く。**

## 定義と記録の置き場

**バージョン番号と選定理由はソース側で管理し、文書に転記しない。**
他の依存・実行環境の版も、それぞれのソース・設定の定義を参照する
([反映前テストの機械化.md §版を動かす変更は段 3](反映前テストの機械化.md))。

| 管理するもの | 正本 |
|---|---|
| 最低版・選定理由、発行するコマンド・オプションの互換性確認記録 | [version.rs の `MINIMUM_GIT` とそのコメント](../crates/platitude-core/src/version.rs) |
| 最低版で使えない形式・同梱レシピ | `version.rs` 末尾のコメント |
| 実際に発行する引数・環境変数 | 各操作の `GitCommand` 構築と `process/executor.rs` の共通設定(§発行前の釘付け) |
| 失敗の類型と出し先 | 出し先の基準は本文書 §異常時の扱いの基準、操作ごとの類型は各操作のモジュールのコメント(§類型 の索引)、kind は `report.rs` の `ReportKind` |
| git の文言から読む語と、全リリースでのその行(版の境目) | [`relied_messages.rs`](../crates/platitude-core/src/relied_messages.rs) と [`relied-messages.txt`](../crates/xtask/relied-messages.txt)(§git の文言に依存する判定) |
| 対応範囲・確認方法・追加時の手順・扱いの基準 | 本文書 |

UI・エラー文言は `version.rs` の `minimum_string()`、対応判定は `GitVersion::supported()` を使う。
コマンドの組み立ては、操作ごとの入力・設定・順序と一緒に管理する。
互換性の一覧を実行用の定数へ重ねて写すと、実装と一覧の両方を直す必要があるため、
定義側のコメントは**互換性の確認記録**に留める。

## 対応範囲

最低版は**動作を保証する下限**であり、起動を禁止する境界ではない。
下限未満の git でもアプリは起動し、古いことを警告した上で操作を実行する。
その git が対応する操作は動き、非対応のコマンド・オプションは git 自身のエラーとして報告する。
下限未満の全機能を保証したり、その版向けの代替コマンドを揃えたりはしない。

git が見つからず実行できない場合は、git を導入するまでリポジトリを操作できない。
設定した実行ファイルが起動時に答えなければ、`PATH` の git へフォールバックする
(`Hub::resolve_git`)。設定画面では版が返ったパスだけを再起動の候補にでき、下限未満も候補に含める。

Git-LFS とマージツール本体は別途導入する物で、git の最低版だけでは存在を保証しない。
submodule と LFS は存在表示まで([実装計画.md](実装計画.md) §14 Post-MVPバックログ)。

## 互換性の確認

定義のコメントには、発行するサブコマンド・オプション・出力形式に加え、
固定引数、設定キー、環境変数、pathspec magic の確認結果も残す。
対象は製品側の `GitCommand` 構築と executor。開発補助コマンドやテスト用の git 操作は別の範囲。

マニュアル照合は「その形式が存在する」確認であり、引数の組合せや実際の挙動まで保証しない。
曖昧な組合せや拒否条件は実 git の統合テストに固定し、最低版を積んだ
[Linux コンテナ](../ci/linux/Dockerfile) でも確認する。
開発機や同じ OS 名の CI runner が積む git は最低版とは限らないため、代用したことにしない。
実バイナリでの全体スモークの残件は [P5-確認事項.md §6](P5-確認事項.md)。

### 新オプション追加時の手順

1. `MINIMUM_GIT` を読み、`https://git-scm.com/docs/git-<cmd>/<最低版>.0` の
   バージョン付きマニュアルで記載を確認する。redirect 先が最低版より新しければ根拠にせず、
   対応する git.git の `v<最低版>.0` タグの `Documentation` を読む
   (`https://raw.githubusercontent.com/git/git/v<版>/Documentation/...`)。
   古い版へ移る場合も、最低版での廃止・挙動変更が無いことを確認する。
2. 個別マニュアルに無い `--no-` 形は gitcli(7) §Negating options も照合する。
   一般規定だけでは通らないコマンドもあるため、曖昧なら最低版の実バイナリで試す。
3. `MINIMUM_GIT` のコメントへ形式と根拠を追記する。使えない形式はコード末尾のコメントへ残す。
   引数の組合せ・戻り値・拒否条件に依存する変更は、実 git の統合テストも追加・更新する。
4. 最低版そのものを変える場合は、同じ定義の選定理由・確認記録とコンテナの整合を見直す。
   版変更の検証範囲は [反映前テストの機械化.md](反映前テストの機械化.md) に従う。

### git の文言に依存する判定

**git も Git for Windows も、文言を版・OS で揃える約束をしていない。** git(1) は最低版から `master` まで、
Porcelain の入出力を「利用者の体験のために変わりうる」とする(安定を目指すのは plumbing だけ)。保守版も文言を
変え(下の記録の `INDEX_UNWRITTEN`)、Git for Windows は独自の文を持つ(同 `GPG_UNSIGNED`)。**だから最低版と
最新版の両端だけでは裏取りにならない。**

| 物 | 置き場 | 役 |
|---|---|---|
| 語の表 | [`relied_messages.rs`](../crates/platitude-core/src/relied_messages.rs) | 製品が git の文言から読む物の全部: 分類器が判定に使う語と、git の文から切り出して画面の文に埋める区切り(worktree のロック理由)。**2 種類で書く** —— **形**(`Words`)は 1 つの行の文で、出所(`- git:` / `- openssh:`)と実 git にそれを言わせるテスト(`- witness:`)を持つ。照合・記録・週次はこの単位。**メッセージ**(`Relied`)は同じことを言う形の束で、どれかが見つかれば当たり(書き方はそのファイルの冒頭コメント)。**分類器は表の判定を呼び、自前の文字列を書かない**(機械化済み: `cargo xtask structure` —— 見る範囲は `structure::relied_messages` の冒頭コメント。git の文でない物 = 機械形式の答え・アプリ自身の印は、その行か直前のコメントに `// not git's words: <何か>`) |
| 版の差の吸収 | 同上 | **動いている git の版で分岐しない**(ユーザー決定 2026-10-13)。版ごとの形はすべてメッセージの束に並べ、どれかに当たれば良い。版の番号は文言の鍵にならない —— 同じ番号でも Git for Windows(2.43〜2.46 の gpg の文)・Apple Git(本家と 73 ファイル違う 2.50.1)・ディストリの取り込み(Ubuntu の 2.43.0 に 2.43.4 の dubious ownership)で文が違い、撃つ git も設定と PATH への切り替えで変わる。**分ける時は、同じ形が版で別の意味になった時だけ、印は git が言ったこと(文・記録)** |
| 照合の記録 | [`relied-messages.txt`](../crates/xtask/relied-messages.txt)(生成物) | 読んだ木と、各語の行を「同じ行を持つ木」ごとに分けた時代 = 版の境目。読む木(最低版以降の本家の全リリースと `master`、Git for Windows、Apple Git、[最低版コンテナ](../ci/linux/Dockerfile)の Ubuntu、OpenSSH)と、行として数える物は `xtask::relied_messages` のコメント |
| 足す・変える時の網 | gate の常時ステップ <!--call:relied-messages.check-->`relied-messages check`(オフライン) | 表と記録が食い違えば赤 —— <!--cmd:relied-messages.audit-->`cargo xtask relied-messages audit`(全木を取り直して記録を書き直す)を撃って commit するまで通らない。証人の完全名は、統合スイートの `relied_witnesses` がそのテストバイナリの `--list` と突き合わせる(表を変えると gate がそのテストを選ぶ) |
| 週次 | [`relied-messages.yml`](../.github/workflows/relied-messages.yml) | `watch`(各系列の先端と記録に無いリリースを記録と比べ、**語の行が変わった時だけ赤**)と `master`(本家 `master` を組んで表の証人を回し、行に出ない経路の変化を証人の在る場面で捕まえる)。中身は `xtask::relied_messages` のコメント |

語を足す・変える時:

1. 使い捨てリポジトリで再現して `tests/it` に固定する(rules-refs/core.md「書き込みの失敗の分類を足す・変える前に」)。
   そのテストを表の `- witness:` に名指す
2. 表に形を足す: 出所の git のファイルと、その行の文(大小は問わない)。**同じ語を書く所が多い語(`already exists` 等)は、
   自分の場面の文を出すファイルだけを名指す** —— 木全体で拾うと、無関係な行の変化で週次が赤になる。同じことを言う
   別の形(別の版・別の経路の文)が在れば、その形も足してメッセージの束に並べる
3. <!--cmd:relied-messages.audit-->`cargo xtask relied-messages audit` を撃ち、`relied-messages.txt` の差分でその語の行の時代を読む。時代が 2 つ以上ある語は、
   変わった版でも分類が正しいかを確かめ、終了コード・stream が変わる境目は分類器のコメントに書く
4. commit する(gate の <!--call:relied-messages.check-->`relied-messages check` が表と記録の一致を見る)

照合が見ない物:

- **語の行は変わらず、同じ場面で先に別の文を言うようになった変化のうち、証人の無い場面** —— 週次の `master` の
  テストが捕まえるのは証人の在る場面だけ
- **git の外の文**: C ライブラリの `strerror`(`File exists`)、forge が名付ける拒否の理由(`protected branch hook
  declined`)
- **表の外**: 機械形式(`--porcelain` / `-z` / `--format` / trace2)、生のまま出すと決めた git の文(コマンドログ・
  ヘッダの 1 行・開けなかった画面)、データとして読む人向けの出力(mergetool の一覧・reflog の件名・stash の名前・
  `git version`)。どれも管理の範囲外(ユーザー決定 2026-10-13)
- **git の文を写した固定の UI 文言**(`Another git process seems to be running` 等)は追従しない(ユーザー決定
  2026-10-13)。写した所のコメントに、どの版のどのファイルの文かを書く

## 異常時の扱いの基準(ユーザー決定 2026-10-12)

失敗の扱いは 0 から作る想定で、次の基準で決める。**「対処」には、PGG の UI 操作からそれが起きないよう
穴を塞ぐ形を含む。**

1. **簡単・単純に判定できる物は分岐を作り込む**(固定文字列・終了コード・ref 行の読み。1 arm + 文 1 つ)。
2. **多少複雑でも作り込む条件**(いずれか): **PGG の UI から容易に再現できる** / **一般的・有名なリポジトリが
   ローカルで使う機能が生む**(フック = husky・lefthook・pre-commit・git-lfs、コミット署名、worktree、
   IDE の裏で走る git の index ロック)/ **有名 forge の一般的な設定・機能が生む**(GitHub・GitLab の
   保護ブランチ・push rules・サイズ上限・required status)。
3. **手動の破壊や稀な構成でしか生じない物に高コストな対処をしない** —— 知らせ(報せバー)+ `GIT COMMANDS`
   (ログに git の全文)で止める。分類の文字列表に 1 行足すのは低コスト、再現に外部プログラム(gpg / ssh)
   の設営が要る統合テスト・専用の preset・専用の読みは高コスト。
4. **発行前の段階・低コストな手段(釘付け・ゲート)で起きなくできる物は積極的にそうし、その失敗を
   「稀」に分類する**(§発行前の釘付け)。
5. **日常性能への悪影響が大きい対処は、意図的にサポート外にすることも検討する**(§日常性能への代金)。
6. **「現状が正しい」とは思い込まない** —— 判定はプラットフォームの公式ドキュメント(git のマニュアル、
   GitHub / GitLab の docs)、git の全リリースのソース(§git の文言に依存する判定)と実測で裏を取る。使い捨てリポジトリで再現し、
   `tests/it` に固定してから分類器を書く(rules-refs/core.md「書き込みの失敗の分類を足す・変える前に」)。

### 出し先は 4 つ

| 出し先 | 何か | 条件 |
|---|---|---|
| **起きない**(釘付け) | 発行する引数・環境・読み方で、利用者の設定・環境が結果を変えないようにする | 低コストに固定できる物は全部(§発行前の釘付け) |
| **ゲート** | 押す前に断る・行を出さない・灰にする・箱の枠で止める | **アプリが押す前に知り得る**失敗(名前の規則・衝突・立っている操作・別 worktree の保持・数の比較・identity・空の選択)。git に断らせて報せるのは往復を 1 つ増やすだけ |
| **報せ**(`GitError::Reported` / `Withheld` = `ReportKind`) | 2 行の報せバー。見出し「何が起きなかったか」+ 2 行目「何が断ったか + `see GIT COMMANDS`」。ログは開かない | **走らせるまで分からず、こちらに直す手が無く、中途半端に残っていない**(外のフック・署名・他の git のロック・向こうの規則・こちらが古いリモート)か、**こちらが git を撃つ前に断った**物(指紋の不一致・書き換えの形) |
| **ログ**(`GitError::Failed` 等) | GIT COMMANDS が自分で開き、ヘッダに 1 行、行の下に git の全文 | 残り全部: 通信・認証、分類しない稀な失敗、読みの失敗。2 行目の文を考える代金を払わない側。**ヘッダの 1 行だけは、届かなかった理由が資格情報・SSH の鍵・host key ならアプリの 1 節**(`remote::unreached`)。**例外は auto fetch(タイマー・開いた時)の失敗**: スリープ・オフラインで日常的に起きるのでログは開かず、fetch ボタンの `!` だけ(ユーザー決定 2026-10-12) |

- **報せが名指す原因は git の文か git 自身の記録(trace2)が言う物だけ**。推さない(フックの在処・ロックファイルの
  在処は断った理由の証拠ではない)。名指せなければ `git turned it down — see GIT COMMANDS`。
- **終了コードで答えるコマンド**(`answers_by_code`)の同じコードが断りでもある時は、断りの語を
  `except_when_it_says` / `except_when` に名指して行に言葉を持たせる(rebase の exit 1・`stash apply` の exit 1・
  **merge / pull の早送りを塞ぐ作業ツリー**の exit 1)。言葉の無い答えの行を報せやヘッダが指すと、読む人は
  何も見つけられない。

## 発行前の釘付け(起きなくした物)

利用者の設定・環境が読みの形や書き込みの結果を変えないよう、発行時に固定している物。
**釘付けは全部、最低版のマニュアルかソースで形式を確かめた物**(`version.rs` の確認記録)。

| 固定する物 | 置き場(手段と、無いと何が起きるかはそのコメント) |
|---|---|
| 色・パスの引用・署名表示・コミット文の文字コード(ユーザー決定 2026-10-12)・index の自動 refresh・任意ロック | `process::executor` `FIXED_ARGS` |
| ロケール・資格情報の対話・任意ロック・エディタ | `FIXED_ENV` |
| 起動元から継いだリポジトリの所在・ref の名前空間・reflog の文・diff と pathspec の形(毎 spawn で消す) | `INHERITED_ENV_DROPPED` |
| diff の形(prefix・文脈行・submodule の表示・外部 diff) | `details::DIFF_SHAPE_ARGS` |
| push の送る ref(明示の refspec + `--no-follow-tags`) | `remote::push::porcelain_push` |
| rebase・merge・pull の autostash、駆動 rebase の merge 形 | `integrate::rebase_command` / `integrate::merge` / `remote::pull`(退け方は `session::build::carry_across`) |
| pull の合流の仕方(`--ff-only --no-rebase`。ユーザー決定 2026-10-12) | `remote::pull` |
| merge / pull を塞ぐ作業ツリーの変更・untracked(断られた時だけ持ち越す。ユーザー決定 2026-10-12) | `session::build::integrate_carrying` |
| stash の index 復元(`--no-index`) | `stash::pop` / `apply` / `carried_apply` |
| mergetool の一時ファイル・ツール・GUI | `conflict::tool` |
| 一時 index の写し(フック・fsmonitor・split index・改行検査) | `discards::record::isolated` |
| worktree add の進捗(`--quiet`) | `worktrees::add` |
| 部分破棄の改行(`core.autocrlf`) | `stage::endings` |
| 部分操作の mode(ユーザー決定。デザイン規約 §mode の変更) | `patch::header::fitted` |
| pathspec の魔法(`:(literal)`。`GIT_LITERAL_PATHSPECS` は使わない) | `process::literal_pathspec` |

### 釘付けない物(意図して利用者の設定に従う)

| 設定 | 従う理由 |
|---|---|
| `commit.gpgsign` / `tag.gpgSign` / `gpg.format` | 署名は利用者の約束(forge の required signatures)。署名できない時は報せ `CommitUnsigned` |
| `branch.<n>.merge` / `branch.<n>.remote` | pull がどこへ取りに行くかは利用者の設定(合流の仕方だけ釘付け = 上の表) |
| `rebase.updateRefs` | 駆動形は `--update-refs` を自分で足す。枝のまとまりの読みは `session::discard_record` の `record_rebase_group` / `stopped_rebase_tips` |
| `gc.auto` / `maintenance.auto` | リポジトリの保守は利用者の物。後ろで走る gc が取るロックは `*Locked` の報せで受ける |
| `core.fsmonitor` / `core.untrackedCache` | 外すと書き込みが遅くなり index の印が消える。trace2 の受け口はデーモンが繋いでも閉じるだけ(`process::trace_sink` のコメント) |

## 類型(出し先と判定)

操作ごとの類型 —— git の答え(`LC_ALL=C`)・起きる場面と頻度の判定(§異常時の扱いの基準 のどれか)・出し先 —— は、
その操作を分類するモジュールのコメントが持つ。kind ごとの意味は `report.rs` の `ReportKind`、2 行目の文そのものは
`Words.qml` の `writeReportedWhy`、文字列・終了コードの根拠(版・ソース・再現)は語の表と、その文を読む分類器のコメント
(`report::lock_taken`・`remote::refusal`・`commit::refused_by` 等。複数の分類器に効く罠だけ rules-refs/core.md)。
**類型を足す・変える時は、分類器のコメントに場面・頻度と出し先を書く。**

| 操作 | 置き場 |
|---|---|
| push(ブランチ・タグ・リモートの削除・差し替え) | `remote::refusal` の冒頭(届かない時のヘッダの 1 節は `remote::unreached`) |
| commit・amend | `commit::refused` / `refused_by` |
| 履歴の書き換え(squash / drop / reword・プラン) | 撃つ前に断る 7 種は `ReportKind::RewriteAcrossMerge` 〜 `RewriteWhileStanding`、`pre-rebase` は `integrate::rebase`、置き直すコミットの断りは `sequencer::run::refused_by`、`rebase.missingCommitsCheck` は `sequencer::todo` のテスト |
| 改名(ブランチ / タグ / stash)・worktree のフォルダ名 | `ReportKind::RenameRefused` 〜 `StashGone`、`WorktreeNotRenamed` / `WorktreeRenameTaken` |
| 部分ステージ・アンステージ・破棄 | `stage::refusal` と `ReportKind::StaleStage` 〜 `DiscardLocked`、LFS・gitlink の部分を出さないのは app の `encode::whole_only_why`、改行が混ざったファイルは `stage::endings::kept` |
| worktree(add / remove)・別の worktree の変更の適用 | `worktrees::add` / `report::worktree_not_removed`、`session::carried_apply` / `report::carried_refused` |
| merge・pull・switch・cherry-pick・revert | `integrate::merge` / `refused_over_work`、`remote::pull`、`session::build::integrate_carrying` / `carry_across` |
| 読み・開く・git そのもの | `repo::open`、`GitError::UntrustedOwner` / `ConfigUnreadable`、`version`、下の §設定の書き換えと重なった読み |

### 設定の書き換えと重なった読み

git は設定ファイルを `<file>.lock` に書いて rename で置き換える。**Windows ではその間に起動した git が
ファイルを開けない** —— リポジトリのコマンドは fatal で落ち、`git config --get*` は読めなかったファイルを飛ばして
答える(そこにだけ在るキーは「未設定」に読まれる)。mac / Linux では起きない。

- **撃ち直すのは読みだけ・1 回だけ**(`process::config_unread`。ユーザー決定 2026-10-13)。書き込みは書いた後で
  落ちうるので撃ち直さない。読みの判定・警告だけの形・流し読み・文言の根拠と、2 回目も落ちた時の扱いは
  `process::config_unread` のコメント
- **効き目**(git 2.55、グローバル設定を書き換え続ける横で読む): 開きの `rev-parse` と `config --get-regexp` を
  計 1,200 回 —— 撃ち直し無しで `rev-parse` が 13 回落ち、撃ち直し有りで 0 回(撃ち直し 18 回)

## 分類しない物(ログに落とす)と理由

| 物 | 理由 |
|---|---|
| 署名 push(`push.gpgSign`)の 2 類型 | 稀な構成で主要 forge が受けない。再現に `receive.certNonceSeed` と鍵の設営が要る(基準 3) |
| 半分済んだ改名の「何が止めたか」(ロック / 断り / 一覧が動いた) | 2 コマンドの隙間の競合は稀。1 つの報せとログで足りる(基準 3) |
| `[rejected] (needs force)` 等 | 稀。理由は stdout の ref 行がログに持つ |
| 届かない通信(名前解決・経路・タイムアウト) | git の 1 行目で足りる。資格情報・鍵・host key の 3 つだけはヘッダの 1 節(§出し先は 4 つ の「ログ」の行) |
| `core.safecrlf` の拒否・`checkout-index` が置けない untracked・大小を区別しないディスク越し | 稀・据え置き(変更を持ってくる時の 2 つは `session::carried_apply` の `apply_work` のコメント) |
| リポジトリの破損・ディスク満杯・メモリ不足 | 範囲外 |

## 日常性能への代金(異常時の扱いが日常に払う物)

実測は [code-costs-windows-x64.md](../ci/baseline/code-costs-windows-x64.md) に 1 行ずつ。

| 仕組み | 代金 | 判定 |
|---|---|---|
| **trace2 の記録**(`GitCommand::traced` = commit / push / 駆動 rebase / worktree add。フック・ssh 署名プログラムの終わりを git 自身の記録で読む) | **Windows は git プロセス 1 本につき約 +19ms**(git が `cmd_ancestry` のために全プロセスを数える。止める設定・環境変数は無い —— 仕組みは `GitCommand::traced` のコメント)。commit・push の増分は code-costs の行。Linux は `/proc` 読みで未実測 | **維持**(ユーザー決定 2026-10-12)。退けた案(終了コードで推す)と他の GUI の扱いは `GitCommand::traced` のコメント |
| 部分ステージ前の指紋の照合(diff を 1 本読み直す) | press ごとに diff 1 本 | 維持(読んだ行と違う物を当てない保証) |
| リモート削除前の lease(`ls-remote`) | press ごとに往復 1 つ | 維持(画面の先端への lease = 向こうの別の commit を消さない) |
| 他の worktree の読み(carried) | worktree 数に比例する `status` | ペースで薄める(`session::pace`。残る代金は P3-確認事項 §別の worktree の未コミット行) |
| `diff.autoRefreshIndex=false`(ポーリングが index を書かない) | 利用者の `git status` が racily-clean を再計算 | 維持(commit を落とさない) |

## サポート外化の候補(日常性能を理由に)

| 候補 | 状況 | 落とすと | 判定 |
|---|---|---|---|
| trace2 でフックの終わりを読む(4 つの書き込み) | §日常性能への代金の行(Windows は git プロセス 1 本ごと) | `CommitHook` / `PrePush` / `RewriteCommitHook` / `WorktreeHookFailed` と ssh 署名プログラムの失敗を名指せず `git turned it down` へ。commit は exit 1 + stdout 空を git の答えとして読めるが、pre-push は ref 行の無い exit 1 が通信の失敗と区別できない | **維持**(ユーザー決定 2026-10-12)。再検討するなら落とすのは commit の trace だけ |

## 判断待ち(要判断)

状況と、基準に照らした選択肢。決まったら上の表か、その操作のモジュールのコメントへ写してここから消す。

- **別のリポジトリの作業ツリーの中に、HEAD の空いた `.git` フォルダを持つフォルダがあると、そこを開くと外側のリポジトリが
  黙って開く**(実測 2.55: `rev-parse --show-toplevel` が exit 0 で外側の頂上を答える。git は受け取れない `.git` フォルダを
  素通りして親へ辿る —— setup.c の discovery、全版。`.git` ファイルなら git がそこで止まるので起きない)。稀(入れ子の
  リポジトリが落ちた時に壊れた)。選択肢: (a) 今のまま / (b) 開けた時に、開いたフォルダ自身が `.git` を持つのに git の
  答えた頂上がそのフォルダでなければ開かずに「git が答えられなかった」へ(`repo::open` の成功側の比較 1 回、git は撃ち足さない。
  ただし出す git の文が無いので、その画面の 3 行目を決める要がある)
