---
name: verify-ui
description: platitude-gg の UI 動作確認・スクリーンショット検証・アプリ起動(「rebase して起動」含む)の前に必ず読む。verify-ui の使い方・PGG_AUTO_ACT 動詞表(verbs.md を Grep)・Windows / Linux の検証の罠は全部ここ。
---

# UI 動作確認(ヘッドレス検証)

**クラウドセッション(`CLAUDE_CODE_REMOTE=true`)ではここの何も撃たない** — 触った動詞は報告で「手元に残る検証」として名指すだけ(internal-docs/クラウドセッション.md)。

**ヘッドレス動確は `cargo xtask verify-ui <動詞> [引数]`** — release ビルド → `screenshot saved=true` 判定と PNG 保存まで 1 コマンド。`--no-build` で連続実行、`--preset` / `--repo` で対象指定、素材だけ欲しければ <!--cmd:demo.repo-->`cargo xtask demo-repo <preset>`。

presets / options の一覧は `cargo xtask` の USAGE(引数なし実行)が正。

**触った動詞は gate が選ぶ** — 各 run は「どの QML 部品を出したか」を `census=` で報告し、PASS した run が `crates/xtask/verb-census.txt` に記録する(commit する生成物。手で触るのは消した / 改名した動詞の行と、段分け表で `twin` とした行を落とす時だけ)。**動詞の行を足したら段を決める**(rules-refs/app-ui.md)。**census に無い部品(新規・まだ誰も撮っていない)は gate が名指しで止まる** — その部品を出す動詞を 1 度 `verify-ui` で単独に撃てば記録され、census の diff を見て commit してから gate を撃ち直す(`gate --verb` は動詞を走らせる前に止まるので、この登録には使えない)。`--repo` / `--restore` / `--config-dir` の run とコンテナの中の run は記録されない。**census に行の無い動詞は gate が一度も回さない** — must_say の表に在っても、誰かが 1 度 `verify-ui` で撃つまでは手で撃つ以外に網が無い。**gate が撃つ動詞も自分の行を書き直す**ので、QML を変えた後の gate は「census を書き直した」と言って**スタンプせずに赤で止まる** — 生成物なので diff を見てそのまま commit し、もう一度撃つ。合否に関係ない反復(揺れの計測・絵を見るだけ)は `--no-census` で撃つと生成物が動かない。

**目視は等倍以上で、判定する所を切り出して見る。** 何枚も並べて縮小した一覧は「どの絵が撮れたか」の確認**だけ**に使う — 縮小の補間は上に載った数 px を消す。文字・枠・重なりを判定する時は、その行を 3 倍以上・**NEAREST**(補間なし)で切り出す —— **`cargo xtask shots crop <png> --at <x>:<y>:<幅>:<高さ> [--scale <n>]`**(既定 3 倍。整数倍のみ)。切り出しは元の絵の隣に置かれる(`app.png` → `app-crop-24-96-220x44.png`)ので、そのまま Read で読む —— **切り出しの道具はこれだけ**。空白・崩れに見えた絵は、直す前に保存ファイルの画素(crop / SHA-256)で確かめる。

**ユーザーに見せる絵は board へ出す**(チャットのペインは画像を縮めるうえ拡大できない)。**明示的に撃った** `verify-ui` は撮った PNG を自動で board に載せる。**`gate` / `check` が回す verify の run は載らない**。**board に載せるのは「ユーザーが求めた絵」と「その指示に直接関係する絵」だけ**(ユーザー決定)。**反映の指示の後に自分の確かめだけで撃つ run は `--no-board`** — 指示の後に board へ載せた絵は、下げてもユーザーの次の発言まで反映を止める(CLAUDE.md §Git 運用)。**窓は 1 枚 — 見せ方の既定は「F5」**(ユーザー決定): 絵を載せたら**「board を更新した。F5 で反映される」と伝えるだけ**にする。<!--cmd:shots.open-->`cargo xtask shots open` は窓が立っていれば開かずに「already open — press F5」と答える。`shots add` は最後に今どちらかを言う(`a window is open — F5 there` / `no window open yet`)ので**それに従う** — **<!--call:shots.open-->`shots open` を撃つのは後者を言われた時だけ**。`cargo xtask shots open --again` は**ユーザーが「閉じた」と言った時だけ**。

手で作った切り出し・比較シートは `cargo xtask shots add --label "<この絵で何を見るか>" [--verb <動詞>] <png>...`(**`--label` は必須**)。**ラベルは日本語で書く**(日本語の字を 1 つも含まないラベルは拒まれる)。識別子・動詞・ファイル名はその綴りのまま名前の中へ入れてよい(`"AskBar.settled を待った絵"`)。**`--label` は `verify-ui` 側にもある** — 見せるために撮る run には付ける(規則も拒み方も `shots add` と同じ)。**before / after は必ず 1 つの run に横並びで載せ、両側を出す** — `cargo xtask shots add --label "<何が変わったか>" [--verb <動詞>] --before <png> --after <png>`(ユーザー決定)。flaky 掃引のように同じ絵が何十枚も出る run は `verify-ui --no-board` で撃つ。

**絵を撮るには席が要る** — board に載せられるのは**この会話が claim を持つ**席 a〜f からだけで、**本体 checkout(と roster 外の worktree)・claim の無い席・他人の席からは拒否される**(`shots add` はエラー、`verify-ui` は `board: not updated (…)` と言って判定自体は続く)。**スクショを頼まれたら `cargo xtask seat` で席を取り、その席のビルドで撮る**。**絵は claim を戻さない — land の後に絵を載せ直さない**(land で claim が外れた席からは載らない。載せ直しで席を握り続けると他セッションが使えない)。**ユーザーの承諾が要る絵は land の前に載せ、承諾を得てから反映する** — 反映の指示より後に載せた絵が席に残っていると `land` は何も動かさずに止まる(CLAUDE.md §Git 運用)。同じ席で同じラベルを撮り直すと**前の run は置き換わる** — **残したい比較は別ラベルか 1 run 複数枚で撮る**(`— linux` は別ラベルなので両 OS のペアは残る)。**run が落ちるのは席の仕事が終わった時だけ** — `land` した時と、その席が次の仕事のために配られた時(`cargo xtask seat`)。見せ終わった絵は <!--cmd:shots.prune-->`cargo xtask shots prune`(引数なしで**自席だけ**。隣の席の絵を触るのは `--seat <letter>` / `--seat all` で指名した時だけ)、**捨てた案は `cargo xtask shots prune --label "<その run の名前>"`**。

**board は上から下へ、載せた順に読む**。**載せる順は次の優先順で決める**(ユーザー決定):

1. **ユーザーが明示指定した順** — 明示指定があるなら、2 以降は無視されうる
2. **動かしているマシン順** — 今動かしているマシンで撮った絵が先、他環境は後
3. **説明における順** — 説明が 1〜3 なら 1 → 2 → 3

両 OS を見せる時は **Windows の 1 → 2 → 3 を全部載せてから、Linux の 1 → 2 → 3**。見せると決めた絵は上の順で `verify-ui --no-build` / `shots add` を順に撃って載せる。

**動詞はまとめて撮る(1 ターンに複数)** — 同じビルドで撮れる動詞は 1 個の複合コマンドに並べて 1 ターンで実行し(2 発目以降は `--no-build`)、PNG の目視も 1 ターンに複数枚まとめて読む。段 2 と重なる時は `cargo xtask check --verb '<動詞と引数>' --verb …` の一括(ホスト / コンテナ並列)が最速。1 動詞ずつ「撮る→見る→直す」を回してよいのは、直前の絵が次の編集を決める時だけ。

**判定には書き込みの失敗も入る** — `write failed` が 1 行でもあれば FAIL(撮れた PNG は「届かなかった状態」のもの)。**`--allow-write-failure` を付けてよいのは「その拒否がこの動詞の見せ物である」時だけ**(対象動詞の全列挙は `cargo xtask` の USAGE が正)。引数の渡し忘れも同じ行に出る(`delete-branch-refused` を引数なしで撃つと `git branch --delete -- ''` が拒まれ、`not merged` の絵は撮れていない)。

## 起動 fast path(「rebase して起動」等、起動だけの要求)

ユーザーが待っているのは**窓が出ること**。前提は「rebase が通り、release exe がビルドできる」ことだけ(fmt / clippy / test / verify-ui / linux 系は段 2 で)。

1. **停止 → rebase → ビルド → 起動 → 生存確認まで 1 個の複合コマンド**(escape を両方付ける)。**素のまま撃つ**(パイプ・コマンド置換は下記の罠、hook が deny):

   ```bash
   PGG_ALLOW_REBASE=1 git rebase main && PGG_ALLOW_GUI=1 cargo xtask launch
   ```

   `launch` が自ツリーの居残りプロセス回収(`cargo xtask kill` 相当)→ release ビルド → 切り離し起動 → 生存確認まで行い、Qt の PATH も自分で解決する。rebase 不要の要求なら <!--cmd:app.launch-->`PGG_ALLOW_GUI=1 cargo xtask launch` だけ。冷えたツリーの release ビルドは既定の 2 分を超えうるので、**前景のまま timeout を上げる**(理由は下の 3)。**落とすのは自ツリーの居残りだけ** — `kill` / `launch` がそれだけを落とす(画像名 kill は hook が deny)。
2. **`launch` の行が最後のツール呼び出し**。窓が出たら報告してターンを終える。起動を待たせてよいのは rebase の衝突と build エラーだけ(衝突を解決したら、この fast path の続きで起動まで行く)。
3. **起動の報告がターンの終わり** — **ターンが終わらない間ユーザーの次の指示はキューに載ったまま届かない**。以下はどれもターンを延ばす:
   - 起動そのものを run_in_background で撃つ(hook が deny)/ 報告と同じターンで `check` 等を run_in_background で始める(完了通知がエージェントを起こし直す)
   - `Get-Process` / `xtask seats` / ログの tail で生存や pid を確かめ直す(生存確認は `launch` が済ませている)
   - 窓が閉じるのを待つ・ユーザーの操作を待つ・撃ち直す・スクショを撮る

   起動の報告には**段 2 が未実行であることを 1 行添える**。**その段 2 は窓を立てたまま回してよい**(窓は複製から立つ)。

**`cargo xtask launch` をパイプ・コマンド置換に通すと、ターンが窓の寿命だけ返らない** — 切り離した窓がシェルの作ったパイプの書き込み端を掴んだまま生き続け、読み手が EOF を見ない(`Stdio::null()` も `DETACHED_PROCESS` も `cmd /c start` も外れない)。**パイプを書かなければ掴まれない**(`2>&1` だけ・ファイルへの `>` は読み手が居ないので無害)。

## PGG_AUTO_ACT 動詞表

**全動詞の正本は [verbs.md](verbs.md)** — 動確の前に、使う動詞の項を動詞名で Grep して必ず読む(引数を省くと何も撮れない動詞がある)。**動詞を足したら verbs.md へ追記する**。**報告行で判定すると書いたら `crates/xtask/src/verify/verbs/*.rs` の表にも行を足す** — verbs.md の `must_say` は散文で、機械はそちらを読まない。表に行が無い動詞は**絵だけで判定される**ので、壊れた run が緑で通る。

**ダイアログ・メニューの見た目は headless で撮れる**: `WindowShotMirrors.qml` のオーバーレイミラー(`ShaderEffectSource`)が **overlay.png** を app.png と並べて保存する(ロック状態と無関係)。オーバーレイ自体の grabToImage は "no QML engine" で不可、ミラーが唯一の経路。

**白紙の overlay.png は「その時点で何も開いていなかった」の意味**。報告行 `overlay saved=<bool> popups=<n>` の **`popups=` がオーバーレイ自身が抱えていた数**(メニュー 1 / サブメニューを開けば 2 / モーダルは dimmer を伴って 2 以上)。`commit-menu` / `reset-menu` はこの行が must_say — **overlay が主題の動詞には同じ 1 行を足せる**(`verify/verbs.rs`)。「閉じたか」は `Popup.opened` の報告行で読む。

**overlay が空でない run は 3 枚目 `scene.png` を出す** —— app.png と overlay.png を**描かれた座標のまま重ねた 1 枚**で、**窓の縁(Windows の窓モードの 1px)が写るのはこの 1 枚だけ**(app.png は本体だけ = Windows では窓より 2px 狭い)。「開いたものは、それを開けたものに対して**どこに**立っているか」(hover の席・カードの密着・メニューの掛かり方)はこれでしか読めない。報告行は `scene saved=<bool>`。**hover の席を見せる時はこの 1 枚を board に出す**。

## 場面ごとの各論(このディレクトリ。その場面に入る前に読む)

- 動詞を足す・直す・反復する → [implement.md](implement.md)(壊れない動詞の実装と反復)
- hover・ツールチップ・カードの絵を撮る → [hover.md](hover.md)(hover の絵の撮り方(仮表示))
- Linux(コンテナ)で撃つ → [linux.md](linux.md)(Linux(コンテナ)での動確 — Done は両 OS)
- macOS(CI の runner)で撃つ・mac の絵を読む → [mac.md](mac.md)(macOS での動確 — 机が無いので CI の runner が撃つ)
- Windows での起動・デバッグ → [windows.md](windows.md)(Windows での実行・デバッグの罠)
