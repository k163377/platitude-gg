# QML の部品の重さ(Windows x64)

窓とページの常駐と生成時間を「どの部品が何個・1 個いくら」に割った実測。**予算に対する答えは
[perf-windows-x64.md](perf-windows-x64.md)(実窓・D3D)が正**で、ここは内訳を読むための値 —
残る手は [P3-確認事項.md](../../internal-docs/P3-確認事項.md) §性能。

**§個数 と §初回の生成 と §プロセスの量と起動 は `git rev-parse <commit>:crates/platitude-app/src/ui` が
`b8573af3` の木**(製品の QML。2026-11-14、Qt 6.12.0、コーパスは [perf-windows-x64.md](perf-windows-x64.md) §計測条件 の
token)。§1 個あたり の表は前の木で測った値 — `AppMenuItem` は `6631d4e5`、`DialogActions` / `ActionButton` /
`NoticeButton` / `ActionButtonSeat` は `82201378`、残りは `1a7f3b1f` の木。今の木との違いは、契機まで出ない部品を席に
置いた側のファイル(`6631d4e5` から)と、`AppMenuItem.qml` / `AppMenuButton.qml` / `TabStrip.qml` / `TopBar.qml`
(`82201378` から。`1a7f3b1f` からはさらに `ActionButton.qml` と `ActionButtonSeat.qml`)で、どれもそれらの行の部品に
入らない(`ActionButtonLabel.qml` はコメントだけ違う)。

## 測り方

ハーネス(`auto/`)と起動の経路に使い捨ての口を足した席のビルドを headless(offscreen)で撃ち、口が出す行を読む。
**offscreen は software scene graph で、フォントの歩きも払わない**(性能.md §計測の各論)ので、**実窓の値とは
並べない** — 読むのは同じ撃ち方どうしの差と、部品どうしの比。

| 読む時点 | 撃つ行 |
|---|---|
| タブの無い窓 | `verify-ui window-floor --restore` の、`WindowHarness.begin` が呼ばれた時点(窓が上がった直後) |
| タブを待つ窓(ページがまだ無い) | `verify-ui perf diff --repo <コーパス>` の、同じ時点(頼んだフォルダはまだ着いていない) |
| 開いて行選択と diff まで | 同じ run の、絵の時点(`WindowCensus.reportAt`) |

どの run にも `--no-board --app-env QML_DISABLE_DISK_CACHE=1` を足す(出荷ビルドと同じく、QML を起動のたびに
コンパイルする。verify-ui の既定はコンパイル済みキャッシュ有り)。動詞の run なので、検証用のドライバが建つ分は
どの数にも同じだけ乗っている。

口の作り直し方(commit しない):

- **個数**: `WindowCensus.qml` に、窓の根から `data`・`contentData`・`contentItem` / `background`・Loader の `item`・
  QML で宣言した名前付きプロパティの値を辿り、`String(object)` の型名で数える関数を足して、上の 2 つの時点で
  `Harness.report` する。ハーネス自身(`WindowHarness` / `PageHarness`)の下は数えない。名前付きプロパティの値が
  別の親を持つ Item なら辿らない(参照を持ち主に数えない)。**辿り方に現れない物は数に入らない**
- **メニュー・ダイアログ・カードの先建てを切る**: `PageHarness` / `WindowHarness` が動詞の run で立てる `keepBuilt`
  (ページのと帯の — `RepoPage` / `TopBar` — と、窓の 3 つの席 — `WindowDialogSeat` / `IdentityGate` /
  `WindowQuitGate` — の)を false にする(切らないと製品と違う数になる)。動詞は開く前にメニューやカードを読むので、
  その動詞は約束の照合で赤くなるが、口の行は出る
- **初回の生成**: 絵の時点で、まだ建っていない席 — `active` が false の `Loader` と、`Component` から作るカード
  (`DetailsAuthorCards.built` / `WipPane.builtEolCard`)— を 1 つずつ建て、前後の `PerfProbe.clockMs` の差と建った
  個数を出す。絵の後で窓の中身を増やすので、その run 自体は赤になる
- **プロセスの量**: `harness/singleton.rs` の `report` に、`K32GetProcessMemoryInfo`(WorkingSet / `PrivateUsage`)と
  `GetProcessHeaps` + `HeapLock` + `HeapWalk`(busy のバイトと個数)を読んで行に添える口を足す
- **1 個の重さ**: `WindowHarness.begin` で、部品を `Qt.createComponent("qrc:/qt/qml/platitude/ui/<名>.qml")` から
  1 個建てて捨てた後、見えない親の下に 200 個建て、前後の上の値の差を 200 で割る。既定のプロパティのまま建てる
  (`ActionButton` は印の無い 1 段の形)ので、形を変えた値は `createObject` に初期値を渡して測る。required property を持つ部品
  (行のデリゲート)は建たない。**見えない Canvas は描かれない**(`InkCanvas.qml` の冒頭)ので、画面に出る印は
  これより重い側に立つ
- **起動の段**: `tracing::info!` を `main.rs` の `qml_engine::load` の前と、`TabsModel::default`(最初のオブジェクト)に置く
- **メニューを初めて開く 1 回**: `RepoPage.openRowMenu` の、席を建てる行(`commitMenuSeat.active = true`)の前後と開き
  終えた所で、`PerfProbe.clockMs()` の時刻と上のプロセスの量を報告する口を足し、`verify-ui commit-menu` を撃つ

## 個数

| | タブの無い窓 | 開いて行選択と diff まで |
|---|---|---|
| 全部 | 2,860 | 8,312 |
| うち `RepoPage`(空ページ / タブのページ) | 2,162 | 7,673 |
| `NavIcon` | 112 | 304 |
| `PlainTextLabel` / `Label` | 165 / 71 | 620 / 239 |
| `TextMetrics` / `FontMetrics` | 107 / 50 | 422 / 83 |
| `TipAsk` | 75 | 185 |
| `Loader` / `Repeater` | 137 / 48 | 1,357 / 262 |
| `Connections` / `Timer` / `Binding` | 206 / 139 / 96 | 316 / 137 / 93 |
| `ActionButton` とその派生(`NoticeButton`・`Band*Button` 3) | 9(ラベル 13・席 9) | 8(12・8) |
| `CardText` | 8 | 6 |
| `AppMenu` / `AppMenuItem` | 0 / 0 | 0 / 0 |

- **タブの無い窓の 4 分の 3 は空ページ** — タブのページと同じ `RepoPage` で、ペインと常に出ている帯を持つ。契機が
  来るまで出ないカード・ダイアログ・帯は、出る時に建つ(§初回の生成)
- **行は 1 行あたり** グラフ 50 個(41 行)/ 左メニュー 37 個(42 行)/ diff 21 個(47 行)/ ファイル一覧 22 個(38 行)
- **メニューはどれも初めて開いた時に建つ**(ページのメニューは `RepoPage.keepBuilt`、帯の ☰ と名前の 2 枚は
  `TopBar.keepBuilt`)ので、上のどちらの窓にも `AppMenu` / `AppMenuItem` は無い。コミット行のメニューは 1 枚で
  1,993 個、初めて開く 1 回は建てるのに 54–58ms・開くのに 22–23ms(3 run、他の席の verify が並ぶ台)、
  ヒープ +9.6MB(確保 6.3 万個)・private +11.4–13.4MB — このメニューの数だけは `6631d4e5` の木で測った値で、
  そこからの違いはメニューの部品に入らない

タブの無い窓に、見せていないまま建っている物(その下の個数):

| 部品 | 個数 |
|---|---|
| `RecoverBand` の殻(物差し 2 本と下線 — 床が読む。行は項目が載っている間だけ) | 45 |
| `AskBar` の殻(物差し 2 本と下線と Esc — 床が読む。中身は最初の問いで建つ) | 42 |
| `NoticeBar` の殻(物差しと Esc — 床が読む。文言と `OK` は最初の報せで建つ) | 26 |
| `PublishFlow`(リモートを書き留めるダイアログは最初に開く時に建つ) | 13 |

`StartupGate`(124)は `begin` の時点では git の確認がまだ出ていて立っている — git が答えると壊れ、開いて diff までの
数には居ない。

## 初回の生成

契機が来て初めて建つ物の、建つまでの時間(ms、開いて diff までの run 3 本の絵の時点。§測り方)と、その下の個数
(子と中身だけを辿った数 — 名前付きプロパティも辿る §個数 の数より少なく出る)。建てた後は持つ物(★)と、出ている間
だけ建つ物がある:

| 部品 | 個数 | ms | 部品 | 個数 | ms |
|---|---|---|---|---|---|
| `IdentityDialog` ★ | 170 | 6.1–7.5 | `AskBar` の中身(見出し・語・ピル・×)★ | 85 | 3.5–3.9 |
| `AuthorCard` ★ | 155 | 6.2–10.2 | `CommitHoverCard` ★ | 76 | 3.1–3.8 |
| `OpExitCard` | 146 | 6.1–8.8 | `RebasePlanRunBar` ★ | 69 | 2.8–4.1 |
| `RemoteDialog` ★ | 143 | 5.2–6.9 | `CarriedPane` ★ | 56 | 2.1–3.5 |
| `OpenFailedDialog` ★ | 128 | 4.3–5.2 | `NoticeBar` の文言と `OK` ★ | 55 | 2.3–2.6 |
| `RecoverPane`(`RecoverHoverCard` 込み)★ | 112 | 5.3–6.1 | `SectionPeekPopup`(畳んだレールの節)★ | 53 | 2.5–2.7 |
| `BandStateCard` ★ | 112 | 4.9–5.7 | `DiffFindBar` ★ | 50 | 2.2–2.7 |
| `RecoverBand` の行 | 110 | 3.7–4.5 | `QuitWaitDialog` ★ | 39 | 2.9–3.3 |
| `StartupGate` | 103 | 4.1–4.6 | `GraphFind` ★ | 36 | 2.2–2.5 |
| `OpenFailedScreen` | 95 | 3.3–5.2 | `EolHoverCard` ★ / `ParentListCard` ★ / `RefListPopup` ★ | 35 / 31 / 31 | 1.5–2.6 |
| | | | `CoAuthorCard` ★ | 10 | 0.6–0.8 |

- **どれも 1 フレームに収まる**(最も重い `AuthorCard` で 6–10ms)— hover のカードは休止の待ちの後に開くので、その開く
  フレームが数 ms 延びるだけ
- `StartupGate` は窓の最初のフレームから git が答えるまで立つ — 起動で建ち、答えの後で壊れる
- `AuthorCard` の個数はコミットで違う(空ページでは 101)

## 1 個あたり

ヒープ(KB)/ 確保の個数 / private(KB)/ 1 個を建てる時間(µs、静かな台):

| 部品 | ヒープ | 確保 | private | 建てる時間 |
|---|---|---|---|---|
| `DialogActions`(ボタン 2 つ) | 423 | 2,609 | 548 | 1,045 |
| `ActionButton` / `NoticeButton` | 208 / 209 | 1,279 / 1,288 | 247 / 272 | 532 / 558 |
| `AppMenuItem`(語だけの行) | 162 | 1,014 | 181 | 626 |
| `RefChipStack` / `RefChip` | 154 / 141 | 820 / 744 | 207 / 190 | 454 / 405 |
| `AppListView` | 116 | 709 | 142 | 495 |
| `NameCell` | 97 | 605 | 127 | 212 |
| `LabeledField` / `HelpText` | 89 / 86 | 576 / 543 | 115 / 112 | 437 / 401 |
| `AvatarButton` / `HeadTrack` | 87 / 86 | 513 / 467 | 132 / 117 | 226 / 165 |
| `CardText` | 84 | 533 | 111 | 358 |
| `ActionButtonLabel` | 81 | 345 | 118 | 245 |
| `PaneScrollBar` / `AutoScrollBar` | 75 / 73 | 497 / 480 | 69 / 73 | 259 / 305 |
| `NameRun` | 73 | 430 | 94 | 198 |
| `CodeLine` / `AppMenu`(行なし) | 59 / 58 | 420 / 410 | 82 / 69 | 179 / 146 |
| `LineText` / `CodeChip` | 52 / 51 | 311 / 269 | 63 / 68 | 200 / 106 |
| `GoneBadge` / `CutName` | 44 / 36 | 199 / 251 | 67 / 50 | 86 / 71 |
| `SpinnerIcon` / `ChangeIcon` | 21 / 20 | 103 / 97 | 32 / 33 | 39 / 44 |
| `NavIcon` | 17 | 91 | 18 | 33 |
| `ActionButtonSeat` | 16 | 105 | 21 | 24 |
| `HoldDriver` / `HoldIcon` | 10 / 9 | 72 / 68 | 14 / 13 | 18 / 14 |
| `PlainTextLabel` / `InkCanvas` | 5.6 / 4.6 | 43 / 32 | 9.2 / 7.2 | 9 / 5 |
| 素の `Text` / `Canvas` | 1.5 | 6〜7 | 1.5 | 2 |
| 素の `Item` / `Rectangle` / `Loader` / `Timer` / `Connections` / `TextMetrics` | 0.5〜1 | 4〜5 | 0〜2 | 1〜2 |

- **束ねた部品の重さは、中身とその部品自身の分の和** — `ActionButton` 1 個 208 のうち、描く形のラベルと席の 1 組が
  ラベル 81 + 席 16 で、残りの半分余りはボタン自身(`HoverToolButton` 22・`HoldDriver` 10 と、`ActionButton` が
  自分に足したプロパティとハンドラ)。席はアイコンと、着ている印だけを建てる。和にぴったりは合わない
  (`DialogActions` は `ActionButton` 2 個で 423)。
  形で変わる分は、bench をこの系に絞った同じ測り方の run で、印の無い形 208 / アイコン付き 242 / 2 段 305。
  2 段は、帯がセルを測る 1 段の語を描かない物差しのラベルとしてもう 1 つ持つ(`ActionButton.lineWord`)ので、
  §個数 のラベル 23 = ボタン 19 + 帯の 2 段 4 つの物差し
- **`AppMenuItem` は行が着ている物だけを建てる** — 見出しと席の印・`!`・回数押しの印と塗りと状態機械・チップの地・
  先後の数・バッジはそれを持つ行にだけ、名前の文の物差しと印と地はその文を持つ行にだけ建つ。形で変わる分は、同じ
  測り方の run で、チップ付き 174 / 見出し 186 / `!` 184 / 回数押し 198 / 名前の文 248 / 枝のカードの行(先後の数・
  バッジ・席の印)300。この行の時間は他の席の verify が並ぶ台で撮った(同じ run の `NavIcon` は 33〜39µs)
- **`NavIcon` は素の `Canvas` の 11 倍余り** — 違いは `NavIcon` が自分に足した分(派生プロパティ 20 本余りとハンドラ 7 本)。
  束ねた部品の大半がこれを数個ずつ持つ
- **素の部品はどれも 1KB 前後・確保 4〜7 個**

## プロセスの量と起動(同じ撃ち方どうし)

MB(確保は個数)。開いて diff までとタブを待つ窓は 3 run、タブの無い窓は 1 run:

| | タブの無い窓 | タブを待つ窓 | 開いて行選択と diff まで |
|---|---|---|---|
| WorkingSet | 129.8 | 84.4–84.7 | 344.8–348.0 |
| private | 110.7 | 66.1–66.5 | 320.2–323.2 |
| プロセスヒープ busy(確保) | 52.9(312.1K) | 30.4(152.3K) | 158.8(787.7–787.9K) |

- **空ページ 1 枚は WorkingSet 45MB・private 44MB・ヒープ 22.5MB(確保 16.0 万個)** — 左 2 列の差
- **起動(3 run)**: 起動から最初のオブジェクトまで 857–888ms — うち `qml_engine::load` の前は 15–16ms で、残りは QML の
  型の読み込み。その後、帯・ダイアログ・ハーネスまで 140–145ms、タブのページを建ててグラフの最初のフレームまで
  377–467ms
- **コンパイル済みキャッシュを使う**(`QML_DISABLE_DISK_CACHE` を外す、キャッシュの温まった 2 run)と、最初の
  オブジェクトまで 673–700ms — **型の読み込みのうちコンパイルが占めるのは約 2 割**で、残りが何かは割っていない。
  開いて diff までの WorkingSet は 342.1–343.8、private は 310.9–312.5、ヒープ busy は 150.7 — **private とヒープは
  8〜12MB 小さいが、予算が読む WorkingSet は数 MB しか動かない**
  (rules-refs/app-ui.md「qmlcachegen はメモリ対策にならない」と同じ向き)
