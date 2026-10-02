# Linux コンテナの VM のメモリ(Windows x64)

`cargo xtask linux` のエンジンは WSL 3.0.1 の `wslc`。コンテナは全部 1 つのセッション(`wslc-cli-<ユーザー>`)の
VM で走り、Windows 側ではその VM が **`vmmemwslc-cli-<ユーザー>`** というプロセスになる。設定は
`%LOCALAPPDATA%\wslc\settings.yaml` で、**`.wslconfig` は効かない**(`memory=6GB` を書いたままで、既定の VM の
MemTotal は 15563 MB)。計器は `cargo xtask footprint <コマンド>`(生の行は `target/footprint/<run>/`)。
台はホスト 32GB / 24 論理 CPU、他の席の Claude セッションが同居した状態(計測の開始時点でホストの空き 10〜12GB)。

## 読み方

- **`vmmemwslc-…` のワーキングセット = Windows が失った量**、**VM の `/proc/meminfo` = Linux が使っていると思っている量**。
  メモリは名前空間で割れないので後者はコンテナを含む VM 全体で、**エンジンの `stats` の合計はこれの代わりにならない**
- **`MemFree` では判定しない** — 回収できるキャッシュが育っただけの VM も小さく見える。枯渇したかを言うのは
  `MemAvailable` と PSI(`/proc/pressure/memory`)と `oom_kill`
- **ホストの空きの低下を外部負荷の証拠にしない** — 低下分はほぼ `vmmemwslc-…` の増加分と同じ

## 何もしていない VM は消える

コンテナが 1 つも無くなってから `idleTimeout`(既定 30 秒)で VM ごと落ち、`vmmemwslc-…` のプロセスが消える
—— **キャッシュも含めて全部ホストへ戻る**(冷えたビルドの 2 分後にプロセスが無いことを確認)。
`wslc system session run` で VM の中のプロセスを動かしている間(`footprint` の VM の計器)は落ちない。

## 上限(`session.memorySize`)

**swap はセッションの `memorySize` と同じ大きさ**(セッションマネージャーが `SwapSizeMb = MemoryMb` で作る)。
VM が抱えられる量は上限の 2 倍まで。

イメージの列: **前** = Qt を全アーカイブで入れ、静的ライブラリだけを落とした app 2.08GB / **最終** = ci/linux/Dockerfile の
今の形(app 1.70GB。code-costs §コンテナのイメージと build cache)。コンパイラと依存の木は同じ。

| 負荷 | イメージ | memorySize | VM の中で使った最大(MemTotal − MemAvailable) | swap 最大 | `vmmemwslc` 最大 | ホストの空き 最小 | PSI some 合計 | oom_kill | 結果 |
|---|---|---|---|---|---|---|---|---|---|
| 冷えた release ビルド 1 本(`linux build --release --features automation`) | 前 | 既定(15.5GB) | 4.4GB | 0 | 8673 MB | 4345 MB | 0 | 0 | 1m06s |
| 冷えたビルド 4 本同時(app の clippy 2 本 + 上の release 2 本、各コンテナの `/tmp` の空の target。Windows 側は何も回していない) | 前 | 既定 | 15.0GB(MemAvailable 100 MB) | 1.2GB | 15699 MB | **3 MB** | 5.0 s | 0 | 全部 PASS(clippy 2m19s / release 2m39s) |
| 同じ 4 本 | 前 | **12GB** | 11.4GB(MemAvailable 46 MB) | 10.8GB | 12273 MB | 3352 MB | 45 s | 0 | 全部 PASS(clippy 3m07s〜3m10s / release 3m26s) |
| 同じ 4 本 | 前 | 8GB | 7.9GB(MemAvailable 12 MB) | 8GB(使い切り) | 8185 MB | 7437 MB | 197 s | 10 | 進まなくなり 300 s で止めた(4 本とも exit 137) |
| `gate --fresh`(段 2 の全ステップ。Linux の target は空から = イメージが組み直された直後の席の最初の gate。ホスト側も 498 ステップを同時に回した。席 1 つ) | 最終 | 12GB | 5.1GB(anon 最大 4.8GB) | 0 | 10236 MB | **678 MB** | 0 | 0 | PASS 6m40s(Linux 側 399 s / ホスト側 389 s) |
| 席 2 つを模した負荷: 上と同じ冷えた `gate --fresh` に、45 s 後からもう 1 席ぶんの Linux 側の冷えたビルド 2 本(app の clippy と release、同じ予算から ticket を取る)を重ねた。**もう 1 席の Windows 側は無い** | 最終(コンテナの incremental を切っていた版) | 12GB | 11.4GB(MemAvailable 102 MB) | 1.7GB | 12229 MB | **73 MB**(Windows のコミット最大 45.3GB) | 9.0 s | 0 | 全部 PASS(gate 8m27s / clippy 3m49s / release 4m30s)。コンテナは同時に最大 4 つ |
| `gate --all`(段 3。Linux 側 625 ステップ = 未スタンプの動詞。target は温まっていた。席 1 つ) | 前 | 12GB | 1.9GB | 0 | 4103 MB | 9029 MB | 0 | 0 | PASS 4m07s(コンテナは同時に最大 2 つ)。**最終イメージの `gate --all` は 3m34s(625 ステップ)で、footprint は取っていない** |
| VM 全体の OOM(`head -c 24G /dev/zero \| tail -n 1` を 1 本) | 前 | 8GB | 8GB + swap 8GB を使い切り | 8GB | — | — | — | 1(殺されたのは `tail` だけ) | エンジンは直後の `run` に 1 秒で答えた |

## 確かめた範囲

3 つを別々に読む。**どれも、席 2 つ以上が Windows 側を含めて同時に gate を撃つ条件(実際の並行開発)の答えではない**。

- **Linux 側の負荷への耐性**: 12GB で冷えたビルド 4 本が通る(Linux 側だけを見れば、2 席が同時に冷えた gate を撃った時の
  コンパイルの量)。swap を 12GB 中 10.8GB まで使い、時間は既定の 1.3 倍。8GB は swap を使い切って VM の中の OOM を
  繰り返し、どのビルドも進まない。既定は通るが VM が 15.7GB を取り、Windows 側の空きが 3 MB まで落ちた
- **VM 全体の OOM の後もエンジンは答える**: OOM killer が殺すのは食った側のプロセス 1 つで、dockerd とセッションの
  プロセスは残る。人工の OOM 1 回と、8GB の 4 本で実際のコンパイラが OOM を 10 回起こした最中(`ps` は 2.8 秒で答え、
  `rm --force` の後の次のコンテナは 1 秒で立った)
- **記録に残っている停止と同じ条件(席 1 つ)**: 記録にあるエンジンの停止は、冷えた Linux の target の上の land の gate で、
  Linux 側の 2 つのコンテナが C++ の依存を同時に組み、6GB + swap 2GB の VM が VM 全体の OOM になった時(anon 5.6GB +
  swap 2GB を使い切り。その後はエンジンも `wsl` も再起動まで答えなかった)。同じ条件 = 空の target からの `gate --fresh` を
  12GB の wslc で撃ち、anon 最大 4.8GB・swap 0・OOM 0 で PASS した。**止まった時に他の席が何を回していたかは記録に無く、
  並行開発での停止がこの 1 件だったかも分からない**

- **席 2 つを模した負荷(席 1 つの冷えた `gate --fresh` + もう 1 席の Linux 側の冷えたビルド 2 本)でもエンジンは
  答え続け、全部が通った**。VM は上限いっぱい(MemAvailable 102 MB、swap 1.7GB、メモリ待ち 9 秒)で OOM 0。
  **詰まったのはホストの側**で、Windows の空きは 73 MB まで落ちた(開始時点で他のセッション等が約 19GB を使っていた)

## 確かめていないこと

- **席 2 つ以上が Windows 側も含めて実際に同時に gate を撃つ条件**。上の模擬にはもう 1 席の Windows 側(ホストの
  コンパイルと動詞)が無い。機械の予算(`budget`)は席を跨いで 1 つなので、同時に走るユニットの数は gate 1 本ぶんで
  止まるが、冷えたコンパイルが重なった時のメモリはそれとは別に決まる。手掛かりは、席 1 つの `gate --fresh` で既に
  ホストの空きが 678 MB、模擬で 73 MB まで落ちたこと —— `vmmemwslc` の大半はページキャッシュ(冷えた `gate --fresh`
  で VM の中の anon は 4.8GB、`vmmemwslc` は 10.2GB)で、VM の中では回収できる量でも Windows には返らず、VM が
  消えるまで抱える
- **冷えたビルド 5 本以上**(4 本で swap の残りは 1.5GB)
- **上限を 12GB より下げた時の模擬負荷**(ホストの空きは増えるが、VM の中の swap と OOM の側へ寄る。8GB は Linux 側
  だけの 4 本で進まなくなった)
