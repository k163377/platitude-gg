# QML の部品の重さ(Windows x64)

窓とページの常駐と生成時間を「どの部品が何個・1 個いくら」に割った実測。**予算に対する答えは
[perf-windows-x64.md](perf-windows-x64.md)(実窓・D3D)が正**で、ここは内訳を読むための値 —
残る手は [P3-確認事項.md](../../internal-docs/P3-確認事項.md) §性能。

**測った木は `git rev-parse <commit>:crates/platitude-app/src/ui` が `82201378` の木**(製品の QML。2026-11-13、
Qt 6.12.0、コーパスは [perf-windows-x64.md](perf-windows-x64.md) §計測条件 の token)。§1 個あたり の表のうち
`DialogActions` / `ActionButton` / `NoticeButton` / `ActionButtonSeat` 以外の行は `1a7f3b1f` の木で測った値 —
2 つの木の違いは `ActionButton.qml` と `ActionButtonSeat.qml`、`ActionButtonLabel.qml` のコメントだけ。

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
- **メニューとダイアログの先建てを切る**: `PageHarness` / `WindowHarness` が動詞の run で立てる `keepBuilt` を
  false にする(切らないと製品と違う数になる)
- **プロセスの量**: `harness/singleton.rs` の `report` に、`K32GetProcessMemoryInfo`(WorkingSet / `PrivateUsage`)と
  `GetProcessHeaps` + `HeapLock` + `HeapWalk`(busy のバイトと個数)を読んで行に添える口を足す
- **1 個の重さ**: `WindowHarness.begin` で、部品を `Qt.createComponent("qrc:/qt/qml/platitude/ui/<名>.qml")` から
  1 個建てて捨てた後、見えない親の下に 200 個建て、前後の上の値の差を 200 で割る。既定のプロパティのまま建てる
  (`ActionButton` は印の無い 1 段の形)ので、形を変えた値は `createObject` に初期値を渡して測る。required property を持つ部品
  (行のデリゲート)は建たない。**見えない Canvas は描かれない**(`InkCanvas.qml` の冒頭)ので、画面に出る印は
  これより重い側に立つ
- **起動の段**: `tracing::info!` を `main.rs` の `qml_engine::load` の前と、`TabsModel::default`(最初のオブジェクト)に置く

## 個数

| | タブの無い窓 | 開いて行選択と diff まで |
|---|---|---|
| 全部 | 5,607 | 11,309 |
| うち `RepoPage`(空ページ / タブのページ) | 3,779 | 9,352 |
| `NavIcon` | 220 | 420 |
| `PlainTextLabel` / `Label` | 296 / 164 | 759 / 343 |
| `TextMetrics` / `FontMetrics` | 312 / 159 | 657 / 211 |
| `TipAsk` | 170 | 291 |
| `Loader` / `Repeater` | 285 / 156 | 1,526 / 389 |
| `Connections` / `Timer` / `Binding` | 340 / 220 / 163 | 462 / 222 / 165 |
| `ActionButton` とその派生(`NoticeButton` 3・`Band*Button` 3) | 19(ラベル 23・席 19) | 19(23・19) |
| `CardText` | 58 | 62 |
| `AppMenu` / `AppMenuItem` | 4 / 10 | 4 / 11 |

- **タブの無い窓の 3 分の 2 は空ページ** — タブのページと同じ `RepoPage` で、ペイン・帯・カードを全部持つ
- **行は 1 行あたり** グラフ 50 個(41 行)/ 左メニュー 37 個(42 行)/ diff 21 個(47 行)/ ファイル一覧 22 個(38 行)
- **ページのメニューは初めて開いた時に建つ**(`RepoPage.keepBuilt`)ので、上の `AppMenu` 4 / `AppMenuItem` 10 は帯の
  メニュー(☰ と `TopBar` の 3 枚)だけ。全部を先に建てると `AppMenu` は 22・`AppMenuItem` は 97 になり、
  コミット行のメニュー 1 枚で 2,822 個

タブの無い窓に、見せていないまま建っている物(その下の個数):

| 部品 | 個数 | 部品 | 個数 |
|---|---|---|---|
| `OpExitCard` | 211 | `NoticeBar` | 90 |
| `IdentityDialog` | 208 | `SectionPeekPopup` / `CommitHoverCard` | 89 / 89 |
| `RecoverBand` | 178 | `RebasePlanRunBar` | 86 |
| `RemoteDialog`(`PublishFlow` の下) | 173 | `CarriedPane` | 77 |
| `OpenFailedDialog` | 160 | `DiffFindBar` | 71 |
| `RecoverPane` | 146 | `QuitWaitDialog` | 46 |
| `AskBar` | 135 | `RecoverHoverCard` / `GraphFind` | 43 / 43 |
| `BandStateCard` / `AuthorCard` | 128 / 116 | `ParentListCard` / `EolHoverCard` | 42 / 42 |
| `StartupGate` | 124 | `RefListPopup` | 41 |
| `OpenFailedScreen` × 2 | 119 | 帯のメニュー 4 枚 | 611 |

## 1 個あたり

ヒープ(KB)/ 確保の個数 / private(KB)/ 1 個を建てる時間(µs、静かな台):

| 部品 | ヒープ | 確保 | private | 建てる時間 |
|---|---|---|---|---|
| `DialogActions`(ボタン 2 つ) | 423 | 2,609 | 548 | 1,045 |
| `AppMenuItem` | 390 | 2,196 | 409 | 1,315 |
| `ActionButton` / `NoticeButton` | 208 / 209 | 1,279 / 1,288 | 247 / 272 | 532 / 558 |
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
- **`NavIcon` は素の `Canvas` の 11 倍余り** — 違いは `NavIcon` が自分に足した分(派生プロパティ 20 本余りとハンドラ 7 本)。
  束ねた部品の大半がこれを数個ずつ持つ
- **素の部品はどれも 1KB 前後・確保 4〜7 個**

## プロセスの量と起動(同じ撃ち方どうし)

MB(確保は個数)。開いて diff までとタブを待つ窓は 3 run、タブの無い窓は 1 run:

| | タブの無い窓 | タブを待つ窓 | 開いて行選択と diff まで |
|---|---|---|---|
| WorkingSet | 155.0 | 95.7–95.8 | 403.7–404.7 |
| private | 135.0 | 76.5–76.6 | 378.5–379.4 |
| プロセスヒープ busy(確保) | 70.0(425.0K) | 37.8(201.3K) | 172.1(803.8K) |

- **空ページ 1 枚は WorkingSet 59MB・private 58MB・ヒープ 32MB(確保 22.4 万個)** — 左 2 列の差
- **起動(3 run)**: 起動から最初のオブジェクトまで 847–863ms — うち `qml_engine::load` の前は 15–16ms で、残りは QML の
  型の読み込み。その後、帯・ダイアログ・ハーネスまで 194–200ms、タブのページを建ててグラフの最初のフレームまで
  446–463ms
- **コンパイル済みキャッシュを使う**(`QML_DISABLE_DISK_CACHE` を外す、3 run)と、最初のオブジェクトまで 670–673ms
  — **型の読み込みのうちコンパイルが占めるのは約 2 割**で、残りが何かは割っていない。開いて diff までの
  WorkingSet は 400.1–401.0、private は 368.1–368.9、ヒープ busy は 163.8 — **private とヒープは 8〜11MB 小さいが、
  予算が読む WorkingSet は 5MB も動かない**(rules-refs/app-ui.md「qmlcachegen はメモリ対策にならない」と同じ向き)
