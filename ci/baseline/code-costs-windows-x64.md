# コードの判断を支えた実測(Windows x64)

ソースのコメントから引き上げた値の置き場。**値を持つのはこの表**(CLAUDE.md
§Rust 規約)— 機械が変われば当然、同じ機械でも負荷で動く数字は、条件を書ける
記録の側に置く。コメントに残すのは「何が支配項か」「どちらが桁で大きいか」で、
その桁を決めた読みがこの表。

- 台: Ryzen 9 9900X(12C/24T)/ 32GB / Windows 11 build 26200 / git 2.55.0 /
  Qt 6.10.3。画面 3 枚(100 / 180 / 100Hz)・GPU 3 系統
  ([perf-windows-x64.md](perf-windows-x64.md) §計測条件 と同じ台。**他の仕事が載る台**)
- **1 度ずつ読んだ値で、撃ち直しの手順を持たない** — [perf-windows-x64.md](perf-windows-x64.md) /
  [poll-cost](poll-cost-windows-x64.md) / [head-reach](head-reach-windows-x64.md) /
  [refs-join](refs-join-windows-x64.md) の 4 本(条件を書いて撃ち直せる記録)とは別物。
  ここの行から読めるのは**順序と桁**だけ
- **数字が要る判断をするなら、その時に自分が使う量を測り直す**。この表は
  「なぜ今の形なのか」を読むためだけにある
- 参照リポジトリは `JetBrains/kotlin`(clone した時期で refs も tip も動く)、
  合成コーパスは `cargo xtask corpus`(token は perf 記録が持つ)

## git のプロセス代

100ms の操作応答(CLAUDE.md §性能予算)に対して、**Windows ではプロセスの起動が
コマンドの代金の大半**という 1 点がこの表の全部。

| 場所 | 読み |
|---|---|
| `process::program`(Git for Windows の `cmd\git.exe` はランチャー) | `git --version` 22.7ms 対 本体直叩き 10.3ms / details の `git show` 19.0ms 対 10.7ms(ウォーム) |
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
| `session::model` の tag 込み walk | 最初の 1 バイトまで約 +1.7s(kotlin の 44k タグ、commit-graph 有り) |
| `session::head_reach` / `reachable` の tag 抜き | tag は refs 53,672 のうち 45,846、walk 501ms のうち 478ms(→ [head-reach](head-reach-windows-x64.md)) |
| `RefListPopup` の「測るためにもう 1 組並べる」案 | kotlin の最深行で 109ms(操作応答 100ms を単体で超える) |
| `RepoPage` の plan range 読み | 浅い click で 27ms、実履歴の根で 1 秒超(kotlin) |
| `RepoPage` の ref delete 後 | refs 読み 54ms → グラフ再構築の walk 1.3s(`busyCount` は前半だけ覆う) |

## メモリの形

300MB(CLAUDE.md §性能予算)に対する読み。**どれも「同じ答えを持つ 2 つの形」の差**で、
形を選んだ理由がこの列。

| 場所 | 読み |
|---|---|
| `session::model::LabelIndex` | `HashMap<Oid, Vec<RefLabel>>` 案は table 4.3MB + 4 枠 Vec 10.7MB で、中身の label は 2.7MB(kotlin の 47,715 ラベル付きコミット) |
| `session::RemoteTagIndex` | 名前ごとの `BTreeMap` 案 43.5MB(プロセスの Rust ヒープの 1/3)対 flat 4.4MB。per-remote の控えを別に持つ案は名前をもう 1 組持つので さらに +4.3MB(kotlin の 45,901 リモートタグ) |
| `session::joins` の `shrink_to_fit` | push で伸ばした Vec の余りはタグ 45,901 本で 1.2MB |
| `session::snapshot::BranchItem`(oid を文字列で持つ案) | 2.6MB(kotlin の 53,724 refs) |
| `models::nav::Source`(組んだ行で持つ案) | +13.9MB(kotlin) |
| `WindowDialogSeat`(設定画面と clone 箱を常時建てる案) | 素の窓に +約 8MB |
| `DiffRowDelegate` の hunk ボタン(見出し以外の行にも建てる案) | 57 行の diff で 75MB(大半は行が描かないもの) |
| `GraphLaneCell` の full-width canvas(行に 2 枚) | 42.6MB(グラフを一度スクロールさせた後の working set)。**直した後の差**(見える幅で建てる = 約 10MB)は [rules-refs/app-ui.md](../../.claude/rules-refs/app-ui.md) の `Canvas` の行が持つ — 別の量なので両方を読む |
| `GraphRowChips`(名前のある行だけに建てて recycle ごとに作り直す案) | 1 スクロールで +35MB(chip 自身ではなく作り直しのヒープ) |
| `corpus` の loose refs | 5 万本で slack 80MB、`pack-refs` 後は 6MB |
| `encode::wire`(橋の値を行の隣に持つ案 = 組んだ時に `QVariant` を 1 度作り、読みは共有ハンドルのコピー) | 合成コーパスの 2,000 行の窓(レーン p50 29 本 / 行、chip 766)で WorkingSet 中央 318.9 → 377.8MB(+59)、private 296.2 → 361.7MB(+65)、settled 312.7 → 371.2MB(+58)= net が予算 300MB の外。同じ座りの fps 177.8 → 176.5、起動 1662 → 1706ms、応答は 5 run の揺れの中 — 読む時に組む形と差なし。型付き化だけ(読む時に組む)は main と同じ 319.8MB。**読む時に組む最終形と main の ABBA**(`--compare`、5 ブロック = 各 10 サンプル、固定ケース 2 本 × 3 周): WorkingSet 中央 320.0 対 316.8(+3.3、5 ブロック中 4 で上 — 5 run の散り 2〜10MB の内側で、見えているデリゲートが持つ JS オブジェクトぶんが上限)、private 295.8 対 292.7、fps 177.5 対 177.7、起動 1733 対 1762ms、details / diff の p50 は 2 ケース × 2 点で上下が混在 = 差なし(2026-09-21、rig の `0bfa34f5` / `3b6fe627` / `d751706e` / `fb67ecd8`。corpus token `fb3d090c` = 作業コピーの refs 8 本が立った状態で、判定記録の座りとは別。起動と walk がその分遅い) |

## 着色(`highlight`)

| 場所 | 読み |
|---|---|
| `highlight::patch::LEX_LINE_BUDGET` = 5,000 | lexer は約 16,000 行/秒(release、このリポジトリのソース)= 約 300ms |
| `highlight::patch::QUICK_LINE_BUDGET` = 1,000 | 同じ速度で約 60ms |
| 予算を置かない読み | 2 万行の全書き換えで 1.2s |
| `DiffColoured` を行と一緒に送る案 | 6,000 行の Rust で 951ms(同じテキストを「何も言えない名前」で読ませると 3ms)= 操作応答の 100ms の外。**同じ 2 つの数が [rules-refs/app-ui.md](../../.claude/rules-refs/app-ui.md) の diff の feed の行にも在る**(そちらは応答が要求順に返らない話)— 撃ち直したら両方を書き換える |
| `models::diff` の行の作り直し | 6,000 行で 2ms(色が届いた時に全行を組み直せる根拠) |

## todo の突き合わせ(`sequencer::merge_todo`)

プランの行数 = 書き換える範囲の長さなので、履歴全部を `rebase -i` に載せた時が上限。**同じ入力で新旧を並べて計った**(release、短縮 9 桁・10 行に 1 本の `update-ref`。出力は 6 通りとも byte 一致)。

| 行数 | 並び | 走査 + `Vec::remove` | oid 索引 |
|---|---|---|---|
| 40,000 | 元のまま | 474ms | 8.0ms |
| 40,000 | 逆順 | 1.260s | 8.8ms |
| 80,000 | 元のまま | 3.13s | 18.0ms |
| 80,000 | 逆順 | 5.12s | 17.7ms |
| 138,915 | 元のまま | 10.20s | 37.6ms |
| 138,915 | 逆順 | 16.85s | 36.1ms |

**並べ替えると走査側だけが倍近く払う**(取った行が前から消えるので、後ろへ動いた行ほど毎回長く走る)。索引側は並びを見ない。

## コーパス生成(`cargo xtask corpus`)

`corpus.rs` / `corpus::shape` の定数を決めた読み。**測っているのは
生成時間**。

| 対象 | 読み |
|---|---|
| blob を 1 プロセスで流す(body を全部綴った 25GB のストリーム) | 596s。同じコミットを 300 バイト body で流すと 200s、生成器がストリームを書くだけなら 20s(同じ台・同じ時間帯) |
| `--depth=0`(delta を試させない) | 446s 対 596s だが、パックは 10.7GiB 対 6.5GiB |
| 大きいファイルの履歴を持たない corpus | パックは 3.52GiB 対 4.66GiB |
| 単列(側枝なし)の corpus | 95MB 軽く出る = 全部の数字を良く見せる |
| ignore をディレクトリ名で書く | git はディレクトリを 1 回 stat して枝を切るので `status` 0.44s 対 パターンの 1.01s |
| `core.fsmonitor`(kotlin 実物) | off 0.28s / on 0.76s = この台ではデーモンが払わせる側 |
| `--no-optional-locks`(タイミング装置ではない) | 有無どちらも 0.76s |
| `demo --preset deep` を 1 コミットずつ書く案 | 2,100 プロセスで約 1 分。1 本の `fast-import` なら 92ms |

## テストとハーネス

| 対象 | 読み |
|---|---|
| `cargo test --workspace` 下の git 1 往復(コアごとにスレッド、全部が git を spawn) | 単独時の約 25 倍。**壁時計の上限は負荷で判定を決める**ので、テストのハーネスは stock timeout を外して自前の backstop を持つ |
| `perf` の warm 判定(`perf::warmth`) | invocation 1 本目の 1 run 目だけ突出(startup 1226ms)、2 本目以降の 1 run 目は採用 run(1096–1178ms)に混ざる(1081–1177ms)= 毎回 1 run 捨てるのは 14 秒の無駄 |
| `xtask::gui` の起動見張り | Qt プラットフォームプラグインの失敗はほぼ即死(約 10ms)。`FIRST_MOMENT` の残りはコールドスタートの余白 |

## build directory の世代(`xtask::sweep`)

cargo が置き換えた成果物を消さないことの代金。**1 世代 = 席 1 つが正規集合を建てた時の量**で、そこから先は `Cargo.lock` / rustc / profile が動くたびに同じだけ積む。

| 対象 | 読み |
|---|---|
| 席 1 つの `target/`(6 席の合計) | 148GB(51 / 47 / 15 / 12 / 12 / 5)。1 世代は 12〜15GB |
| その内訳(積もった席の `target/debug` 42.7GB) | `incremental` 24.7GB(設定 250 個、生きているのは十数個)/ `deps` 12.8GB(外部 crate の rlib が典型 6 世代・最大 14 世代、テスト exe が 7〜9 世代)/ `build` 5.2GB(`qtbridge-type-lib` の出力 146MB × 8 世代) |
| Linux 側のボリューム `pgg-linux-target-<席>`(積もった席) | 11.3GB = `debug/incremental` 4.8 + `debug/deps` 2.6 + `debug/build` 2.0 + `release` 1.4。6 席で 56GB、1 世代は 6GB |
| incremental セッションの日付 | `<crate>-<id>/` の 2 段下の最新ファイルが、そのディレクトリ自身の日付と秒まで一致(40 件中 39 件。残り 1 件は 1 秒差)。artifact との差は ±5 秒 |
| fresh なビルドが `.fingerprint/*/invoked.timestamp` に触るか | 触らない(`target/hooks` で hook が朝から数百回走っても、時刻は最後の再ビルドのまま)= **古さは「死んでいる」の証拠にならない** |
| 正規集合を続けて 2 回読んだ時に relink する unit(listing 1 回あたり) | 2。どちらも `pgg-todo-editor` — feature 解決の違う 2 行(`test --locked --workspace --no-run` と `test --locked -p platitude-core --no-run`)が hash 無しの同じ bin を書くので、listing 1 回につき 1 回ずつ相手を上書きする。他の 15 行は全部 fresh |

## コンテナのイメージと build cache(`xtask::linux`)

イメージの tag は入力(Dockerfile / rust-toolchain.toml / ci.yml)の指紋なので、それが動くたびに
**1 世代まるごと**積む。**何が何を掴んでいるかを決めるのは tag で、cache の上限ではない**。

| 対象 | 読み |
|---|---|
| `docker builder prune --max-used-space` が数える範囲 | **image が共有していない記録だけ**。未参照 6.328GB に上限 6.0GB を当てて消えたのは 11 日前の 616.3MB 1 本で、shared 6.38GB は不動(buildkit v0.33 `cache/manager.go`: 総和は `if ui.Shared { continue }` の後で、shared は削除候補に入らない) |
| build cache 1 世代 | 6.38GB(Qt install 1.821 + bare への Qt 複写 1.519 + toolchain 0.847 + apt 群 2.1 + 端数)。世代が 1 つ死ぬと同じだけ未参照として残る |
| `--rebuild` を cache 全ヒットで撃った代金 | core の unique size が 7.17kB → 1.376GB。**再 export が新しい層の digest を作り**、app は古い core の層を持ったままなので 1 世代分が二重になる。動作確認の手段には使えない |
| 同じ入力から建った app の tag 2 本 | unique size は各 2.402kB(shared 4.295GB)。**片方を消しても戻るのは kB で、4.3GB は残る側へ移るだけ** |
