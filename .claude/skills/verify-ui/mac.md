# verify-ui 各論: macOS(CI の runner)で撃つ

## macOS での動確 — 机が無いので CI の runner が撃つ

mac の実機は無い。**mac の絵と判定は GitHub Actions の `shots.yml` が出す** — runner(`macos-26` = arm64)の上で `cargo xtask replay` が verify-ui の行を 1 本ずつ撃ち、絵とログを artifact `shots-macos` に残す。**判定は手で撃った run と同じ**(`screenshot saved=true` + 報告行)で、行の書き方も **verbs.md の表がそのまま通る**。

- **いつ撃たれるか**: main への push = 段分け表の `linux` 行(判定がフォントで決まる行)/ 週 1 = census の全行(public のリポジトリで 60 日活動が無いと GitHub が schedule を止める — Actions のページから戻す)/ **名指しは dispatch**:

  ```bash
  gh workflow run shots.yml --ref main -f lines='band; nav-tip branch:0'
  ```

  `lines` は `verify-ui` の後ろに書く行そのものを `;` で並べる。`-f tier=linux`(`merge` / `all`)は census の行を足し、`-f locale=en` は英語のシステムで撃つ(既定は日本語)。行も tier も名指さない dispatch は job ごと skip される。**`--ref` に書けるのは GitHub に在る枝だけ** — 席の枝は GitHub に無く、出すかどうかはユーザーが決める
- **結果を読む**: run のページの Summary に行ごとの `ok` / `FAIL` が並ぶ(赤が先頭。ビルドが赤くて撃てなかった行は末尾に名指される)。天井(270 分)で切れた run は Summary が出ない — 行ごとの `ok` / `FAIL` は Replay の step のログ、そこまでの絵は artifact に在る。絵は artifact を席の `target/` へ落として読む:

  ```bash
  gh run list --workflow shots.yml --limit 5
  ```

  ```bash
  gh run download <run-id> --name shots-macos --dir target/ci-shots/<run-id>
  ```

  中身は `summary.md`、行ごとの `<番号>-<行>.log` と `<番号>-<行>/app.png`(開いたものがあれば `overlay.png` / `scene.png`)。**ユーザーに見せる絵は board へ**(`cargo xtask shots add`)— ラベルの末尾を `— mac` にすると Windows / Linux の run を置き換えない
- **写るのは offscreen の絵まで** — フォントは CoreText(Hiragino Sans / Menlo。`QT_QPA_FONTDIR` は Windows の話)、描画は Windows と同じ software scene graph。**Cocoa の窓(タイトルバー・ネイティブメニュー・Dock)は写らない**(P5-確認事項 §3.6)
- **言語は runner のシステム設定で決まる** — Qt は macOS では `LANG` ではなくシステムの言語を読む。英文中の `—` `…` と等幅の和文フォールバックを判定する時は、**日本語と英語の両方で撃って並べる**
- **赤を見たら先に机で同じ行を撃つ** — `cargo xtask replay --line '<行>'` は机でも同じ道を通る(絵は `target/replay/`、census も board も動かない)。机で緑なら mac の答えが違う — 絵で「実際に切れている」か「判定の閾がフォントに依っている」かを分ける
- **1 run = runner 1 台**(release ビルド + 行数ぶん)。public の間は無料で、同時に走れる macOS の job は 5 本まで
