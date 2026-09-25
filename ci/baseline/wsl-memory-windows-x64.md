# gate を繰り返した時の WSL のメモリ(Windows x64)

`cargo xtask footprint gate --all --fresh` を**間に `wsl --shutdown` を挟まず 3 本連続**で撃った実測。
生の行は `target/footprint/<run>/`(`host.csv` / `vm.txt` / `docker-before.txt` / `docker-after.txt`)。

## 読み方

- **`vmmemWSL` のワーキングセット = Windows が失った量**、**VM の `/proc/meminfo` = Linux が使っていると思っている量**。
  メモリは名前空間で割れないので後者はコンテナを含む VM 全体で、**`docker stats` の合計はこれの代わりにならない**
- **`MemFree` では判定しない** — 回収できるキャッシュが育っただけの VM も小さく見える。枯渇したかを言うのは
  `MemAvailable` と PSI(`/proc/pressure/memory`)と `oom_kill`
- **ホストの空きの低下を外部負荷の証拠にしない** — 低下分はほぼ `vmmemWSL` の増加分と同じ。常駐の内訳を挙げる時は
  **同時刻に撮ったプロセス表**を添える。時刻の違う 2 枚の差は外部負荷の証拠にならない

## 3 本の gate(連続、リセット無し)

`--all --fresh` を使うのは、**キャッシュの当たった gate は何も実行しないから**。
ホスト 31903 MB。VM の MemTotal は 15564 MB。`~/.wslconfig` は `[experimental] autoMemoryReclaim=gradual` のみで
**`memory=` の上限は無い**。

**1 本あたり返らない分は約 900 MB**(run 2 終 → run 3 終で `vmmemWSL` +896 MB、ホスト空き −750 MB)。
内訳は **Buffers +337 / Cached +301 / SReclaimable +198 / Percpu +25 / SUnreclaim +25**、
AnonPages は +2 = **育つのは全部キャッシュとカーネルのスラブで、プロセスのメモリではない**。gate 後のコンテナ残留は 0。

**VM の中は枯渇していない** —— MemAvailable は 3 本とも 12.4 GB を下回らず、PSI の待ちは累計 166 µs、
oom_kill は 0。Linux には回収する理由が無く、**WSL がホストへ返すのは VM の中で free なページだけ**なので、
Windows 側だけが減る。

## gate を止めた後も増える

run 3 の後、**gate もコンテナも 1 つも走っていない 9 分間**に、ホスト空き **4352 → 447 MB**、
`vmmemWSL` **9447 → 14966 MB**(VM の MemAvailable は 14.3 GB で、やはり枯渇していない)。
読んでいるのは **`/initd services`(Docker Desktop が distro の中で回している常駐)** 1 本で、約 24 MB/s。
同時刻の `dockerd` と `containerd` の `read_bytes` は 1 バイトも動かない(`/proc/<pid>/io`)。
**この間、`autoMemoryReclaim=gradual` は 1 度も返していない**。

## 読み手が黙れば返る

`/initd services` の `read_bytes` が実質止まると、約 600 MB/分で返った(1 回の観測)。アイドル判定は VM の中を見るので、
**Docker Desktop の常駐が読み続けている間はアイドルにならない**。

## 読み手が何を読んでいるか

走査中の `/initd services` が開いていたのは
`/var/lib/docker/volumes/pgg-linux-demo-a/_data/one-commit-43-…-0/repo/.git` ——
**gate が作ったコンテナ用のデモリポジトリ**(`/proc/<pid>/fd` で現認)。掃除を入れる前のボリューム:

| ボリューム | ファイル数 | 大きさ |
|---|---|---|
| `pgg-linux-demo-a` | **2,646,738**(run ディレクトリ 14,895) | 10.6 GB |
| `pgg-linux-target-a` | 22,132 | 10.4 GB |

**inode の 99% はデモ側**で、gate 1 本が約 500 ディレクトリ(約 9 万ファイル)を足していた。
コンテナ側の掃除(`linux::runner::SCRIPT`)はこれを
14,922 → 1,797 ディレクトリに落とし、それまで単調増加だった SReclaimable が初めて減った。
**`vmmemWSL` の終値は掃除の前後の比較に使えない** —— Buffers はアイドルになるまで返らないので、
この列は開始状態の話をしている。
`/initd` のログ(`%LOCALAPPDATA%\Docker\log\vm\init.log`)は
**コンテナが 1 つ消えるたびに `fstrim /var/lib/docker` を仕掛け直し**(gate 中は約 10 秒ごと)、
**走査そのものは 1 行も記録しない** —— 走査を起こす条件は未特定。

## `/initd services` のストレージ読み取り量 — 区間で分けて読むこと

**`read_bytes` はストレージから読んだバイト数であって、何かの「開始・停止」ではない。**
増えたことは「そのプロセスがディスクを読んだ」としか言わず、止まったことは
「キャッシュに当たっていた」でも成り立つ。窓の長さが違う区間から**削減率を出してはいけない** ——
掃除が走査をどれだけ減らしたかは確定していない。言えるのは「掃除の後にもこのプロセスはディスクを読んでいる」ことだけ。

## 単発の `drop_caches` は恒久対策にしない

効いたのは約 90 秒(`vmmemWSL` 9922 → 3243 MB)で、そのあと `/initd services` の `read_bytes` が 25 MB/s で伸びて
3.5 分で元の水準へ戻った。**キャッシュを落とせばディスクからの読み直しが増える**ので、恒久対策としては採らない。

## 「通った run が自分のデモルートを返す」の前後(窓を揃えた 1 対)

同じ `gate --all --fresh`、**終了後の観測窓はどちらも 1200 秒**、開始時の VM を揃えて(`vmmemWSL` 6226 / 5578 MB)。
ホスト空きの底は gate 終了の後に来ることがあり(この対ではどちらも終了 +11 分)、240 秒の窓では見えない。

**効いたのは 1 つだけ。** gate 1 本がデモボリュームに残す run ディレクトリは **740 → 99**
(514 の動詞に対して。残った 99 はほぼテンプレート)。

**効いていないもの。** ホスト空きの底(2019 → 1395 MB)、2.5 GB 未満の継続(146 → 438 秒)、
`/initd services` の読み取り量(9178 → 9118 MB)—— **この 1 対では改善していない**。
**読み取り量がデモの inode 数に比例するという読みはこの対では支持されない**。
ホスト空きの差は開始時点で 2.1 GB 違っており、**この 1 対から返却の効果とは言えない**。
**残骸の削減とホストの低空きは別々に判定する** —— 残骸が減ったことをもって WSL の低空き症状を片付いたことにしない。

## `memory=6GB` の上限

`~/.wslconfig` は `[wsl2] memory=6GB` と `[experimental] autoMemoryReclaim=gradual`(WSL 2.6.3。今の WSL は
`autoMemoryReclaim` を書かないと `dropCache` が既定)。VM の MemTotal は 5922 MB。計器は 3 秒ごとの docker-desktop distro の
`/proc/meminfo`・`/proc/pressure/memory`・`/proc/vmstat` の `oom_kill` と `vmmemWSL` の私有メモリで、ホストの空きは
`Win32_OperatingSystem.FreePhysicalMemory`(**WMI** —— 次に撃つなら `xtask::footprint`)。

| 負荷 | 上限 | VM の中で使った最大(MemTotal − MemAvailable) | `vmmemWSL` 最大 | ホストの空き 最小 | VM のメモリ待ち(PSI some)| 結果 |
|---|---|---|---|---|---|---|
| `gate --fresh`(他席の gate と同居) | 無し(15564 MB) | 2335 MB | 8301 MB | 122 MB | 未計測 | PASS 22m45s |
| `gate --fresh`(単独、雛形を組み直した回) | 無し | 2863 MB | 6580 MB | 220 MB | 未計測 | PASS 12m52s |
| 依存を最初から組む release ビルド(`linux build --target-dir //tmp/…`) | 無し | 4800 MB | 11638 MB | 387 MB | 未計測 | 35.3s |
| `gate --fresh`(Defender の記録中、後半は他席の gate と同居、雛形を組み直した回) | 6GB | 3691 MB | 6214 MB | 173 MB | 16 分で 0.10 秒・OOM 0 | PASS 16m02s |
| 依存を最初から組む release ビルド | 6GB | 4024 MB(キャッシュは 760 MB まで押された) | 6220 MB | 273 MB | 57 秒で 0.19 秒・OOM 0 | 44.4s |
| `gate --fresh`(単独、雛形が立っていた回) | 6GB | 2168 MB | 4787 MB | 379 MB | 0・OOM 0 | PASS 8m57s |

- **上限は WSL が取る量を 6.2GB で止める**。上限なしでは gate で 6.6〜8.3GB、依存を最初から組むビルドで 11.6GB まで取った。
  **VM の中の余裕と Windows 側の低空きは別の指標**で、上限が効くのは後者の側
- **VM の中は足りている**: 最悪の負荷(依存を最初から組むビルド、VM の中で 4.0〜4.8GB)でもキャッシュが押されるだけで、
  メモリ待ちは 0.2 秒以下・OOM 0、gate の `Cannot allocate memory` は 0 本
- **4GB は置かない**: 依存を最初から組むビルドの 4.0〜4.8GB が入らない。6GB が最悪の負荷を RAM に収める最小
- **速さは動かない**: host の動詞 593 本を全部回し、雛形が立っていた gate 同士で 8m52s(上限無し)
  対 8m57s(上限 6GB)。表の 12m52s 対 8m57s の差は雛形の組み直しの有無(code-costs §テストとハーネス の雛形の行)。
  ビルドの 35.3s → 44.4s は 1 本ずつの対で、メモリ待ちは合計 0.19 秒 —— 上限の代金とは言えない
