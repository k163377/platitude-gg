# QML の部品の重さ(Windows x64)

窓とページの常駐と生成時間を「どの部品が何個・1 個いくら」に割った実測。**予算に対する答えは
[perf-windows-x64.md](perf-windows-x64.md)(実窓・D3D)が正**で、ここは内訳を読むための値 —
残る手は [P3-確認事項.md](../../internal-docs/P3-確認事項.md) §性能。

**測った木は `git rev-parse <commit>:crates/platitude-app/src/ui` が `1a7f3b1f` の木**(製品の QML。2026-11-13、
Qt 6.12.0、コーパスは [perf-windows-x64.md](perf-windows-x64.md) §計測条件 の token)。

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
  1 個建てて捨てた後、見えない親の下に 200 個建て、前後の上の値の差を 200 で割る。required property を持つ部品
  (行のデリゲート)は建たない。**見えない Canvas は描かれない**(`InkCanvas.qml` の冒頭)ので、画面に出る印は
  これより重い側に立つ
- **起動の段**: `tracing::info!` を `main.rs` の `qml_engine::load` の前と、`TabsModel::default`(最初のオブジェクト)に置く

## 個数

| | タブの無い窓 | 開いて行選択と diff まで |
|---|---|---|
| 全部 | 5,989 | 11,691 |
| うち `RepoPage`(空ページ / タブのページ) | 4,008 | 9,581 |
| `NavIcon` | 342 | 542 |
| `PlainTextLabel` / `Label` | 356 / 179 | 819 / 358 |
| `TextMetrics` / `FontMetrics` | 312 / 159 | 657 / 211 |
| `TipAsk` | 170 | 291 |
| `Loader` / `Repeater` | 186 / 171 | 1,427 / 404 |
| `Connections` / `Timer` / `Binding` | 340 / 220 / 163 | 463 / 222 / 165 |
| `ActionButton` とその派生(`NoticeButton` 3・`Band*Button` 3) | 19(ラベル 38・席 38) | 19(38・38) |
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
| `IdentityDialog` | 253 | `RebasePlanRunBar` | 109 |
| `OpExitCard` | 234 | `NoticeBar` | 90 |
| `RemoteDialog`(`PublishFlow` の下) | 218 | `SectionPeekPopup` / `CommitHoverCard` | 89 / 89 |
| `OpenFailedDialog` | 205 | `CarriedPane` | 77 |
| `RecoverBand` | 201 | `DiffFindBar` | 71 |
| `StartupGate` | 147 | `QuitWaitDialog` | 46 |
| `RecoverPane` | 146 | `RecoverHoverCard` / `GraphFind` | 43 / 43 |
| `OpenFailedScreen` × 2 | 142 | `ParentListCard` / `EolHoverCard` | 42 / 42 |
| `AskBar` | 135 | `RefListPopup` | 41 |
| `BandStateCard` / `AuthorCard` | 128 / 116 | 帯のメニュー 4 枚 | 611 |

## 1 個あたり

ヒープ(KB)/ 確保の個数 / private(KB)/ 1 個を建てる時間(µs、静かな台):

| 部品 | ヒープ | 確保 | private | 建てる時間 |
|---|---|---|---|---|
| `DialogActions`(ボタン 2 つ) | 1,000 | 5,085 | 1,141 | 3,390 |
| `ActionButton` / `NoticeButton` | 519 / 520 | 2,981 / 2,991 | 648 / 706 | 1,257 / 1,467 |
| `AppMenuItem` | 390 | 2,196 | 409 | 1,315 |
| `RefChipStack` / `RefChip` | 154 / 141 | 820 / 744 | 207 / 190 | 454 / 405 |
| `ActionButtonSeat` | 116 | 631 | 167 | 287 |
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
| `HoldDriver` / `HoldIcon` | 10 / 9 | 72 / 68 | 14 / 13 | 18 / 14 |
| `PlainTextLabel` / `InkCanvas` | 5.6 / 4.6 | 43 / 32 | 9.2 / 7.2 | 9 / 5 |
| 素の `Text` / `Canvas` | 1.5 | 6〜7 | 1.5 | 2 |
| 素の `Item` / `Rectangle` / `Loader` / `Timer` / `Connections` / `TextMetrics` | 0.5〜1 | 4〜5 | 0〜2 | 1〜2 |

- **束ねた部品の重さは、ほぼ中身で決まる** — `ActionButton` 1 個は横並びと 2 段の両方のラベルと席を建て(ラベル
  81 × 2 + 席 116 × 2 で 394)、席は印 3 つ・リング・回数押しの印 2 つ・分数の斜線を状態に依らず持つ。和に
  ぴったりは合わない(`DialogActions` は `ActionButton` 2 個で 1,000)
- **`NavIcon` は素の `Canvas` の 11 倍余り** — 違いは `NavIcon` が自分に足した分(派生プロパティ 20 本余りとハンドラ 7 本)。
  束ねた部品の大半がこれを数個ずつ持つ
- **素の部品はどれも 1KB 前後・確保 4〜7 個**

## プロセスの量と起動(同じ撃ち方どうし)

MB(確保は個数)。開いて diff までとタブを待つ窓は 3 run、タブの無い窓は 1 run:

| | タブの無い窓 | タブを待つ窓 | 開いて行選択と diff まで |
|---|---|---|---|
| WorkingSet | 162.8 | 98.3–98.8 | 415.8–416.6 |
| private | 143.0 | 79.2–79.7 | 390.9–391.4 |
| プロセスヒープ busy(確保) | 75.2(440.4K) | 40.1–40.2(212.8K) | 180.4(858.0K) |

- **空ページ 1 枚は WorkingSet 64MB・private 63〜64MB・ヒープ 35MB(確保 22.8 万個)** — 左 2 列の差
- **起動(3 run)**: 起動から最初のオブジェクトまで 876–879ms — うち `qml_engine::load` の前は 16ms で、残りは QML の
  型の読み込み。その後、帯・ダイアログ・ハーネスまで 230–235ms、タブのページを建ててグラフの最初のフレームまで
  462–526ms
- **コンパイル済みキャッシュを使う**(`QML_DISABLE_DISK_CACHE` を外す、3 run)と、最初のオブジェクトまで 684–709ms
  — **型の読み込みのうちコンパイルが占めるのは約 2 割**で、残りが何かは割っていない。開いて diff までの
  WorkingSet は 411.6–412.6、private は 380.0–380.4、ヒープ busy は 172.1 — **private とヒープは 8〜11MB 小さいが、
  予算が読む WorkingSet は 5MB も動かない**(rules-refs/app-ui.md「qmlcachegen はメモリ対策にならない」と同じ向き)
