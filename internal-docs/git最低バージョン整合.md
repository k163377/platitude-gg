# 最低 git バージョン整合チェック

**対応最低バージョンは 2.43 で、この数値の正本は本ファイル**(コード側の定義は `platitude-core::version::MINIMUM_GIT`)。
根拠は **Ubuntu 24.04 LTS の標準版**であること。

**これは「保証する下限」** — 下限未満の git でもアプリは起動して動く([デザイン規約.md](デザイン規約.md) §git が無い時・古い時)。
下限未満の環境で何が落ちるかは下表がそのまま答える(**その版に無いものだけが git 自身の文言で失敗する**)。

以下は、アプリが発行する全 git コマンド・オプションがその 2.43 に存在するかの論理チェック記録。
根拠は**マニュアル照合**(git-scm.com のバージョン付きマニュアル + git.git の v タグ付きドキュメントソース)。実バイナリでの一巡は [P5-確認事項.md](P5-確認事項.md) §6。

- 洗い出し: `platitude-core/src` の `GitCommand` 構築の全数(`-c` 固定引数・環境変数・pathspec magic 含む)
- 増えたコマンド・オプションは §新オプション追加時の手順で行単位に追記(全数の再洗い出しは P5 §6 の実バイナリ一巡と併せて行う)
- 判定基準: 2.43.0 のマニュアルに記載があれば ✓。2.30 より前からある古参は導入バージョンの知見で確定し「古参」と記す

## 結論

**発行する全コマンド・全オプションが 2.43 に存在する。**

**マニュアル照合が拾えない例外が 1 つある — `reset` は `--end-of-options` を受け付けない。** gitcli(7) の一般規定に載っていても、`git reset` は 2.43 で位置によらず `fatal: option '--end-of-options' must come before non-option arguments`(exit 128)を返す。アプリが発行する動詞のうちこれだけで、`branch` / `switch` / `tag` / `remote add` / `remote set-url` / `ls-remote` / `rev-parse --verify --quiet` / `log -1 --format=` / `stash list` は全て通る。名前を渡したい経路が将来できたら、`rev-parse --end-of-options` で解決してから object id を渡す。**この差は開発機の git(新しい版は受け付ける)では出ない** — 最低バージョンを積んだ Linux コンテナ(`cargo xtask linux`)が唯一の検出点。

**同梱物にもバージョン差がある。** git が持つのはマージツールの**起動レシピだけ**(`$(git --exec-path)/mergetools/`)で、その顔ぶれが版で変わる — **2.43.0 は `vscode` を含まない**(追加は 2.47)。`smerge` は 2.22 から在る。**アプリはツール名を利用者から受け取るだけで候補を持たない**ので現状これに依存しないが、将来「既定を提案する」を作るなら名前を書くだけでは 2.43 で外れる(`mergetool.vscode.cmd` を自前で書く形になる)。

## 毎回付くもの(process/executor.rs)

| 種別 | 値 | 確認 |
|---|---|---|
| 固定引数 | `-c color.ui=false` / `-c core.quotepath=false` / `-c log.showSignature=false` | 古参(設定キーはいずれも 2.10 以前) |
| 固定引数 | `--no-optional-locks` | 2.15 |
| 環境変数 | `LC_ALL=C` / `GIT_EDITOR=true` | 古参 |
| 環境変数 | `GIT_TERMINAL_PROMPT=0` | 2.3 |
| 環境変数 | `GIT_OPTIONAL_LOCKS=0` | 2.15 |
| 環境変数 | `GIT_SEQUENCE_EDITOR`(rebase 駆動時) | 1.7.8。rebase 2.43.0 マニュアルにも記載 ✓ |
| pathspec | `:(literal)` プレフィックス | 1.9 |
| diff 表示固定 | `-c diff.noprefix=false` / `-c diff.mnemonicPrefix=false` / `-c diff.external=` | 古参 |
| 破棄記録の写しと記録(`discards::record` / `discards::copy`)と復元(`discards::restore`) | `-c core.hooksPath=<在ることの無いフォルダ>` / `-c core.fsmonitor=false` / `-c core.splitIndex=false` / `-c core.safecrlf=false` / `-c index.skipHash=true`(一時の index)、復元の `git apply` に `-c core.safecrlf=false`、環境変数 `GIT_INDEX_FILE` / `GIT_AUTHOR_NAME` `GIT_AUTHOR_EMAIL` `GIT_COMMITTER_NAME` `GIT_COMMITTER_EMAIL` | `core.hooksPath` 2.9、`core.splitIndex` 2.12、`core.fsmonitor` を真偽で読むのは 2.36(それより前はフックのパス)、`index.skipHash` 2.40 — いずれも 2.43.0 ✓。環境変数は古参。効き方は `discards_integration::recording` が最低版のコンテナでも見る |

## サブコマンド別

| コマンド | 使用オプション | 確認 |
|---|---|---|
| `--version` | — | 古参 |
| `add` | `--all` / `--intent-to-add` / `--force` / `--sparse`(破棄記録の写し) | 古参(1.6.1)。`--sparse` は 2.34 — 2.43.0 ✓ |
| `apply` | `--cached` / `--reverse` / `--check` / `--whitespace=nowarn` | 古参 |
| `branch` | `-d` `-D` `-m` `-M` / `--set-upstream-to=` / `--end-of-options` | 古参(1.8.0)+ gitcli 2.43.0 ✓ |
| `cat-file` | `-s` / `blob` / `commit` | 古参 |
| `check-attr` | `-z` / 属性名の並び / `--` | 2.43.0 ✓ |
| `restore` | `--ours` / `--theirs`(conflict 用) | 2.43.0 ✓ |
| `cherry-pick` | `--no-edit` / `--continue` `--abort` `--skip` `--quit` | 2.39.3 ✓。**`--no-edit` のみ個別マニュアル外** → gitcli §Negating options でカバー |
| `clean` | `-f` `-d` | 古参 |
| `clone` | `[--] <repository> <directory>` | 古参(v2.43.0 の synopsis に `[--]` を確認) |
| `commit` | `--amend` / `--allow-empty` / `--reset-author` / `--no-edit` / `--cleanup=whitespace` / `--file` | 2.43.0 ✓ |
| `commit-tree` | `-p` / `-m` | 古参(1.7.6) |
| `config` | `--get` / `--get-regexp` / `-z` / `--global` / `--local` / `--show-scope` / `--unset` / `--type=bool`(`filter.lfs.required` を真偽値として読む)/ 書き込みは旧形式 `config <key> <value>` | 古参 + **`--show-scope` は 2.26** ✓ + **`--type=bool` は 2.18** ✓(Linux コンテナの `lfs_integration` が 2.43 で読む)。`--unset` の exit 5(元から無い)も 2.43 実測。**印が書くキー `checkout.defaultRemote` は 2.19**(`remote.pushDefault` は古参) |
| `diff` | `--cached` / `--no-ext-diff` / `--find-renames` / `--no-renames` / `--name-status` / `--diff-filter=` / `-z` / `--no-index` / `--end-of-options`(`reset --hard` が書き潰す untracked) | 古参(`--no-index` 1.5.1、`--end-of-options` 2.24) |
| `diff-tree` | `-r` / `--no-commit-id` / `-z` / `--name-status` / `--name-only` / `--diff-filter=A` / `--find-renames` / `--no-renames` / `--root` / `--no-ext-diff` / `-p` / `--binary` / `--full-index` / `--no-color`(破棄記録の復元のパッチ)/ `--end-of-options` | 2.43.0 ✓ |
| `fetch` | `--prune` / `--all` | 古参(1.6.5 / 1.6.6) |
| `for-each-ref` | `--format=`(`refname` `objecttype` `objectname` `*objectname` `upstream` `HEAD` `creatordate:unix`) | 2.43.0 ✓(`:unix` は `--date=unix` 委譲、git-log 2.43.0 ✓) |
| `for-each-ref` | `--points-at=` / `--count`(プラン preview の onto 名) | 古参(--points-at は 2.7、--count は 2.0)+ 2.43.0 ✓ |
| `for-each-ref` | 前置 `-c push.default=current` / `--format=`(`push` `push:remoteref` `push:track`)/ パターン `refs/heads/<branch>`(push 先の追跡 ref との数 — `remote::push_track`) | 2.43.0 ✓(`push` は `upstream` と同じ `:track` `:remoteref` を受ける旨の記載。`push.default=current` は古参)。実バイナリは `push_default_integration` を `cargo xtask linux test` で |
| `ls-files` | `-z` / `--eol` / `--unmerged`(破棄記録の写し) / pathspec | `--eol` は 2.8、`--unmerged` は古参 |
| `ls-tree` | `-r` / `-z` / `--name-only` / `--full-tree` / `--end-of-options`(写しの untracked のパス・conflict 中のパスの HEAD の版) | 古参 + gitcli 2.43.0 ✓ |
| `read-tree` / `write-tree` | `<tree-ish>` / —(一時の index で) | 古参 |
| `update-index` | `--add` / `--cacheinfo <mode>,<object>,<path>` / `--force-remove` / `--`(一時の index で) | 古参(カンマ区切りの `--cacheinfo` は 2.0) |
| `update-ref` | `--create-reflog` / `-m` / `<ref> <new>`(`refs/pgg/discards`)/ `--end-of-options <ref> <new> ""`(復元のタグ = 名前が空いている時だけ立てる) | `--create-reflog` 2.6、`--end-of-options` 2.24 — 2.43.0 ✓ |
| `log` | `-z` / `-1` / `--date-order` / `--branches` `--remotes` `--tags` / `--max-count=` / `--ignore-missing` / `--reverse` / `--format=` / `--end-of-options` | 2.43.0 ✓。`-z` は diff-options.txt の `ifdef::git-log` を v2.43.0 ソースで確認。`--date-order` は古参 |
| `log` | `--walk-reflogs` に `--branches` と名指しの `HEAD` / `main-worktree/HEAD` / `worktrees/<id>/HEAD` / `refs/pgg/discards`、`--ignore-missing` / `--date=unix` / `--shortstat` / `--diff-merges=first-parent` / `--no-walk` / `--format=%(trailers:key=,valueonly,separator=)`(破棄記録 — `discards`) | 古参(`--walk-reflogs` 1.5)+ `--date=unix` 2.9 + `--diff-merges=` 2.31。他のコピーの HEAD の名指しは git-worktree(1) 2.43.0 §REFS ✓。実バイナリは `discards_integration` を `cargo xtask linux test` で |
| `ls-remote` | `--tags` / `--heads` / `--end-of-options` / `--` / パターン複数(`refs/tags/<t>` と `refs/tags/<t>^{}`) | 古参(`--tags` / `--heads` は 1.0 以前)。`--end-of-options` は gitcli 2.43.0 ✓。`^{}` の行はそれを名指すパターンにしか当たらない(2.43 / 2.55 で実測) |
| `merge` | `--no-edit` / `--no-ff` / `--ff-only` / `--squash` / `--continue` `--abort` `--quit` | 2.43.0 ✓ |
| `merge-base` | `--is-ancestor` | 古参(1.8.0) |
| `mergetool` | `--no-prompt` / `--gui` / `--tool=` / `--tool-help` | 2.43.0 ✓ |
| `config` | `--get-regexp` に `^mergetool\..*\.cmd$`(自前定義ツールの列挙) | 古参 |
| `-c` | `mergetool.writeToTemp=true`(mergetool 実行時) | 2.43.0 ✓ |
| `push` | `--porcelain` / `--set-upstream` / `--force-with-lease=<ref>:<oid>` / `--force` / `--delete` / **`--force-with-lease=<ref>:<oid>` と `--delete` の併用**(`refs/heads/<b>:<oid>` + `--delete -- <remote> refs/heads/<b>`、タグは `refs/tags/<t>:<oid>` + `--delete -- <remote> refs/tags/<t>`) | 2.43.0 ✓。併用は最低バージョンのコンテナで実測(`cargo xtask linux test -p platitude-core --test it remote_`)— 一致で `[deleted]`、ズレと向こうに無い修飾名で `(delete):<ref>` の `[rejected] (stale info)` + exit 1。2.55 と同じ答え |
| `rebase` | `--interactive` / `--no-rebase-merges` / `--onto` / `--root` / `--update-refs` / `--continue` `--abort` `--skip` `--quit` | 2.43.0 ✓(`--update-refs` 2.38 = **最も新しい依存**) |
| `remote` | `add` / `set-url` / `--end-of-options` / `get-url --push --`(`pushurl` が無ければ fetch の URL) | 古参(`set-url` 1.7.0、`get-url` 2.7)+ 2.43 実測(上記。`get-url --push --` はコンテナの `remote_` テスト) |
| `reset` | `--quiet` / `--soft` `--mixed` `--hard` | 古参。**`--end-of-options` は付けられない** |
| `restore` | `--staged` / `--worktree`(併用) | 2.43.0 ✓(2.23 導入。2.43 時点 EXPERIMENTAL 表記) |
| `rev-list` | `--count` / `--merges` / `--not` / `--remotes` / `--max-count=` / `--branches` / `--exclude=` / `--glob=` | 古参(`--count` 1.7.2 / `--exclude` 1.9 / `--glob` 1.7.0)。`--exclude` / `--glob` の効き方の罠は rules-refs/core.md |
| `rev-list` | `--ignore-missing` / `--parents` / `--tags`(破棄記録の届かないコミットの walk) | 古参 + rev-list-options 2.43.0 ✓ |
| `rev-parse` | `--verify` / `-q`(`--quiet` の綴りも) / `--git-path` / `--path-format=absolute` / `--show-toplevel` / `--absolute-git-dir` / `--show-object-format` / `--symbolic-full-name`(破棄記録の写しが読む HEAD の名前) / `--end-of-options` / `<rev>^{commit}` / `<rev>^{tree}` | 2.43.0 ✓(`--path-format` は 2.31) |
| `revert` | `--no-edit` | 2.43.0 ✓ |
| `rm` | `--cached` / `-r` / `-f` / `--quiet` | 古参 |
| `show` | `-z` / `-r` / `--name-status` / `--find-renames` / `--diff-merges=first-parent` / `--format=` / `--format=%(trailers:key=,valueonly,unfold,separator=)` | 古参 + `--diff-merges=` は **2.31**(`first-parent` は導入時からの値)。**trailers の 4 オプションとも 2.43.0 ✓** |
| `stash` | `list -z --format=`(`%P` と `%(trailers:key=Stands-on,valueonly,separator=%x20)` = 破棄記録の写しの土台) / `push -m --include-untracked --keep-index --staged` / `pop --index` / `apply` / `drop` / `store -m` | 2.43.0 ✓(`push --staged` 2.35。trailers の 3 オプションは `show` の行と同じく 2.43.0 ✓) |
| `status` | `--porcelain=v2` / `-z` / `--branch` / `-uall` / `-uno`(tracked だけを問う書き込み) | 2.43.0 ✓(v2 は 2.11) |
| `switch` | `--create` / `--force-create` / `--track` / `--no-track` | 2.43.0 ✓(2.23 導入。EXPERIMENTAL 表記は restore と同様)。**`--no-track` と `--force-create` の併用はマニュアルが明示しない**ので最低バージョンのコンテナで実測(`cargo xtask linux test -p platitude-core --test it upstream_integration`) |
| `symbolic-ref` | `-q` / `--short` | 古参(1.7.10) |
| `tag` | `--delete` / `--end-of-options` | 2.42.0 ✓ |
| `worktree` | `list --porcelain -z` | 2.42.1 ✓(`-z` 2.36) |
| `worktree` | `remove -- <path>` | 2.43.0 ✓(2.17 導入。拒否の文 `contains modified or untracked files` も 2.17 から同じ) |
| `worktree` | `add -- <path> <branch>` / `add -b <name> -- <path> <commit>` / `add --track -b <name> -- <path> refs/remotes/<remote>/<branch>` / `add --detach -- <path> <commit>`(破棄記録の復元) | 2.42.1 ✓(`-b` と `--[no-]track` の記載あり。`--detach` は 2.5 から)。2.43 のコンテナで `tests/it/worktree_add.rs` が通る。**`-b` はブランチを作ってからフォルダを見る**(物の在るフォルダ・記録だけ残るパスでも枝だけ残る)のと、**`-b` の名前を `git branch` へ `--` 無しで渡す**(`-` 始まりはオプションとして走る)のは 2.43 と 2.55 で同じ |

`--format=` のプレースホルダ(`%H %P %T %an %ae %at %aI %cn %ce %ct %cI %B %s %gd %gD %gs %G? %GS %GK %x00`)は pretty-formats 2.43.0 で全て記載確認 ✓。

## 2.43 に無いもの(使ってはいけない — 手元の git では動いてしまう)

- `cherry-pick --empty=…`(**2.45**)— `integrate::pick` は空になったコミットの停止を `--skip` で越える
- `git config get / set / unset / list`(サブコマンド形式、**2.46**)— 現行コードは旧形式で正しい
- `for-each-ref --format='%(is-base:…)'`(**2.47**)
- `rev-list -z`(**2.49**。`log -z` とは別物 — log 側は 2.43 に在る)

※ `%(ahead-behind:…)`(2.41)は 2.43 に在るが未使用。使ってよい。

## 新オプション追加時の手順

1. `https://git-scm.com/docs/git-<cmd>/<最低バージョン>.0` で記載を確認(redirect されたら近い旧版で可 — 旧版に在れば新版にも在る)
2. 個別マニュアルに無い `--no-` 形は gitcli(7) §Negating options の一般規定で判断
3. この表へ追記
