# verify-ui 各論: Linux(コンテナ)で撃つ

## Linux(コンテナ)での動確 — Done は両 OS

**`cargo xtask linux verify-ui <動詞> [引数]`** が Ubuntu 側の同じ 1 コマンド。オプションも動詞も **verbs.md の表がそのまま通る**。フォントスタックも Qt のビルドも別物で、**片方の PASS はもう片方を保証しない**。

- **反映前の gate がコンテナで回す動詞は `linux` 行だけ**(段分け表)。gate の緑は、触った動詞が Linux で通ることを言わない —— **UI を配線したら、その動詞は自分で `cargo xtask linux verify-ui` を撃つ**
- **スクショはホスト側の一時ディレクトリに出る**。パスは実行時に `screenshots and settings: <path>` として印字されるので、そこを読む。**Linux の run の shot dir は既定のまま** — Windows のパスは MSYS に書き換えられ、**カレントディレクトリの下に `C:` という名前のディレクトリが生えて**その中に絵が出る(`PASS` は出るので、`git status` に `?? "C:/"` が並ぶまで気付かない。掃除は `rm -rf "./C:"`)
- フォントは fontconfig が解決する(`QT_QPA_FONTDIR` も .ttc の罠も Windows 側の話)。**日本語はそのまま出る**。コンテナは日本語の Ubuntu デスクトップのフォントと `LANG=ja_JP.UTF-8` を積む(P5-確認事項 §実測済み)ので、**英文中の `…` `—` は和文字形**(中央の三点・細いダッシュ)で写るのが正しい
- **判定は各 OS で `screenshot saved=true` + 目視**。フォントのラスタライズが違うので 2 枚の画素は一致しないのが正常
- **どの run も自分の gitconfig で立つ**(`GIT_CONFIG_GLOBAL` + `GIT_CONFIG_NOSYSTEM=1` と**チェックアウトの外にある作業ディレクトリ**、`verify::run`)。identity 系と `badges` 系(`badges` / `badges-hover` / `badges-hover-early`)だけが自分の種を持ち、他は全部同じ fixture identity(`Verify Fixture`)。**識別できる帰結が 3 つ**: identity 未設定のコンテナでも**質問モーダルが出ない** / **帯に `SET IDENTITY` が立たない**(両 OS 同じ)/ **worktree の壊れた `.git`(Windows の絶対パスを書いたファイル)を踏む git が居ない**ので `not a git repository` の行も identity 欄の赤も出ない。**`--repo` / `--shot-dir` / `--config-dir` の相対パスは打った場所で解決される**
- **`--repo` にコンテナ内のパスを渡す時は Git Bash の変換を止める** — `--repo /work/…` は MSYS に `C:/Program Files/Git/work/…` へ書き換えられ、アプリは**開けないフォルダの画面**を撮る。そして `screenshot saved=true` は出るので **PASS する**(絵が「Not a git repository」であることだけが手掛かり)。`MSYS_NO_PATHCONV=1` を立てるか PowerShell から叩く。**`--preset` だけの run は当たらない**
- 初回だけ Qt の取得で時間がかかり、以後はキャッシュ
