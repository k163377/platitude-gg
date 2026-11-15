# QML の部品の重さ(Windows x64)

窓とページの常駐と生成時間を「どの部品が何個・1 個いくら」に割った実測。**予算に対する答えは
[perf-windows-x64.md](perf-windows-x64.md)(実窓・D3D)が正**で、ここは内訳を読むための値 — 残る手は
[P3-確認事項.md](../../internal-docs/P3-確認事項.md) §性能。

**§個数・§1 個あたり・§プロセスの量と起動・§import の代金 は `git rev-parse <commit>:crates/platitude-app/src/ui` が
`2e474242` の木**(製品の QML。2026-11-14、Qt 6.12.0、コーパスは [perf-windows-x64.md](perf-windows-x64.md) §計測条件 の
token)。**§初回の生成 と、§個数 のコミット行のメニューの数・§1 個あたり の形ごとの値は、その木になる前の形で測った値** —
違いは、開いたまま建つ `AskBar` が開幕を取る順序・`RepoPage` の `Connections` 1 個・`SidebarPane` のプロパティ 1 本とコ
メントで、どれもそれらの数の部品に入らない。

## 測り方

ハーネス(`auto/`)と起動の経路に使い捨ての口を足した席のビルドを headless(offscreen)で撃ち、口が出す行を読む。
**offscreen は software scene graph で、フォントの歩きも払わない**(性能.md §計測の各論)ので、**実窓の値とは並べない** —
読むのは同じ撃ち方どうしの差と、部品どうしの比。

| 読む時点 | 撃つ行 |
|---|---|
| タブの無い窓 | `verify-ui window-floor --restore` の、`WindowHarness.begin` が呼ばれた時点(窓が上がった直後) |
| タブを待つ窓(ページがまだ無い) | `verify-ui perf diff --repo <コーパス>` の、同じ時点(頼んだフォルダはまだ着いていない) |
| 開いて行選択と diff まで | 同じ run の、絵の時点(`WindowCensus.reportAt`) |

どの run にも `--no-board --app-env QML_DISABLE_DISK_CACHE=1` を足す(出荷ビルドと同じく、QML を起動のたびにコンパイルす
る。verify-ui の既定はコンパイル済みキャッシュ有り)。動詞の run なので、検証用のドライバが建つ分はどの数にも同じだけ乗っ
ている。

口の作り直し方(commit しない):

- **個数**: `WindowCensus.qml` に、窓の根から `data`・`contentData`・`contentItem` / `background`・Loader の `item`・
  QML で宣言した名前付きプロパティの値を辿り、`String(object)` の型名で数える関数を足して、上の 2 つの時点で
  `Harness.report` する。ハーネス自身(`WindowHarness` / `PageHarness`)の下は数えない。名前付きプロパティの値が別の親を持
  つ Item なら辿らない(参照を持ち主に数えない)。**辿り方に現れない物は数に入らない**
- **メニュー・ダイアログ・カードの先建てを切る**: `PageHarness` / `WindowHarness` が動詞の run で立てる `keepBuilt`
  (ページのと帯の — `RepoPage` / `TopBar` — と、窓の 3 つの席 — `WindowDialogSeat` / `IdentityGate` / `WindowQuitGate` —
  の)を false にする(切らないと製品と違う数になる)。動詞は開く前にメニューやカードを読むので、その動詞は約束の照合で赤く
  なるが、口の行は出る
- **初回の生成**: 絵の時点で、まだ建っていない席 — `active` が false の `Loader` と、`Component` から作るカード
  (`DetailsAuthorCards.built` / `WipPane.builtEolCard`)— を 1 つずつ建て、前後の `PerfProbe.clockMs` の差と建った個数を
  出す。絵の後で窓の中身を増やすので、その run 自体は赤になる
- **プロセスの量**: `harness/singleton.rs` の `report` に、`K32GetProcessMemoryInfo`(WorkingSet / `PrivateUsage`)と
  `GetProcessHeaps` + `HeapLock` + `HeapWalk`(busy のバイトと個数)を読んで行に添える口を足す
- **1 個の重さ**: `WindowHarness.begin` で、部品を `Qt.createComponent("qrc:/qt/qml/platitude/ui/<名>.qml")` から 1 個建
  てて捨てた後、見えない親の下に 200 個建て、前後の上の値の差を 200 で割る。既定のプロパティのまま建てる (`ActionButton`
  は印の無い 1 段の形)ので、形を変えた値は `createObject` に初期値を渡して測る。required property を持つ部品 (行のデリ
  ゲート)は建たない。**見えない Canvas は描かれない**(`InkCanvas.qml` の冒頭)ので、画面に出る印はこれより重い側に立つ
- **起動の段**: `tracing::info!` を `main.rs` の `qml_engine::load` の前と、`TabsModel::default`(最初のオブジェクト)に置
  く
- **メニューを初めて開く 1 回**: `RepoPage.openRowMenu` の、席を建てる行(`commitMenuSeat.active = true`)の前後と開き終え
  た所で、`PerfProbe.clockMs()` の時刻と上のプロセスの量を報告する口を足し、`verify-ui commit-menu` を撃つ

## 個数

| | タブの無い窓 | 開いて行選択と diff まで |
|---|---|---|
| 全部 | 2,859 | 8,311 |
| うち `RepoPage`(空ページ / タブのページ) | 2,161 | 7,672 |
| `NavIcon` | 112 | 304 |
| `PlainTextLabel` / `Label` | 165 / 71 | 620 / 239 |
| `TextMetrics` / `FontMetrics` | 107 / 50 | 422 / 83 |
| `TipAsk` | 75 | 185 |
| `Loader` / `Repeater` | 137 / 48 | 1,357 / 262 |
| `Connections` / `Timer` / `Binding` | 205 / 139 / 96 | 315 / 137 / 93 |
| `ActionButton` とその派生(`NoticeButton`・`Band*Button` 3) | 9(ラベル 13・席 9) | 8(12・8) |
| `CardText` | 8 | 6 |
| `AppMenu` / `AppMenuItem` | 0 / 0 | 0 / 0 |

- **タブの無い窓の 4 分の 3 は空ページ** — タブのページと同じ `RepoPage` で、ペインと常に出ている帯を持つ。契機が来るま
  で出ないカード・ダイアログ・帯は、出る時に建つ(§初回の生成)
- **行は 1 行あたり** グラフ 50 個(41 行)/ 左メニュー 37 個(42 行)/ diff 21 個(47 行)/ ファイル一覧 22 個(38 行)
- **メニューはどれも初めて開いた時に建つ**(ページのメニューは `RepoPage.keepBuilt`、帯の ☰ と名前の 2 枚は
  `TopBar.keepBuilt`)ので、上のどちらの窓にも `AppMenu` / `AppMenuItem` は無い。コミット行のメニューは 1 枚で 1,993 個、
  初めて開く 1 回は建てるのに 54–58ms・開くのに 22–23ms(3 run、他の席の verify が並ぶ台)、ヒープ +9.6MB(確保 6.3 万
  個)・private +11.4–13.4MB — このメニューの数だけは `6631d4e5` の木で測った値で、そこからの違いはメニューの部品に入らな
  い

タブの無い窓に、見せていないまま建っている物(その下の個数):

| 部品 | 個数 |
|---|---|
| `RecoverBand` の殻(物差し 2 本と下線 — 床が読む。行は項目が載っている間だけ) | 45 |
| `AskBar` の殻(物差し 2 本と下線と Esc — 床が読む。中身は最初の問いで建つ) | 42 |
| `NoticeBar` の殻(物差しと Esc — 床が読む。文言と `OK` は最初の報せで建つ) | 26 |
| `PublishFlow`(リモートを書き留めるダイアログは最初に開く時に建つ) | 13 |

`StartupGate`(124)は `begin` の時点では git の確認がまだ出ていて立っている — git が答えると壊れ、開いて diff までの数に
は居ない。

ページの中のペインで、出る契機が来る前から建っている物(§個数 の数え方での、その下の個数。空ページは
`verify-ui window-floor --restore` の絵の時点):

| 部品 | 空ページ | 開いて行選択と diff まで | 出るのは |
|---|---|---|---|
| `DetailsPane` | 469 | 1,336(出ている — ファイルの行 852 込み) | コミットの行を選んでいる間(`!page.wipShown`) |
| `WipPane` | 408 | 408 | 作業ツリーの行を選んでいる間(`page.wipShown`) |
| `DiffPane` | 280 | 1,294(出ている — 行 1,005 込み) | ファイルを開いている間 |
| `NavRail` | 70 | 70 | 左メニューを畳んでいる間 |
| `GraphHeadPin` | 48 | 48 | HEAD の行が画面の外に在る間 |

- **空ページ 2,161 個のうち、右ペインの出ていない側と diff のペインで 約 3 割**(`WipPane` 408 + `DiffPane` 280)— リポジ
  トリが無い間はどちらも出ない
- **開いたページでは、右ペインの出ていない側(`WipPane` 408 か、`DetailsPane` の行を除いた 約 480)が常に 1 つ建っている**
  — どちらも 1 回の選択で出る側になる

## 初回の生成

契機が来て初めて建つ物のうち下の表の分の、建つまでの時間(ms、開いて diff までの run 3 本の絵の時点。§測り方)と、その下の
個数(子と中身だけを辿った数 — 名前付きプロパティも辿る §個数 の数より少なく出る)。建てた後は持つ物(★)と、出ている間だけ
建つ物がある。メニューは §個数、設定と clone のダイアログ・`CommandsPane`・rebase の計画の面は測っていない:

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

- **どれも 1 フレームに収まる**(最も重い `AuthorCard` で 6–10ms)— hover のカードは休止の待ちの後に開くので、その開くフ
  レームが数 ms 延びるだけ
- `StartupGate` は窓の最初のフレームから git が答えるまで立つ — 起動で建ち、答えの後で壊れる
- `AuthorCard` の個数はコミットで違う(空ページでは 101)

## 1 個あたり

ヒープ(KB)/ 確保の個数 / private(KB)/ 1 個を建てる時間(µs、静かな台)。1 本の run の値で、並べる順と前に建てた物で 1 割ほ
ど揺れる — 比べるのは同じ一覧の run どうし:

| 部品 | ヒープ | 確保 | private | 建てる時間 |
|---|---|---|---|---|
| `DialogActions`(ボタン 2 つ) | 421 | 2,577 | 538 | 1,200 |
| `ActionButton` / `NoticeButton` | 207 / 209 | 1,264 / 1,272 | 276 / 245 | 603 / 610 |
| `AppMenuItem`(語だけの行) | 159 | 991 | 181 | 663 |
| `NavHeader` | 155 | 875 | 218 | 808 |
| `RefChipStack` / `RefChip` | 154 / 141 | 820 / 744 | 209 / 173 | 449 / 391 |
| `AppListView` | 115 | 709 | 108 | 509 |
| `NavFactLine` | 104 | 669 | 140 | 486 |
| `NameCell` | 97 | 605 | 108 | 229 |
| `ActionButtonLabel` | 92 | 550 | 120 | 232 |
| `LabeledField` / `HelpText` | 90 / 85 | 576 / 543 | 121 / 115 | 560 / 376 |
| `AvatarButton` / `HeadTrack` | 87 / 87 | 513 / 467 | 130 / 116 | 242 / 166 |
| `CardText` | 84 | 533 | 97 | 422 |
| `PaneScrollBar` / `AutoScrollBar` | 75 / 74 | 497 / 480 | 72 / 78 | 252 / 326 |
| `CodeLine` / `AppMenu`(行なし) | 58 / 58 | 420 / 411 | 76 / 71 | 178 / 121 |
| `LineText` / `CodeChip` | 52 / 51 | 311 / 269 | 56 / 71 | 232 / 152 |
| `GoneBadge` / `CutName` | 44 / 36 | 200 / 251 | 61 / 32 | 91 / 75 |
| `HoverToolButton` / `LineRuler` | 22 / 21 | 130 / 119 | 18 / 25 | 49 / 122 |
| `ShoulderBadge` / `ChangeIcon` | 20 / 20 | 100 / 97 | 30 / 30 | 56 / 42 |
| `NavIcon` / `MiddleAutoScroll` | 17 / 17 | 91 / 122 | 17 / 18 | 40 / 30 |
| `ActionButtonSeat` | 16 | 105 | 20 | 24 |
| `HoldDriver` / `HoldIcon` | 9 / 9 | 72 / 68 | 10 / 11 | 20 / 15 |
| `PlainTextLabel` / `InkCanvas` | 6.1 / 4.1 | 43 / 32 | 5.1 / 5.6 | 9 / 8 |
| 素の `Text` / `Canvas` | 1.5 | 6〜7 | 1〜1.5 | 2 |
| 素の `Item` / `Rectangle` / `Loader` / `Timer` / `Connections` / `TextMetrics` | 0.5〜1 | 4〜5 | 0〜2 | 1〜2 |

- **束ねた部品の重さは、中身とその部品自身の分の和** — `ActionButton` 1 個 207 のうち、描く形のラベルと席の 1 組がラベル
  92 + 席 16 で、残りの半分ほどはボタン自身(`HoverToolButton` 22・`HoldDriver` 9 と、`ActionButton` が自分に足したプロパ
  ティとハンドラ)。席はアイコンと、着ている印だけを建てる。和にぴったりは合わない (`DialogActions` は `ActionButton` 2
  個で 421)。形で変わる分は、bench をこの系に絞った同じ測り方の run で、印の無い形 208 / アイコン付き 242 / 2 段 305。 2
  段は、帯がセルを測る 1 段の語を描かない物差しのラベルとしてもう 1 つ持つ(`ActionButton.lineWord`)ので、 §個数 のタブの
  無い窓のラベル 13 = ボタン 9 + 帯の 2 段 4 つの物差し
- **`AppMenuItem` は行が着ている物だけを建てる** — 見出しと席の印・`!`・回数押しの印と塗りと状態機械・チップの地・先後の
  数・バッジはそれを持つ行にだけ、名前の文の物差しと印と地はその文を持つ行にだけ建つ。形で変わる分は、同じ測り方の run
  で、チップ付き 174 / 見出し 186 / `!` 184 / 回数押し 198 / 名前の文 248 / 枝のカードの行(先後の数・バッジ・席の
  印)300。この行の時間は他の席の verify が並ぶ台で撮った(同じ run の `NavIcon` は 33〜39µs)
- **`NavIcon` は素の `Canvas` の 11 倍余り** — 違いは `NavIcon` が自分に足した分(派生プロパティ 20 本余りとハンドラ 7
  本)。束ねた部品の大半がこれを数個ずつ持つ
- **素の部品はどれも 1KB 前後・確保 4〜7 個**

## プロセスの量と起動(同じ撃ち方どうし)

MB(確保は個数)。開いて diff までとタブを待つ窓は 3 run、タブの無い窓は 1 run:

| | タブの無い窓 | タブを待つ窓 | 開いて行選択と diff まで |
|---|---|---|---|
| WorkingSet | 130.5 | 84.4–84.7 | 345.5–346.3 |
| private | 111.2 | 66.1–66.5 | 320.9–321.6 |
| プロセスヒープ busy(確保) | 52.8(312.0K) | 30.4–30.5(152.3–152.8K) | 158.8(787.7K) |

- **空ページ 1 枚は WorkingSet 46MB・private 45MB・ヒープ 22.4MB(確保 16.0 万個)** — 左 2 列の差
- **起動(3 run)**: 起動から最初のオブジェクトまで 880–903ms — うち `qml_engine::load` の前は 15–17ms で、残りは QML の型
  の読み込み(§import の代金)。その後、帯・ダイアログ・ハーネスまで 142–149ms、タブのページを建ててグラフの最初のフレーム
  まで 375–394ms
- **コンパイル済みキャッシュを使う**(`QML_DISABLE_DISK_CACHE` を外す、キャッシュの温まった 3 run)と、最初のオブジェクト
  まで 697–713ms — **型の読み込みのうちコンパイルが占めるのは約 2 割**。開いて diff までの WorkingSet は
  342.2–343.4、private は 310.9–312.1、ヒープ busy は 150.7 — **private とヒープは 8〜10MB 小さいが、予算が読む
  WorkingSet は数 MB しか動かない**(rules-refs/app-ui.md「qmlcachegen はメモリ対策にならない」と同じ向き)

## import の代金

型の読み込みが、ファイル 1 枚ごとに何を払っているか。使い捨ての口が `Qt.createQmlObject` で中身 1 行の文書
(`Item { property int probe: 1 }`)を 100 枚ずつ読ませ、1 枚あたりの時間を出す(µs、2 巡。パース・コンパイル・生成 1 個込
み)。文書の置き場(URL)と import だけを変える:

| 文書の置き場と import | 1 枚あたり |
|---|---|
| ハーネスのディレクトリ、`import QtQuick` だけ | 532–555 |
| 同 + `QtQuick.Layouts` / `QtQuick.Window` / Rust の `platitude` | 516–545(足しても動かない) |
| 同 + `QtQuick.Controls.Fusion` | 577–588 |
| 同 + `platitude.ui` | 2,057–2,106 |
| 製品のディレクトリ(`ui/`)、`import QtQuick` だけ | 1,607–1,626 |
| 同 + `platitude.ui`(製品のファイルの書き方) | 1,823–1,838 |

合成のモジュール(部品 N 個・singleton S 個の qmldir を `QML_IMPORT_PATH` で足したもの)を import した時:

| N / S | 1 枚あたり | N / S | 1 枚あたり |
|---|---|---|---|
| 11 / 0 | 557–575 | 11 / 11 | 1,221–1,389 |
| 267 / 0 | 843–845 | 267 / 11 | 1,488–1,608 |
| 1,000 / 0 | 4,024–4,034 | 1,000 / 11 | 4,755–4,877 |
| 267 / 1 | 940–974 | 11 / 40 | 2,779–3,716 |

- **qmldir の singleton は、その qmldir が見える文書 1 枚ごとに 1 個 約 65µs** — 使っているかに依らない。`2e474242` の製
  品の qmldir は 14 個なので 1 枚 約 0.9ms
- **部品の数にも乗る** — 267 個で 1 枚 約 0.25ms、1,000 個で 約 3.4ms(数の 2 乗に近い)
- **製品のファイルは自分のディレクトリを暗黙に import する**ので、`import QtQuick` だけの文書でも `ui/` に在れば約 1.1ms
  を払う。明示の `import platitude.ui` はその上に 約 0.2ms を足すだけ(同じ singleton を 2 度は解決しない)
- **製品の 266 枚で 約 0.3 秒** — 型の読み込み(880–903ms からコンパイルの 約 2 割を引いた残り)の 4 割ほど。 singleton を
  名指すファイルは `Theme` 201・`Metrics` 72・`Words` 62・`WordFace` 39・`Hand` 13・`MarkSeat` 12・`Tips` 10・ `KeyMenu`
  8・`OutsidePress` 7・`NavFacts` 5・`MiddleHand` 4・`Ink` / `Motion` / `Hovers` 各 1 で、266 枚 × 14 個 = 3,724 回の解
  決のうち、名指しに当たるのは 436 回
- **ファイルを 1 枚足すと、その 1 枚の約 1.1ms に加えて、部品の数に乗る分が全部のファイルで増える**
- **singleton 12 個を 1 つの別モジュールへ移した試作**(`keep/shared-singletons-prototype` — 製品の qmldir に残すのは
  `Theme` / `Metrics`、他は使うファイル 139 枚だけが import する)では、起動から最初のオブジェクトまで 734–791ms (3 run。
  同じ撃ち方の `2e474242` は 880–903ms)
- **今の製品は、その 12 個を責務ごとの 6 つのモジュール(`ui/<名>/`)に置き、読むファイルだけが import す
  る**(rules/app-ui.md §QML モジュール)。この § と §プロセスの量と起動 の値は `2e474242` のもので、今の木の起動の値は未
  取得
