# verify-ui 各論: hover・ツールチップ・カードの絵を撮る

## hover の絵の撮り方(仮表示)

テスト環境からアプリへポインタの hover 入力は送信できない(windows.md §Windows での実行・デバッグの罠)が、**絵は撮れる** — テスト用ドライバから、実 hover と同じ表示経路へ状態を渡す。**絵は次の 3 つの道で出す**。上から順に検討する:

1. **既にその状態の動詞がある** — [verbs.md](verbs.md) の一覧を見た目の種類から逆引きする。ツールチップ(attached ToolTip): `signature-tip` / `stash-tip` / `path-tip` / `tab-name` / `fetch-hover <失敗数>`(いずれも overlay.png 側。**意匠は共有インスタンス 1 つ** = `SharedToolTip.dressToolTip` なので、どれか 1 枚で全ツールチップの見た目が言える)。hover カード: `row-card` / `ref-list-card` / `author-card-open` / `co-authors-open` / `badges-hover` / `eol-hover` / `eol-commit` / `avatar-hover`。ポインタの下で色・印・道具が変わる形: `tab-mark` / `card-note-lit` / `line-tools` / `hunk-tools` / `stage-many` / `graph-divider` / `graph-bar` / `nav-peek` 系 / `avatar-row-lit`。
2. **配線済みだが動詞が無い** — 動詞を 1 つ足すのが正道で、**安い**: xtask は動詞を検査せず `PGG_AUTO_ACT` へ素通しするので、実装は QML の分岐と完了 predicate — ページ内は `AutoActDriver.qml`、ウィンドウ横断(タブ・identity・窓)は `WindowAutoActDriver.qml`。作法は 3 点: **実 hover が書くのと同じ 1 つのプロパティ / シグナルへ書く**(`eol-hover` / `pointAtTab` が手本。書き先が無い形なら先に `*PointedAt` / `pointed` の 1 本を切る — 形の一覧は rules-refs/app-ui.md)・**ページ側の表示条件の判定を通す**(`row-card` の注意 = 直接呼出しは出さないはずの場面まで開いて緑になる)・**出力側 predicate を観測して `finishAutoAct()` する**。
3. **未配線・意匠検討の仮当て(撮影用の一時表示)** — 使い捨てパッチで出す。**worktree に当てるだけ**。出したい状態は**仮の bool 1 本に束ねて、見た目の条件へ `|| <その bool>` を足す**(意匠が採用されたらそのまま道 2 の書き先になる)。QML は release exe 埋め込みなので**パッチのたびにビルドが要る**。撮影は周囲の状態を作る既存動詞に乗せる — 仮表示は無条件に出るので、どの動詞の PNG にも写る。**複数の的の棚卸しは撮影用に一括表示して 1 枚に集める**。往復しそうなら revert の前に `git diff > force-<何>.patch` で保存する。**ツールチップを仮当てで出す時は `tipForced` を後から立てる** — `ToolTip.delay: 0` と組にすると**生成中に `text` のバインディングより先に開いて、字の無い空の箱**が写る。**`delay: 0` + 開いた後に立てる**(メニューなら `onOpened`)。撮影は完了の 1 フレーム後なので、既定の 600ms を待つ余地は無い。

実窓でしか見えないのは OS カーソルとの重なりだけ。hover の**配送規則そのもの**(どこに handler を置くと立つか)の検証は使い捨ての qmltestrunner シーン(rules-refs/app-ui.md)。
