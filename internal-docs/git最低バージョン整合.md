# 最低 git バージョン整合チェック

アプリが発行する全 git コマンド・オプションが、対応最低バージョン([要望.md](要望.md) §その他 — 照合時点で **2.43**)に存在するかの論理チェック記録。
実バイナリでの実測ではなく**マニュアル照合**(git-scm.com のバージョン付きマニュアル + git.git の v タグ付きドキュメントソース)。実バイナリでの一巡は [P5-確認事項.md](P5-確認事項.md) §4。

- 洗い出し: `platitude-core/src` の `GitCommand` 構築の全数(`-c` 固定引数・環境変数・pathspec magic 含む)。app 側に git 実行は無い(core に集約)ことも確認済み
- 照合日: 2026-08-06(コード状態: main 43f02c8)
- 判定基準: 2.43.0 のマニュアルに記載があれば ✓。git-scm.com が旧版へ redirect する場合は旧版で確認(旧版に在れば 2.43 にも在る — git がオプションを削除することは極めて稀)。2.30 より前からある古参は導入バージョンの知見で確定し「古参」と記す

## 結論

**発行する全コマンド・全オプションが 2.43 に存在する。** 個別マニュアルに無いのは `cherry-pick --no-edit` 1 件のみで、gitcli(7) の「long option は `--no-` で否定できる」一般規定(§Negating options)+ 実測(CLAUDE.md の実測注記)でカバーされる。最も新しい依存は `rebase --update-refs`(2.38 — 要望.md が明示的に前提化)。

**マニュアル照合が拾えない例外が 1 つある — `reset` は `--end-of-options` を受け付けない。** gitcli(7) の一般規定に載っていても、`git reset` は 2.43 で位置によらず `fatal: option '--end-of-options' must come before non-option arguments`(exit 128)を返す。アプリが発行する動詞のうちこれだけで、`branch` / `switch` / `tag` / `remote add` / `remote set-url` / `rev-parse --verify --quiet` / `log -1 --format=` / `stash list` は全て通る(2.43 実測)。名前を渡したい経路が将来できたら、`rev-parse --end-of-options` で解決してから object id を渡す。**この差は開発機の git(新しい版は受け付ける)では出ない** — 最低バージョンを積んだ Linux コンテナ(`cargo xtask linux`)が唯一の検出点。

**同梱物にもバージョン差がある。** git が持つのはマージツールの**起動レシピだけ**(`$(git --exec-path)/mergetools/` の数十行のシェル。ツール本体は利用者が入れる)で、その顔ぶれが版で変わる — **2.43.0 は 23 個で `vscode` を含まない**(追加は 2.47.0 = `6b77283f mergetools: vscode: new tool`)。`smerge` は 2.22 から在る。**アプリはツール名を利用者から受け取るだけで候補を持たない**ので現状これに依存しないが、将来「既定を提案する」を作るなら名前を書くだけでは 2.43 で外れる(`mergetool.vscode.cmd` を自前で書く形になる)。なお Git for Windows は vim を同梱するため `vimdiff` 系だけは常に「利用可能」と表示されるが、`CREATE_NO_WINDOW` のため**このアプリからは起動できない**。

## 毎回付くもの(process.rs)

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

## サブコマンド別

| コマンド | 使用オプション | 確認 |
|---|---|---|
| `--version` | — | 古参 |
| `add` | `--all` / `--intent-to-add` | 古参(1.6.1) |
| `apply` | `--cached` / `--reverse` / `--whitespace=nowarn` | 古参 |
| `branch` | `-d` `-D` `-m` `-M` / `--set-upstream-to=` / `--end-of-options` | 古参(1.8.0)+ gitcli 2.43.0 ✓ |
| `cat-file` | `-s` / `blob` / `commit` | 古参 |
| `check-attr` | `-z` / 属性名の並び / `--` | 2.43.0 ✓。**最低バージョンのコンテナで実測**(`cargo xtask linux verify-ui diff-file --preset eol`)— `path\0attr\0value\0` の三つ組で返る |
| `checkout` | `--ours` / `--theirs`(conflict 用) | 古参(1.6.1) |
| `cherry-pick` | `--no-edit` / `--continue` `--abort` `--skip` `--quit` | 2.39.3 ✓。**`--no-edit` のみ個別マニュアル外** → gitcli §Negating options でカバー |
| `clean` | `-f` `-d` | 古参 |
| `commit` | `--amend` / `--allow-empty` / `--reset-author` / `--no-edit` / `--cleanup=whitespace` / `--file` | 2.43.0 ✓ |
| `commit-tree` | `-p` / `-m` | 古参(1.7.6) |
| `config` | `--get` / `--get-regexp` / `-z` / `--global` / 書き込みは旧形式 `config <key> <value>` | 古参 |
| `diff` | `--cached` / `--no-ext-diff` / `--find-renames` / `--name-status` / `-z` / `--no-index` | 古参(`--no-index` 1.5.1) |
| `diff-tree` | `-r` / `--no-commit-id` / `-z` / `--name-status` / `--find-renames` / `--root` / `--no-ext-diff` | 2.43.0 ✓ |
| `fetch` | `--prune` / `--all` | 古参(1.6.5 / 1.6.6) |
| `for-each-ref` | `--format=`(`refname` `objecttype` `objectname` `*objectname` `upstream` `HEAD` `creatordate:unix`) | 2.43.0 ✓(`:unix` は `--date=unix` 委譲、git-log 2.43.0 ✓) |
| `ls-files` | `-z` / `--eol` / pathspec | `--eol` は 2.8。**最低バージョンのコンテナで実測**(同上)— 列は `i/<v>  w/<v>  attr/<v>\t<path>`、未チェックアウトは `w/` が空 |
| `log` | `-z` / `-1` / `--date-order` / `--branches` `--remotes` `--tags` / `--max-count=` / `--ignore-missing` / `--reverse` / `--format=` / `--end-of-options` | 2.43.0 ✓。`-z` は diff-options.txt の `ifdef::git-log`「Separate the commits with NULs instead of newlines」を v2.43.0 ソースで確認。`--date-order` は古参(1.5 系から rev-list-options.txt に在る) |
| `merge` | `--no-edit` / `--no-ff` / `--ff-only` / `--squash` / `--continue` `--abort` `--quit` | 2.43.0 ✓ |
| `merge-base` | `--is-ancestor` | 古参(1.8.0) |
| `mergetool` | `--no-prompt` / `--gui` / `--tool=` / `--tool-help` | 2.43.0 ✓(`--gui` は guitool → tool のフォールバックまで記載を確認) |
| `config` | `--get-regexp` に `^mergetool\..*\.cmd$`(自前定義ツールの列挙) | 古参 |
| `-c` | `mergetool.writeToTemp=true`(mergetool 実行時) | 2.43.0 ✓(`keepBackup` / `guiDefault` / `hideResolved` も同じ config 文書に在る) |
| `push` | `--porcelain` / `--set-upstream` / `--force-with-lease=<ref>:<oid>` / `--force` / `--delete` | 2.43.0 ✓(値付き lease 形式まで記載確認) |
| `rebase` | `--interactive` / `--onto` / `--root` / `--update-refs` / `--continue` `--abort` `--skip` `--quit` | 2.43.0 ✓(`--update-refs` 2.38 = **最も新しい依存**)。`--autostash` は発行しない(規約 §未コミット変更がある状態で履歴を書き換える) |
| `reset` | `--quiet` / `--soft` `--mixed` `--hard` | 古参。**`--end-of-options` は付けられない**(下記) |
| `restore` | `--staged` / `--worktree`(併用) | 2.43.0 ✓(2.23 導入。2.43 時点 EXPERIMENTAL 表記 — 存在と記載は確認済) |
| `rev-list` | `--count` / `--merges` / `--not` / `--remotes` | 古参(1.7.2) |
| `rev-parse` | `--verify` / `-q` / `--git-path` / `--show-toplevel` / `--absolute-git-dir` / `--show-object-format` / `--end-of-options` / `<rev>^{commit}` | 2.43.0 ✓ |
| `revert` | `--no-edit` | 2.43.0 ✓ |
| `rm` | `--cached` / `-r` / `-f` / `--quiet` | 古参 |
| `show` | `--no-patch` / `-z` / `--format=` / `--format=%(trailers:key=,valueonly,unfold,separator=)` | 古参(`--no-patch` 1.8.4)。**trailers の 4 オプションとも 2.43.0 ✓**(pretty-formats に `key=<key>`「Matching is done case-insensitively」= `Co-Authored-By` も拾う・`valueonly`・`unfold`・`separator=<sep>`「may contain the literal formatting codes described above」= `%x1F` が書ける、を確認) |
| `stash` | `list -z --format=` / `push -m --include-untracked --keep-index --staged` / `pop --index` / `apply` / `drop` / `store -m` | 2.43.0 ✓(`push --staged` 2.35) |
| `status` | `--porcelain=v2` / `-z` / `--branch` / `-uall` | 2.43.0 ✓(v2 は 2.11) |
| `switch` | `--create` / `--force-create` / `--track` | 2.43.0 ✓(2.23 導入。EXPERIMENTAL 表記は restore と同様) |
| `symbolic-ref` | `-q` / `--short` | 古参(1.7.10) |
| `tag` | `--delete` / `--end-of-options` | 2.42.0 ✓ |
| `worktree` | `list --porcelain -z` | 2.42.1 ✓(`-z` 2.36) |

`--format=` のプレースホルダ(`%H %P %T %an %ae %at %aI %cn %ce %ct %cI %B %s %gd %gs %G? %GS %GK %x00`)は pretty-formats 2.43.0 で全て記載確認 ✓。

## 2.43 に無いもの(使ってはいけない — 手元の git では動いてしまう)

- `git config get / set / unset / list`(サブコマンド形式、**2.46**)— 現行コードは旧形式で正しい
- `for-each-ref --format='%(is-base:…)'`(**2.47**)
- `rev-list -z`(**2.49**。`log -z` とは別物 — log 側は 2.43 に在る)

※ `%(ahead-behind:…)`(2.41)は 2.43 に在るが未使用。使ってよい。

## 新オプション追加時の手順

1. `https://git-scm.com/docs/git-<cmd>/<最低バージョン>.0` で記載を確認(redirect されたら近い旧版で可 — 旧版に在れば新版にも在る)
2. 個別マニュアルに無い `--no-` 形は gitcli(7) §Negating options の一般規定で判断
3. この表へ追記
