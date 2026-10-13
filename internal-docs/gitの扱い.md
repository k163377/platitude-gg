# git の扱い

**git に関する判断の正本(SSoT)。** 対応範囲(版)、異常時の扱いの基準と類型、発行前の釘付け(穴塞ぎ)、
分類しない物、日常性能への代金、サポート外、判断待ちをここに集約する。
各論の罠と実測の記録は [rules-refs/core.md](../.claude/rules-refs/core.md)(1 項目 1 行)、
画面の配置・文言は [デザイン規約.md](デザイン規約.md)(§答えの要らない報せ / §git が言ったことを読む場所 /
§git が無い時・古い時)、検証の動詞は verify-ui スキル。コードの正本は `platitude-core` の
`report.rs`(`ReportKind` / `RemoteWhy` / `WriteReport`)と各操作のモジュール。
**この文書に無い判断を実装する時は、先にここへ行を足す。**

## 定義と記録の置き場

**バージョン番号と選定理由はソース側で管理し、文書に転記しない。**
他の依存・実行環境の版も、それぞれのソース・設定の定義を参照する
([反映前テストの機械化.md §版を動かす変更は段 3](反映前テストの機械化.md))。

| 管理するもの | 正本 |
|---|---|
| 最低版・選定理由、発行するコマンド・オプションの互換性確認記録 | [version.rs の `MINIMUM_GIT` とそのコメント](../crates/platitude-core/src/version.rs) |
| 最低版で使えない形式・同梱レシピ | `version.rs` 末尾のコメント |
| 実際に発行する引数・環境変数 | 各操作の `GitCommand` 構築と `process/executor.rs` の共通設定(§発行前の釘付け) |
| 失敗の類型と出し先 | 本文書 §類型 と `report.rs` の `ReportKind` |
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
| 語の表 | [`relied_messages.rs`](../crates/platitude-core/src/relied_messages.rs) | 製品が git の文言から読む物の全部: 分類器が判定に使う語と、git の文から切り出して画面の文に埋める区切り(worktree のロック理由)。**2 種類で書く** —— **形**(`Words`)は 1 つの行の文で、git のどのファイルのどの文から来るか(`- git:` / `- openssh:`)と、実 git にそれを言わせるテスト(`- witness:`。無ければ `none` と理由)を持つ。照合・記録・週次はこの単位。**メッセージ**(`Relied`)は同じことを言う形の束で、どれかが見つかれば当たり(`LOCK_TAKEN.found_in(said)`)。形は表の中でしか作れない(型のコンストラクタが表のモジュールの外から見えない)。**分類器は表の判定を呼び、自前の文字列を書かない**(機械化済み: `cargo xtask structure` —— git の文を読む関数(`said` / `stderr` / `stderr_utf8` / `failure_message` を名指す関数。git の文はこの名で受ける)の中で、文字列リテラル・その `format!`・そのファイルの文字列定数を照合に使う行を止める。`#[cfg(test)]` の項目は読まない。git の文でない物 = 機械形式の答え・アプリ自身の印は、その行か直前のコメントに `// not git's words: <何か>`) |
| 版の差の吸収 | 同上 | **動いている git の版で分岐しない**(ユーザー決定 2026-10-13)。版ごとの形はすべてメッセージの束に並べ、どれかに当たれば良い。版の番号は文言の鍵にならない —— 同じ番号でも Git for Windows(2.43〜2.46 の gpg の文)・Apple Git(本家と 73 ファイル違う 2.50.1)・ディストリの取り込み(Ubuntu の 2.43.0 に 2.43.4 の dubious ownership)で文が違い、撃つ git も設定と PATH への切り替えで変わる。**分ける時は、同じ形が版で別の意味になった時だけ、印は git が言ったこと(文・記録)** |
| 照合の記録 | [`relied-messages.txt`](../crates/xtask/relied-messages.txt)(生成物) | 読んだ木と、各語の行を「同じ行を持つ木」ごとに分けた時代 = 版の境目。行として持つのは**その行が呼ぶ関数(`die` と `error` の別 = 終了コード)と文字列だけ** —— 変数名・コメントの書き換えは変化に数えない。読む木は、最低版以降の本家の全リリース(rc は除く)と `master`、Git for Windows の全リリースと `main`、最低版以降を元にした Apple Git(macOS の Command Line Tools)、[最低版コンテナ](../ci/linux/Dockerfile)の Ubuntu のパッチ済みの木、OpenSSH(git が文をそのまま運ぶ 2 語)。**系列ごとに全タグを並べるので、並行する保守版も 1 本ずつ載る**(v2.43.7 は v2.50.0 より後に出た) |
| 足す・変える時の網 | gate の常時ステップ <!--call:relied-messages.check-->`relied-messages check`(オフライン) | 表と記録が食い違えば赤 —— <!--cmd:relied-messages.audit-->`cargo xtask relied-messages audit`(全木を取り直して記録を書き直す)を撃って commit するまで通らない。証人の完全名は、統合スイートの `relied_witnesses` がそのテストバイナリの `--list` と突き合わせる(表を変えると gate がそのテストを選ぶ) |
| 週次 | [`relied-messages.yml`](../.github/workflows/relied-messages.yml) | `watch`: 本家 `master`・Git for Windows の `main` と、記録に無いリリース(Apple の新しいタグ・動いた Ubuntu の枝を含む)を記録と比べ、**語の行が変わった時だけ赤**(変わった行と、その行を持つ木 = どのリリースまで入ったかを出す)。**保守版・セキュリティ版のタグはどれも `master` の祖先**なので、どのリリースの変化も `master` に現れる。`master`: 本家 `master` を組み、表の証人をその git で回す。**登録した証人が全部走って通らなければ赤**(`--exact` は無い名前を何も選ばずに通すので、実行の報告で数える) —— 行に出ない経路の変化(同じ場面で先に別の `die()` に当たる)を、証人の在る場面で捕まえる |

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

| 固定する物 | 手段 | 置き場 | 無いと何が起きるか |
|---|---|---|---|
| 色・パスの引用・署名表示・index の自動 refresh・任意ロック | `-c color.ui=false` `core.quotepath=false` `log.showSignature=false` `diff.autoRefreshIndex=false` `--no-optional-locks` | `process::executor` `FIXED_ARGS` | `--format` の記録に色・gpg 出力・`"..."` の引用が混ざる / ポーリングの diff が `index.lock` を取って commit を落とす |
| コミット文の文字コード(読み・書き) | `-c i18n.logOutputEncoding=utf-8` `i18n.commitEncoding=utf-8` | `FIXED_ARGS` | 端末向けに Shift_JIS 等を設定した機械で `log` の文が変換されて化ける / アプリが書く UTF-8 の文に別の文字コードの `encoding` ヘッダが付く(ユーザー決定 2026-10-12) |
| ロケール・資格情報の対話・任意ロック・エディタ | `LC_ALL=C` `GIT_TERMINAL_PROMPT=0` `GIT_OPTIONAL_LOCKS=0` `GIT_EDITOR=true` | `FIXED_ENV` | 分類器が読む語が翻訳される / 資格情報の入力で止まる / エディタで止まる |
| 起動元から継いだリポジトリの所在と diff・pathspec の形 | `GIT_DIR` `GIT_WORK_TREE` `GIT_INDEX_FILE` `GIT_OBJECT_DIRECTORY` `GIT_COMMON_DIR` 等 git 自身の `local_repo_env`(environment.c、2.43〜2.55)+ `GIT_EXTERNAL_DIFF` `GIT_DIFF_OPTS` と pathspec magic の 4 変数、`GIT_NAMESPACE`(ref の名前空間)と `GIT_REFLOG_ACTION`(reflog の文 —— `discards::read` が読む)を**毎 spawn で消す**(コマンド自身の `env` は後から載る) | `INHERITED_ENV_DROPPED` | フック・git alias・IDE の実行構成から起動されたアプリの全コマンドが別のリポジトリを指す / パーサが読まない diff の形 |
| diff の形(prefix・文脈行・submodule の表示・外部 diff) | `-c diff.noprefix=false` `diff.mnemonicPrefix=false` `diff.srcPrefix=a/` `diff.dstPrefix=b/`(2.45+ のキー。最低版は無視)`diff.context=3` `diff.submodule=short` `diff.external=` と各 diff の `--no-ext-diff` | `details::DIFF_SHAPE_ARGS` | `a/` `b/` を剥がすヘッダのパースが壊れる / `diff.context=0` の hunk は `git apply` が当てられず部分ステージが常に断られる / gitlink が `diff --git` 無しの形で出る |
| push の送る ref | 明示の refspec + `--no-follow-tags` | `remote::push::porcelain_push` | `push.followTags` で頼んでいないタグが一緒に出る |
| rebase・merge・pull の autostash、駆動 rebase の merge 形 | 3 つとも `--no-autostash`、駆動形だけ `--no-rebase-merges` | `integrate::rebase_command` / `integrate::merge` / `remote::pull` | `rebase.autoStash` / `merge.autoStash` の git が先に自分で退けて素の `apply` で戻し(staged が unstaged に落ちる)、止まった時の作業が `rebase-merge/autostash` / `MERGE_AUTOSTASH` に入って stash list の外へ行く / `label onto` が todo に入る |
| pull の合流の仕方 | `--ff-only --no-rebase`(行が出るのは早送りできる木だけ) | `remote::pull` | `pull.rebase` を読ませると rebase 経路の clean-tree 検査が先に走り、早送りで足りる汚れた木を `cannot pull with rebase: You have unstaged changes` 128 で断る(実測 2.55。ユーザー決定 2026-10-12) |
| merge / pull を塞ぐ作業ツリーの変更・untracked | 断られた時だけ `stash push -u` → 撃ち直し → `stash pop --index`(`build::integrate_carrying`。switch / rebase と同じ 3 手) | `session::build` | 直している当のファイルが向こうでも動いた日常の場面で、`would be overwritten by merge` の失敗がログに落ちる(ユーザー決定 2026-10-12) |
| stash の index 復元 | `--no-index` | `stash::pop` / `apply` / `carried_apply` | `stash.index=true`(2.52 からの鍵)で `--index` 扱いになり着地が変わる |
| mergetool の一時ファイル・ツール・GUI | `-c mergetool.writeToTemp=true` `--no-prompt --gui --tool=` | `conflict::tool` | 一時ファイルが隣に出て WIP が埋まる / ツール未設定で stdin を踏む |
| 一時 index の写し | `core.hooksPath=<absent>` `core.splitIndex=false` `core.fsmonitor=false` `core.safecrlf=false` `index.skipHash=true` + `GIT_INDEX_FILE` | `discards::record::isolated` | 写しのフック・fsmonitor・改行検査が本体に混ざる |
| worktree add の進捗 | `--quiet` | `worktrees::add` | 2 秒を超える checkout の `Updating files:` が拒否の頭に付く |
| 部分破棄の改行 | 作業ツリーのファイルの改行が一様なら、その書き方の `core.autocrlf` を渡す(LF = `input`、CRLF = git が変換した時だけ `true`) | `stage::endings` | `core.autocrlf` の下の `apply -R` がファイル全体を変換し直し、触っていない行まで改行が変わる |
| 部分操作の mode | 部分 patch から `old mode` / `new mode` の対を落とす(ユーザー決定。デザイン規約 §mode の変更) | `patch::header::fitted` | 対があると `apply` がその mode を当て、選んだ行と一緒に実行ビットが動く(アンステージがステージ済みの `+x` を戻す) |
| pathspec の魔法 | 各パスを `:(literal)` で包む(`GIT_LITERAL_PATHSPECS` は使わない —— git 内部の pathspec まで届き `stash push -u` が untracked を置き忘れる) | `process::literal_pathspec` | `*` `?` `[` を含むファイル名が glob と読まれる |

### 釘付けない物(意図して利用者の設定に従う)

| 設定 | 従う理由 |
|---|---|
| `commit.gpgsign` / `tag.gpgSign` / `gpg.format` | 署名は利用者の約束(forge の required signatures)。署名できない時は報せ `CommitUnsigned` |
| `branch.<n>.merge` / `branch.<n>.remote` | pull がどこへ取りに行くかは利用者の設定(合流の仕方だけ釘付け = 上の表) |
| `rebase.updateRefs` | 駆動形は `--update-refs` を自分で足す。枝のまとまりの読みは rules-refs/core.md |
| `gc.auto` / `maintenance.auto` | リポジトリの保守は利用者の物。後ろで走る gc が取るロックは `*Locked` の報せで受ける |
| `core.fsmonitor` / `core.untrackedCache` | 外すと書き込みが遅くなり index の印が消える(rules-refs/core.md)。trace2 の受け口はデーモンが繋いでも閉じるだけ |

## 類型(出し先と判定)

列は 類型 / git の答え(`LC_ALL=C`)/ 起きる場面と頻度の判定 / 出し先(`ReportKind` と 2 行目の要旨)。
2 行目の文そのものは `Words.qml` の `writeReportedWhy` とデザイン規約 §答えの要らない報せ の表が持つ。
文字列・終了コードの根拠(版・ソース・再現)は rules-refs/core.md の各行。

### push(ブランチ・タグ・リモートの削除・差し替え)

送った ref の行だけを読む(`remote::refusal::sent_line`)。`[rejected]` = こちらの判定、`[remote rejected]` = 向こうの判定。

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| こちらが古い | `[rejected] (fetch first / non-fast-forward / remote ref updated since checkout)` | 日常 | 報せ `Outdated` + その場で fetch |
| lease が外れた | `[rejected] (stale info)` | 画面の後に向こうが動いた(日常) | 報せ `Moved` / `MovedDelete` + 読み直し |
| タグが向こうで別の commit | `[rejected] (already exists)` | 同名タグ(ある) | 報せ `TagElsewhere`(同じ commit なら成功扱い) |
| 向こうのフック・forge の規則 | `[remote rejected] (… hook declined)` | **GitHub の保護ブランチ・required signatures・100MB のファイル上限、GitLab の push rules・保護ブランチは全部ここ**(基準 2) | 報せ `RemoteUpdate/Delete(Rule)` |
| `receive.denyNonFastForwards` | `(non-fast-forward)` | `git init --shared` の bare(稀。文字列 1 行なので残す = 基準 1) | 報せ `(Rewrite)` fetch は撃たない |
| `receive.denyDeletes` / `denyCurrentBranch` / `denyDeleteCurrent` | `(deletion prohibited)` / `(branch is currently checked out)` / `(deletion of the current branch prohibited)` | 作業ツリーを持つ向こうへ直に送る(初心者の典型・稀)。GitHub の既定ブランチ削除は独自文で `Other` に落ちる | 報せ `(NoDeletes / CheckedOut / Current)` |
| サイズ上限 | `(unpacker error)` + `remote: … pack exceeds maximum allowed size` | GitHub の 2GiB/push、GitLab の上限(基準 3) | 報せ `(TooLarge)`(向こうの locale が別なら `Other`) |
| shallow から送る / 向こうの ref ロック / その他 | `(shallow update not allowed)` / `remote:` 行のロック / 他 | 稀 | 報せ `(Shallow / Locked / Other)` |
| 手元の `pre-push`(husky・lefthook・git-lfs、2.54+ の設定フック) | ref 行も `To` 行も無い exit 1、git の語は `failed to push some refs` だけ | **日常**(基準 2) | 報せ `PrePush` / `PrePushDelete` —— **trace2 のフックの終わり**で見分ける(§日常性能への代金) |
| LFS サーバーに届かない | git-lfs の `pre-push` の文 | オフライン | 報せ `PrePush`(断ったのはフック) |
| **署名 push(`push.gpgSign`)**を署名できない・向こうが受けない | `fatal: failed to sign the push certificate` / `the receiving end does not support --signed push` | **稀な構成。GitHub / GitLab は signed push を受けない**(基準 3) | **ログ**(分類しない) |
| `needs force` / `remote does not support deleting refs` / `[remote failure]` | stdout の ref 行 | 稀 | ログ(push の失敗行は stdout も載せる) |
| 資格情報が無い・鍵が通らない・host key が未知 | `could not read Username … terminal prompts disabled` / `Authentication failed for` / `Permission denied (publickey)` / `Host key verification failed` | **日常**(新しい機械・WSL・初回の SSH) | ログ + **ヘッダの 1 行はアプリの 1 節**(`remote::unreached` → `Words.unreachedWhy`。ユーザー決定 2026-10-12) |
| 届かない(名前解決・経路・タイムアウト) | `Could not resolve host` / `unable to access` / `GitError::TimedOut` | 日常(オフライン) | ログ(ヘッダは git の 1 行目)。**auto fetch の失敗だけはログを開かず fetch ボタンの `!`** |

### commit・amend

押す前のゲートで、こちらが答えられる拒否は全部取り除いてある(identity・空メッセージ・空の選択・空になる amend・
merge 中の amend・unmerged)。残る失敗は常に外が断った物。

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| `pre-commit` / `commit-msg` / `prepare-commit-msg` | フックの文 exit 1(git は語を足さない) | **日常**(基準 2) | 報せ `CommitHook`(trace2 のフックの終わり) |
| 署名できない | `gpg failed to sign the data` / ssh の鍵・`ssh-keygen -Y sign` の失敗 | 日常(verified commits の鍵・agent) | 報せ `CommitUnsigned`(語か trace2 の署名プログラムの終わり) |
| 他の git のロック | `.lock': File exists` / reftable `cannot lock references` | **日常**(IDE の裏の git。基準 2) | 報せ `CommitLocked` |
| それ以外(index を書けない等) | git の文 | 範囲外(ディスク) | 報せ `Commit`(`git turned it down`) |

### 履歴の書き換え(squash / drop / reword・プラン)

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| 撃つ前に断る 7 種 | — | merge を跨ぐ / 今のブランチが見ない / 最初のコミットの畳み先 / 全部 drop / 取得されていない親 / 先端が動いた / 操作が立っている | 報せ `Withheld`(`RewriteAcrossMerge` 〜 `RewriteWhileStanding`) |
| `pre-rebase` | 2.47+ `error:` exit 1 / 2.43〜2.46 `fatal:` 128 | 日常(フック) | 報せ `RewritePreRebase` |
| 置き直すコミットのフック・署名・ロック | 文 + 止まり、または trace2 | 日常(husky・署名) | `rebase --abort` で戻してから報せ `RewriteCommitHook` / `RewriteUnsigned` / `RewriteLocked` |
| 最初の checkout が untracked を上書き | `untracked working tree files would be overwritten by checkout` exit 1 | ある(git の前に推さない —— carry が先に退ける) | 報せ `RewriteUntrackedInTheWay` |
| conflict・`edit`・空になった pick の停止 | exit 1 + `rebase-merge` | 日常 | 停止(帯と出口カード) |
| `rebase.missingCommitsCheck=error` | — | 起きない: 検査は todo から消えた行だけを数え(rebase-interactive.c `todo_list_check`)、プランは `drop` 行を書く(網 `drop::a_readers_missing_commits_check_never_sees_the_plans_drop_lines`) | — |

### 改名(ブランチ / タグ / stash)・worktree のフォルダ名

箱が押す前に断る: 空・git の名前規則・既存名・大小だけ違う名前・D/F 衝突・どこかの worktree で止まった
rebase / bisect が持つブランチ・OS が許さないフォルダ名・空いていない場所・他の worktree の記録。

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| git が改名を断った(押した後の競争・ロック) | 文 128 | 稀(ロックは日常) | 報せ `RenameRefused` / `RenameLocked`(stash は `StashRenameRefused` / `StashRenameLocked`)、箱は開いたまま |
| ブランチの ref は動いた後で止まった | `branch is renamed, but update of config-file failed` / `HEAD is not updated` | 稀 | 報せ `RenameUnfinished` |
| **2 手の改名が途中で止まった**(タグの削除・stash の drop・一覧が動いた) | 文 1 / 無し | **稀(2 コマンドの隙間の競合)** | 報せ **`HalfRenamed` 1 つ**。何が止めたかは分けない(基準 3)—— git の断り・ロックはログのその段の行が持ち、一覧が動いた形は赤い行が無い(`rev-parse` が別の entry を答えただけ) |
| stash が選択の後に消えた | — | 稀 | 報せ `StashGone`(箱は閉じる) |
| worktree のフォルダ名を git が断った / 行き先に物 | 文 128 / — | 稀 | 報せ `WorktreeNotRenamed` / `WorktreeRenameTaken` |

### 部分ステージ・アンステージ・破棄

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| 読んだ diff の後にファイルが動いた(指紋の不一致・hunk が消えた・apply の後の読み直しが違う) | — / `patch does not apply` | 日常(編集中のファイル) | 報せ `StaleStage/Unstage/Discard`(`Withheld`、ペインは読み直す) |
| conflict したファイルの一部 | — | ゲート(部分選択を出さない) | 届けば報せ `ConflictedPart` |
| LFS のポインタ・gitlink の diff の一部 | git は通すが、壊れたポインタが index に入る / 破棄で作業ファイルがポインタの断片や 0 バイトになる / `corrupt patch for submodule` 128 | ゲート(diff は 1 枚 = ファイル単位の操作だけ。mode `160000` と属性 `filter=lfs` で見分け、hunk の上の 1 行が理由を言う) | — |
| 他の git の `index.lock` | `index.lock': File exists` 128 | 日常(IDE) | 報せ `StageLocked / UnstageLocked / DiscardLocked` |
| `diff.context=0` | — | 釘付けで起きない | — |
| `core.safecrlf=true` の変換拒否・その他 | `LF would be replaced by CRLF` 128 | 稀 | ログ |
| 改行が混ざったファイルの部分破棄(git が改行を変換する所) | git は通すが、触っていない行の改行まで一様に書き直す(渡せる `core.autocrlf` が無い)。git は改行を揃えて比べるので diff にもコミットにも出ない | 稀(基準 3) | 今のまま(ユーザー決定) |

### worktree(add / remove)・別の worktree の変更の適用

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| 未コミットの変更・ロック・submodule で消せない | 128 | ある | 報せ `WorktreeKept / WorktreeLocked / WorktreeRemoveRefused` |
| フォルダを消し切れない(Windows の使用中・長いパス) | 255 | **ある(Windows)** | 報せ `WorktreeHalfRemoved` |
| 作る前にフォルダが埋まっている / 記録だけ残る場所 / `-b` のブランチが残った / 他の拒否 | — / 128 / 255 | 押す前に言える衝突が押した後に起きた | 報せ `WorktreeFolderTaken`(`Withheld`)/ `WorktreePlaceTaken` / `WorktreeBranchLeft` / `WorktreeNotAdded` |
| `post-checkout` が失敗(git-lfs 無し) | フックの終了コード、worktree は残る | ある(LFS) | 報せ `WorktreeHookFailed`(trace2) |
| 別の worktree の変更を当てられない | `would be overwritten` / `could not write index` / 無言(他の git の index ロックを名指すのは 2.51+。2.43.2〜2.50 は `could not write index` だけ、2.43.0〜2.43.1 は無言。断りの終了コードは 2.55 まで 1、2.56 から 128) | ある | 報せ `CarriedInTheWay / CarriedFileInTheWay / CarriedUnread / CarriedRefused / CarriedLocked` |
| 同、ファイルとフォルダの入れ替わり・大小を区別しないディスク越しの 3 つの形(追跡分が当たった後に untracked を置けない 2 つ、`d/f` → untracked `d` を git の前に断り過ぎる 1 つ) | `checkout-index` 128 / — | 稀(手で作る形) | **据え置き**(ユーザー決定 2026-10-12): 追跡分は当たったまま git の文がログへ。詳細は P3-確認事項 §別の worktree の未コミット行 |

### merge・pull・switch・cherry-pick・revert

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| conflict で止まった | exit 1 + marker | 日常 | 停止(帯・出口カード) |
| 早送り・本物の merge を作業ツリーの変更・untracked が塞ぐ | `would be overwritten by merge` exit 1(早送り)/ 2(本物の merge)、marker 無し | **日常**(merge / pull で、直している当のファイルが向こうでも動いた) | **carry**(`stash push -u` → 撃ち直し → `stash pop --index`。conflict で止まれば stash は残る。ユーザー決定 2026-10-12)。stash で越えられない断り(`--skip-worktree` 等)は戻して git の最初の文をログへ |
| `pull.rebase=true` + 作業ツリーに変更 | — | 釘付けで起きない(`--ff-only --no-rebase`) | — |
| 割れた木の pull | — / 数が古ければ `Not possible to fast-forward` 128 | ゲート(行を灰にして行き先を言う)/ 数が古い木だけ届く | ログ |
| switch を塞ぐ変更 | `CheckoutOutcome::Blocked` | 日常 | carry(stash → switch → pop) |
| 名前が merge できない・2 つ目の merge | 128 | 稀 | ログ |

### 読み・開く・git そのもの

| 類型 | 答え | 場面・頻度 | 出し先 |
|---|---|---|---|
| git が無い | spawn の `NotFound` | 初回 | 全画面(デザイン規約 §git が無い時・古い時) |
| 古い git | `--version` | ある | 帯の `OLD GIT`、操作は通す |
| リポジトリでない・bare | `not a git repository` / `--is-bare-repository` | 日常 | 開けなかった画面・ダイアログ |
| 壊れた `.git`(指す先が消えた・壊れた `.git` ファイル、HEAD の空いた `.git` フォルダ) | 128。`.git` フォルダは全版で `not a git repository (or any of the parent directories)`、`.git` ファイルは 2.55 まで `not a git repository: <指す先>`(2.54・2.55 は `(null)`)・2.56 から `gitfile does not point to a valid repository` —— 開いたフォルダ自身に `.git` が在ることで見分ける(git の文では分けられない)。中のフォルダを開いた時は見分けず git の文が決める(デザイン規約 §リポジトリを開く) | 稀(リンクした worktree の本体・submodule の git dir を動かした・消した、落ちた時の破損) | 開けなかった画面の「git が答えられなかった」+ git の文 |
| `safe.directory`(別の所有者のフォルダ) | `fatal: detected dubious ownership in repository` 128 | **ある(Windows のネットワークドライブ・別アカウントの clone・WSL の mount)** | 開けなかった画面の見出し `git does not trust this folder's owner` + git の文(末尾が許す手順。`GitError::UntrustedOwner`。ユーザー決定 2026-10-12) |
| 設定の書き換えと重なった読み(Windows) | `fatal: unknown error occurred while reading the configuration files` 128 / `fatal: unable to access '<file>'` 128 / `git config --get*` は `warning: unable to access '<file>'` だけで残りの設定から答える | ある(アプリ自身の設定の書き込みや外の `git config` と、開き・読み直しが重なった時) | **読みだけ 1 回撃ち直す**(`process::config_unread`。書き込みは撃ち直さない。ユーザー決定 2026-10-13)—— §設定の書き換えと重なった読み |
| 設定がずっと読めない(権限・別のプログラムが掴む) | 同じ文が撃ち直しでも | 稀 | 開けなかった画面の見出し `git could not read its configuration` + 直してから開き直す 1 文 + git の文(ファイルを名指す。`GitError::ConfigUnreadable`。ユーザー決定 2026-10-13) |
| 読みの失敗(log・status・diff) | 非ゼロ | 稀 | 帯 `STALE GRAPH` / ログの背景読み |
| 押した fetch の失敗(連なり) | 非ゼロ | オフライン(日常) | 最初の失敗だけログが開き、復帰で閉じる。タイマー・開いた時の fetch はボタンの `!` だけ(§出し先) |

### 設定の書き換えと重なった読み

git は設定ファイルを `<file>.lock` に書いて rename で置き換える。**Windows ではその間に起動した git が
ファイルを開けない**(削除待ちのファイル = `Permission denied`)。リポジトリのコマンドは上の 2 つの fatal で
落ち、`git config --get*` は読めなかったファイルを飛ばして答える(builtin/config.c `get_value` は読みの結果を
見ない)—— そこにだけ在るキーは「未設定」に読まれる。次に起動した git は普通に読める。mac / Linux の rename は
読み手に対して一瞬で済み、この形は起きない。

- **撃ち直すのは読みだけ・1 回だけ**(`process::config_unread`)。読みかどうかはサブコマンドとその形で決める
  (`rev-parse` / `log` / `status` 等と、`config --get*` / `stash list` / `worktree list` / 値も `-d` も無い `symbolic-ref` 等)。
  書き込みは書いた後で落ちうるので撃ち直さない。警告だけの形は `git config` の読みに限る —— 他のファイルが
  ずっと読めないリポジトリで、毎回の `status` を 2 本にしない。流し読みは 1 バイトも渡していない時だけ。同じ実行枠・同じログの行のまま撃ち直し、行は最後の答えを言う
- **文言の根拠**: 3 つの文(config.c `repo_read_config` の die、wrapper.c の `warn_on_fopen_errors` と
  `access_or_die`、die は 128)は語の表の `CONFIG_UNREAD` / `FILE_UNOPENED` で、読んだ全リリースでの行は
  [照合の記録](../crates/xtask/relied-messages.txt)が持つ。executor は `LC_ALL=C` で起こすので翻訳されない。将来の版で
  文が変われば、その版では撃ち直さずに 1 回目の答えのまま失敗する(週次の照合が赤になる)
- **効き目**(git 2.55、グローバル設定を書き換え続ける横で読む): 開きの `rev-parse` と `config --get-regexp` を
  計 1,200 回 —— 撃ち直し無しで `rev-parse` が 13 回落ち、撃ち直し有りで 0 回(撃ち直し 18 回)
- **2 回目も落ちれば、それはずっと読めない形**: 開きでは専用の画面(上の表)、それ以外の読みは今までどおり

## 分類しない物(ログに落とす)と理由

| 物 | 理由 |
|---|---|
| 署名 push(`push.gpgSign`)の 2 類型 | 稀な構成で主要 forge が受けない。再現に `receive.certNonceSeed` と鍵の設営が要る(基準 3) |
| 半分済んだ改名の「何が止めたか」(ロック / 断り / 一覧が動いた) | 2 コマンドの隙間の競合は稀。1 つの報せとログで足りる(基準 3) |
| `[rejected] (needs force)` 等 | 稀。理由は stdout の ref 行がログに持つ |
| 届かない通信(名前解決・経路・タイムアウト) | git の 1 行目で足りる。資格情報・鍵・host key の 3 つだけはヘッダの 1 節(上の表) |
| `core.safecrlf` の拒否・`checkout-index` が置けない untracked・大小を区別しないディスク越し | 稀・据え置き(P3-確認事項) |
| リポジトリの破損・ディスク満杯・メモリ不足 | 範囲外 |

## 日常性能への代金(異常時の扱いが日常に払う物)

実測は [code-costs-windows-x64.md](../ci/baseline/code-costs-windows-x64.md) に 1 行ずつ。

| 仕組み | 代金 | 判定 |
|---|---|---|
| **trace2 の記録**(`GitCommand::traced` = commit / push / 駆動 rebase / worktree add。フック・ssh 署名プログラムの終わりを git 自身の記録で読む) | **Windows は git プロセス 1 本につき約 +19ms**(git が `cmd_ancestry` のために `CreateToolhelp32Snapshot` で全プロセスを数える。compat/win32/trace2_win32_process_info.c —— 止める設定・環境変数は無く、どの target でも同じ)。commit・push の増分は code-costs の行。Linux は `/proc` 読みで未実測 | **維持**(ユーザー決定 2026-10-12)。git にフックの断りを名指す無料の機構は無い: 終了コード(commit の 1)で推す案は空メッセージ等のゲートに精度が依存し、ssh 署名プログラムの失敗も名指せなくなる。他の GUI(GitHub Desktop・GitKraken・Sublime Merge)は名指さず、フックの生の出力を出して `--no-verify` の「それでも commit」を添える |
| 部分ステージ前の指紋の照合(diff を 1 本読み直す) | press ごとに diff 1 本 | 維持(読んだ行と違う物を当てない保証) |
| リモート削除前の lease(`ls-remote`) | press ごとに往復 1 つ | 維持(画面の先端への lease = 向こうの別の commit を消さない) |
| 他の worktree の読み(carried) | worktree 数に比例する `status` | ペースで薄める(P3-確認事項 §別の worktree の未コミット行) |
| `diff.autoRefreshIndex=false`(ポーリングが index を書かない) | 利用者の `git status` が racily-clean を再計算 | 維持(commit を落とさない) |

## サポート外化の候補(日常性能を理由に)

| 候補 | 状況 | 落とすと | 判定 |
|---|---|---|---|
| trace2 でフックの終わりを読む(4 つの書き込み) | §日常性能への代金の行(Windows は git プロセス 1 本ごと) | `CommitHook` / `PrePush` / `RewriteCommitHook` / `WorktreeHookFailed` と ssh 署名プログラムの失敗を名指せず `git turned it down` へ。commit は exit 1 + stdout 空を git の答えとして読めるが、pre-push は ref 行の無い exit 1 が通信の失敗と区別できない | **維持**(ユーザー決定 2026-10-12)。再検討するなら落とすのは commit の trace だけ |

## 判断待ち(要判断)

状況と、基準に照らした選択肢。決まったら上の表へ写してここから消す。

- **別のリポジトリの作業ツリーの中に、HEAD の空いた `.git` フォルダを持つフォルダがあると、そこを開くと外側のリポジトリが
  黙って開く**(実測 2.55: `rev-parse --show-toplevel` が exit 0 で外側の頂上を答える。git は受け取れない `.git` フォルダを
  素通りして親へ辿る —— setup.c の discovery、全版。`.git` ファイルなら git がそこで止まるので起きない)。稀(入れ子の
  リポジトリが落ちた時に壊れた)。選択肢: (a) 今のまま / (b) 開けた時に、開いたフォルダ自身が `.git` を持つのに git の
  答えた頂上がそのフォルダでなければ開かずに「git が答えられなかった」へ(`repo::open` の成功側の比較 1 回、git は撃ち足さない。
  ただし出す git の文が無いので、その画面の 3 行目を決める要がある)
