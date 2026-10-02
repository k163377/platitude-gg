# コードの判断を支えた実測(Windows x64)

ソースのコメントから引き上げた値の置き場(.claude/rules/code.md)。

- 台: Ryzen 9 9900X(12C/24T)/ 32GB / Windows 11 build 26200 / git 2.55.0 /
  Qt 6.10.3。画面 3 枚(100 / 180 / 100Hz)・GPU 3 系統
  ([perf-windows-x64.md](perf-windows-x64.md) §計測条件 と同じ台。**他の仕事が載る台**)
- **1 度ずつ読んだ値で、撃ち直しの手順を持たない** — ここの行から読めるのは**順序と桁**だけ
- **数字が要る判断をするなら、その時に自分が使う量を測り直す**
- 参照リポジトリは `JetBrains/kotlin`(clone した時期で refs も tip も動く)、
  合成コーパスは `cargo xtask corpus`(token は perf 記録が持つ)

## git のプロセス代

100ms の操作応答(CLAUDE.md §性能予算)に対して、**Windows ではプロセスの起動が
コマンドの代金の大半**という 1 点がこの表の全部。

| 場所 | 読み |
|---|---|
| `process::program`(Git for Windows の `cmd\git.exe` はランチャー) | `git --version` 22.7ms 対 本体直叩き 10.3ms / details の `git show` 19.0ms 対 10.7ms(ウォーム) |
| `process::executor` の `CREATE_NO_WINDOW`(git 1 本ごとに `conhost.exe` を 1 本立てる) | `git rev-parse --show-toplevel` 200 本の直列で隠れ console 21ms/本 対 console 無し(`DETACHED_PROCESS`)12ms/本・親の console を継ぐ(フラグ無し)12ms/本 — 親が GUI(console 無し)でも console 付きでも同値。verify-ui の run の中(16 本同時)では git の spawn の中央値 139ms 対 32ms・所要の中央値 236ms 対 85ms。**console 無しは採れない**(rules-refs/core.md の `CREATE_NO_WINDOW` の行) |
| `details::commit_details` | git 自身の仕事は約 1ms、残りはプロセス |
| `repo::is_bare`(`rev-parse` 1 本) | 33ms |
| `models::tabs` の folder 検査(`rev-parse` 1 本) | 30–36ms(リポジトリでもそうでなくても) |
| `session::write` の replay(1 コミット) | 約 11ms(数百コミットの range で数秒) |
| `session::build` の carry(refused → detect → stash) | 追加 3 spawn で 100–300ms |
| `eol::attrs`(`check-attr` を 200 パスまとめて) | 1 batch 71ms 対 1 パス spawn 42ms |
| `eol::sample`(index ではなく worktree を読ませた場合) | 24.7s(106k ファイル)対 settled 数本の 42ms |
| `eol::worktree`(`ls-files --eol` は worktree ファイルを全部読む) | 120MB のファイル 1 本で 213ms |
| `conflict::tool`(`mergetool --tool-help`) | 約 8 秒(ウォーム) |
| `process::executor` の `diff.autoRefreshIndex=false` の代金 | stat が全部動いた 780 ファイルで `status` 72ms、refresh 後なら 28ms |
| `session::refresh` の refs listing | 300ms(kotlin) |
| `session::mod` / `session::read_flight` の `status --porcelain=v2 -uall` | 2.9s wall / 2.3 CPU 秒(合成コーパス = tracked 109,652 + ignored 78,000) |
| `session::query` の「動いていない snapshot を組み直して等値比較」 | 39ms / 1 コア(kotlin) |
| `session::log` の pass が walk の前に読む物(`PassReads`) | **直列に並べると最初の行までの支配項** — 7 本の直列で kotlin のタブ切替が 2.96s(他席が動いている機械。同じ 7 本を素の git で撃つと合計 0.2s = 残りは全部プロセス起動) |
| kotlin(refs 48,341 本)の素の git・ウォーム | `rev-parse --show-toplevel …` 23ms / `for-each-ref refs/remotes` 44ms / `stash list` 41ms / 全 refs の `for-each-ref` 0.88s / `status -uall` 1.15s / walk 0.14s(`--max-count` は 100 も 2000 も 15ms しか違わない = `session::log_limit` の「窓の広さは walk をほとんど動かさない」の裏) |
| `session::model` の tag 込み walk | 最初の 1 バイトまで約 +1.7s(kotlin の 44k タグ、commit-graph 有り) |
| `RefListPopup` の「測るためにもう 1 組並べる」案 | kotlin の最深行で 109ms(操作応答 100ms を単体で超える) |
| `RepoPage` の plan range 読み | 浅い click で 27ms、実履歴の根で 1 秒超(kotlin) |
| `RepoPage` の ref delete 後 | refs 読み 54ms → グラフ再構築の walk 1.3s(`busyCount` は前半だけ覆う) |

## メモリの形

300MB(CLAUDE.md §性能予算)に対する読み。**どれも「同じ答えを持つ 2 つの形」の差**で、
形を選んだ理由がこの列。

| 場所 | 読み |
|---|---|
| `session::model::LabelIndex` | `HashMap<Oid, Vec<RefLabel>>` 案は table 4.3MB + 4 枠 Vec 10.7MB で、中身の label は 2.7MB(kotlin の 47,715 ラベル付きコミット) |
| `session::RemoteTagIndex` | 名前ごとの `BTreeMap` 案 43.5MB(プロセスの Rust ヒープの 1/3)対 flat 4.4MB。per-remote の控えを別に持つ案はさらに +4.3MB(kotlin の 45,901 リモートタグ) |
| `session::joins` の `shrink_to_fit` | push で伸ばした Vec の余りはタグ 45,901 本で 1.2MB |
| `session::snapshot::BranchItem`(oid を文字列で持つ案) | 2.6MB(kotlin の 53,724 refs) |
| `models::nav::Source`(組んだ行で持つ案) | +13.9MB(kotlin) |
| `WindowDialogSeat`(設定画面と clone 箱を常時建てる案) | 素の窓に +約 8MB |
| `DiffRowDelegate` の hunk ボタン(見出し以外の行にも建てる案) | 57 行の diff で 75MB(大半は行が描かないもの) |
| `GraphLaneCell` の full-width canvas(行に 2 枚) | 42.6MB(グラフを一度スクロールさせた後の working set)。**直した後の差**(見える幅で建てる)は約 10MB(スクロールの有無によらない定数)で、別の量 |
| `GraphRowChips`(名前のある行だけに建てて recycle ごとに作り直す案) | 1 スクロールで +35MB(chip 自身ではなく作り直しのヒープ) |
| `corpus` の loose refs | 5 万本で slack 80MB、`pack-refs` 後は 6MB |
| `encode::wire`(橋の値を行の隣に持つ案) | 合成コーパスの 2,000 行の窓(レーン p50 29 本 / 行、chip 766)で WorkingSet 中央 318.9 → 377.8MB(+59)、private 296.2 → 361.7MB(+65)、settled 312.7 → 371.2MB(+58)= net が予算 300MB の外。同じ座りの fps 177.8 → 176.5、起動 1662 → 1706ms、応答は 5 run の揺れの中 — 読む時に組む形と差なし |

## 橋の値の組み立て(`encode::wire`)

QML が行を読むたびに組む値と、QML から読み戻す値(§メモリの形 の `encode::wire`)の代金。
release・Qt 6.12.0・qtbridge 0.3.0(同梱版)・rustc 1.98.1。製品の変換をそのまま回す計測
(グラフ行の 15 role を `QModelItem::get_role` で読む / `GitFacts.chipsShown` の往復)を、
順序を入れ替えた 6 プロセスで撃った 1 操作あたりの中央値。6 プロセスの中央値の範囲が重なる差は
「判定できない」と書く(等しいとは読まない)。**どれもこの版の上で、形 2 つを比べた結果**
(`key_of` は `perf(app): a record's field names are made once per thread…` の前後、読み戻しは
同じ版の上の読み方 2 つ)。依存の更新そのもの(更新前の Qt 6.10.3・qtbridge 0.2 との差)の性能は
ここからは読めない — それは P3-確認事項 §依存更新(qtbridge 0.3 / Qt 6.12)の点灯後に要る検証 の比較が答える。

| 場所 | 読み |
|---|---|
| `encode::wire::key_of`(欄名の `QString` を名前ごとに 1 度作って複製する) | グラフ行の全 role、毎回作る → 使い回す: 8 レーン・チップ 3・共著者 1 の行 11.5 → 10.0µs(和文 11.8 → 10.6µs)、40 レーン・チップ 30・共著者 10 の行 77.3 → 69.9µs。ASCII の通常行だけ 6 プロセスの範囲が重なり、差を判定できない。初回だけ確保が 4 回・約 2.5KB 増える(キャッシュの建て込み)。キャッシュを貸して `insert_clone` へ渡す形は、複製との差がどの行でも判定できない |
| 文字列の欄の読み戻し(橋の `String::from(&QString)`) | `GitFacts.chipsShown` の Rust 側(QML が渡したチップを読み・絞り・組み直す): チップ 1 / 4 / 30 個で 2.3 / 9.4–9.6 / 71–86µs。UTF-8 長を数えて 1 回で確保する読み方と比べると、ASCII・和文とも 6 プロセスの範囲が重なり時間の差は判定できない(確保は 13→12 / 40→35 / 278→238 回、和文 30 個は 308→238 回、バイトは 2〜10% 少ない)。文字列 1 つでは、その読み方は短い和文・絵文字で 0.5〜0.7 倍、ASCII 40 字で 0.8 倍、ASCII 1〜8 字は判定できず、1000 字では ASCII・和文とも 1.3〜1.4 倍遅い(2 度走査する)。橋の変換は UTF-16 長の半分の容量から伸ばすので和文で 2〜4 回確保し、出来た文字列は長さの最大 1.33 倍の容量を持つ。`toUtf8`(`QByteArray` 経由)は短い文字列で 1.6 倍遅く、長い ASCII でだけ速い。時間は計数アロケータ無し、回数は有りのビルドで測った |

## 着色(`highlight`)

| 場所 | 読み |
|---|---|
| `highlight::patch::LEX_LINE_BUDGET` = 5,000 | lexer は約 16,000 行/秒(release、このリポジトリのソース)= 約 300ms |
| `highlight::patch::QUICK_LINE_BUDGET` = 1,000 | 同じ速度で約 60ms |
| 予算を置かない読み | 2 万行の全書き換えで 1.2s |
| `DiffColoured` を行と一緒に送る案 | 6,000 行の Rust で 951ms(同じテキストを「何も言えない名前」で読ませると 3ms)= 操作応答の 100ms の外 |
| `models::diff` の行の作り直し | 6,000 行で 2ms(色が届いた時に全行を組み直せる根拠) |

## 操作パネルのブランチのカード(`OpsBranchMenu`)

| 場所 | 読み |
|---|---|
| 押下でカードの木を全部作る形 | ローカルブランチ 1,001 本(40 フォルダ)で 1,115–1,168ms、数本の preset `panel` で 20ms(`ops-branch`、開く呼び出しの前後を測った 2 run)= **行の部品を作る代金が支配項**で、1 本あたり約 1ms |
| 1 段ずつ作る形(`NavSectionModel.cardLevel` + 開いた時に中身を作るフォルダのカード) | 同じ 1,001 本で 60ms、`panel` で 16ms。押下が作るのは最上段の行だけ |

## todo の突き合わせ(`sequencer::merge_todo`)

プランの行数 = 書き換える範囲の長さなので、履歴全部を `rebase -i` に載せた時が上限(release、短縮 9 桁・10 行に 1 本の `update-ref`)。

| 行数 | 並び | 走査 + `Vec::remove` | oid 索引 |
|---|---|---|---|
| 40,000 | 元のまま | 474ms | 8.0ms |
| 40,000 | 逆順 | 1.260s | 8.8ms |
| 80,000 | 元のまま | 3.13s | 18.0ms |
| 80,000 | 逆順 | 5.12s | 17.7ms |
| 138,915 | 元のまま | 10.20s | 37.6ms |
| 138,915 | 逆順 | 16.85s | 36.1ms |

**並べ替えると走査側だけが倍近く払う**。索引側は並びを見ない。

## コーパス生成(`cargo xtask corpus`)

`corpus.rs` / `corpus::shape` の定数を決めた読み。

| 対象 | 読み |
|---|---|
| blob を 1 プロセスで流す(body を全部綴った 25GB のストリーム) | 596s。同じコミットを 300 バイト body で流すと 200s、生成器がストリームを書くだけなら 20s(同じ台・同じ時間帯) |
| `--depth=0`(delta を試させない) | 446s 対 596s だが、パックは 10.7GiB 対 6.5GiB |
| 大きいファイルの履歴を持たない corpus | パックは 3.52GiB 対 4.66GiB |
| 単列(側枝なし)の corpus | 95MB 軽く出る = 全部の数字を良く見せる |
| ignore をディレクトリ名で書く | `status` 0.44s 対 パターンの 1.01s |
| `core.fsmonitor`(kotlin 実物) | off 0.28s / on 0.76s = この台ではデーモンが払わせる側 |
| `--no-optional-locks`(タイミング装置ではない) | 有無どちらも 0.76s |
| `demo --preset deep` を 1 コミットずつ書く案 | 2,100 プロセスで約 1 分。1 本の `fast-import` なら 92ms |

## テストとハーネス

| 対象 | 読み |
|---|---|
| `cargo test --workspace` 下の git 1 往復(コアごとにスレッド、全部が git を spawn) | 単独時の約 25 倍。**壁時計の上限は負荷で判定を決める**ので、テストのハーネスは stock timeout を外して自前の backstop を持つ |
| `perf` の warm 判定(`perf::warmth`) | invocation 1 本目の 1 run 目だけ突出(startup 1226ms)、2 本目以降の 1 run 目は採用 run(1096–1178ms)に混ざる(1081–1177ms)= 毎回 1 run 捨てるのは 14 秒の無駄 |
| `xtask::gui` の起動見張り | Qt プラットフォームプラグインの失敗はほぼ即死(約 10ms)。`FIRST_MOMENT` の残りはコールドスタートの余白 |
| `platitude-app` の `qrc::embed!`(QML のディスクキャッシュ) | 時刻 0 の qrc(置き場を渡されない起動 = 製品)は起動のたびに 268 ファイルをコンパイルし、そのログはプロセス時刻 0.07s → 0.77s に並ぶ(`app-menu` の run 全体 1.4s)。キャッシュが効いた時のアプリの CPU 時間は [gate-load-windows-x64.md](gate-load-windows-x64.md) §git の console と QML のキャッシュの代金、私有メモリは約 220MB のまま動かない。キャッシュ置き場を渡された起動は両モジュールぶん(`ui/` 2.7MB + `auto/` 1.1MB のファイル)を私有メモリに持つ |
| verify-ui の 32 run(動詞 8 種 × 4、`--no-build`)の置き場(`pgg-demo` の fixture のコピーと `pgg-verify` の shot dir)を `%TEMP%` から席の `target\` = Defender の除外の内へ(交互に 2 巡、全 run PASS) | 壁時計 幅 8: 28.5 / 18.8s → 30.8 / 24.2s・幅 16: 19.6 / 18.9s → 18.7 / 16.9s。**同じ置き場の 2 巡の差(最大 1.5 倍)の方が置き場の差より大きい = 置き場では速くならない**。run の代金の 9 割以上はアプリの側 |
| demo の雛形(`demo::template`) | 雛形を組む run の fixture は `app-menu`(`basic`)で約 2.0–2.2s、雛形のコピーは約 0.1s。全 preset の雛形を組む gate は、雛形が立っている gate より host の動詞平均 9.1–12.4s 対 6.7–6.8s・fixture が 3s を超える run 74–78 本 対 4–6 本・壁時計 12m52s–13m39s 対 8m52s–8m57s(同じ席の `gate --fresh`) |

## build directory の世代(`xtask::sweep`)

cargo が置き換えた成果物を消さないことの代金。**1 世代 = 席 1 つが正規集合を建てた時の量**で、そこから先は `Cargo.lock` / rustc / profile が動くたびに同じだけ積む。

| 対象 | 読み |
|---|---|
| 席 1 つの `target/`(6 席の合計) | 148GB(51 / 47 / 15 / 12 / 12 / 5)。1 世代は 12〜15GB |
| その内訳(積もった席の `target/debug` 42.7GB) | `incremental` 24.7GB(設定 250 個、生きているのは十数個)/ `deps` 12.8GB(外部 crate の rlib が典型 6 世代・最大 14 世代、テスト exe が 7〜9 世代)/ `build` 5.2GB(`qtbridge-type-lib` の出力 146MB × 8 世代) |
| Linux 側のボリューム `pgg-linux-target-<席>`(空から gate を 1 本回した 1 世代) | 3.8GB = `debug/incremental` 1.5 + `debug/deps` 0.96 + `debug/build` 0.53 + `release` 0.83。debug の debuginfo は自前のクレートの行テーブルだけ(イメージの `/usr/local/cargo/config.toml`)。テスト exe `it` は 119MB・`libplatitude_core` の rlib は 105MB(full debuginfo の時は 250MB / 262MB) |
| コンテナの incremental | 持つ。1 世代の 1.5GB がそれで、持たないと core を touch した後の `linux test -p platitude-core --no-run` が 5s → 10.5s(2 回ずつ交互) |
| incremental セッションの日付 | `<crate>-<id>/` の 2 段下の最新ファイルが、そのディレクトリ自身の日付と秒まで一致(40 件中 39 件。残り 1 件は 1 秒差)。artifact との差は ±5 秒 |
| fresh なビルドが `.fingerprint/*/invoked.timestamp` に触るか | 触らない(`target/hooks` で hook が朝から数百回走っても、時刻は最後の再ビルドのまま)= **古さは「死んでいる」の証拠にならない** |
| 正規集合を続けて 2 回読んだ時に relink する unit(listing 1 回あたり) | 2。どちらも `pgg-todo-editor` — feature 解決の違う 2 行(`test --locked --workspace --no-run` と `test --locked -p platitude-core --no-run`)が hash 無しの同じ bin を書くので、listing 1 回につき 1 回ずつ相手を上書きする。他の 15 行は全部 fresh |

## コンテナのイメージと build cache(`xtask::linux`)

イメージの tag は入力(Dockerfile / rust-toolchain.toml / ci.yml)の指紋なので、それが動くたびに
**1 世代まるごと**積む。**何が何を掴んでいるかを決めるのは tag で、cache の上限ではない**。

| 対象 | 読み |
|---|---|
| `docker builder prune --max-used-space` が数える範囲 | **image が共有していない記録だけ**。未参照 6.328GB に上限 6.0GB を当てて消えたのは 11 日前の 616.3MB 1 本で、shared 6.38GB は不動(buildkit v0.33 `cache/manager.go`: 総和は `if ui.Shared { continue }` の後で、shared は削除候補に入らない)。wslc の VM の docker 25 は同じ上限を `--keep-storage` と綴り、数え方は同じ(buildkit v0.12.5 の同じ行) |
| イメージ(wslc が数える大きさ) | core 1.08GB / app 1.70GB / runtime 0.66GB。app の残りの大物は Rust のツールチェーン 646MB・gcc 系・Qt・Noto Sans CJK(太さ違いを含む)・llvmpipe の libLLVM 140MB で、ベースの ubuntu:24.04 は 78MB |
| app から落とした物 | Qt の静的ライブラリ(`lib/*.a` 1.38GB。QML の language server と DOM が 1.08GB — アプリのリンクは Qt の共有ライブラリだけ)/ qttools・qtwayland・qttranslations・qtdoc のアーカイブ(`qdoc` だけで 79MB)と sbom 34MB / 明朝体の CJK 179MB / Qt を入れるためだけの Python。**前後のイメージで同じビルドを撮った 8 動詞 18 枚の PNG がバイト一致** |
| app に残した物 | `fonts-noto-cjk-extra` 214MB(Sans の太さ違い)。無いと Font.DemiBold が Medium でなく Bold で描かれる(13px の同じ文で幅 246.3 → 250.2)= 絵が変わる |
| build cache 1 世代(wslc) | image と共有 2.6GB(core / app / runtime が建った時点の `docker buildx du`) |
| `--rebuild` を cache 全ヒットで撃った代金 | core の unique size が 7.17kB → 1.376GB。**再 export が新しい層の digest を作り**、app は古い core の層を持ったままなので 1 世代分が二重になる。動作確認の手段には使えない |
| 同じ入力から建った app の tag 2 本 | unique size は各 2.402kB(shared 4.295GB)。**片方を消しても戻るのは kB で、4.3GB は残る側へ移るだけ** |
