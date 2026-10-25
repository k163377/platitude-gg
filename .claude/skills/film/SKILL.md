---
name: film
description: platitude-gg の動き(hover・カード・メニュー・ドラッグ・外からの変更への追従)をアニメーション(APNG)で撮る・before/after を動きで見せる時に必ず読む。`cargo xtask film` の台本の書き方・側ごとの撮り方・焼き方・board への出し方・罠は全部ここ。
---

# 動きを APNG で撮る(`cargo xtask film`)

**クラウドセッション(`CLAUDE_CODE_REMOTE=true`)ではここの何も撃たない**(internal-docs/クラウドセッション.md)。

1 枚の絵では言えない挙動 —— 手を置いて待つと出るカード、メニューの後に戻るツールチップ、外の commit で行が動いた時のカード、ドラッグ —— は、**台本を書いてヘッドレスの窓に演じさせ、コマを APNG に焼いて board に before / after で出す**。入力は QtTest の `TestEvent` が窓へ送る本物のマウス・キーのイベントなので、hover は Qt 自身のもの(動詞の仮当てではない)。

- **画面は点けない** —— 撮影はヘッドレス(offscreen)だけ。実窓・本物のマウスでの撮影は、ユーザーがその場で明示した時だけ(表示が点き、カーソルを奪う)
- **Windows だけ** —— board に載せるのは Windows の run だけ(verify-ui スキル)で、Linux の撮影経路は無い
- **撮影はレビューの代わりにもなる** —— 演じさせると、枝の取りこぼし(メニュー後の戻り・行が消えた時の手)が絵に出る

## 流れ

席で撃つ(`cargo xtask seat`)。台本・repo・コマは全部その席の `target/film/` に置かれる(gitignore 済み)。

1. **台本を書く** —— `target/film/scenes/<名前>.film`(§台本)。座標を拾うには、`view 0 0 1440 900` と `tick 1000` の下見の台本を 1 本撮って、コマから読む
2. **側ごとに撮る** —— 側は `before` / `main` / `after` / `now` / `opt2` / `opt3`(帯の名札は 直す前 / main / 直した後 / 今 / 案 2 / 案 3)。1 側ごとに:
   1. 木をその側に合わせる —— `main` は `git switch --detach main`、`before` は直す前の commit へ `git switch --detach <commit>`、`now` / `after` は席の枝のまま
   2. <!--cmd:film.in-->`cargo xtask film in` —— film 動詞をその木へ入れる(QML 部品 1 つと 8 ファイルの置換。commit しない)
   3. `cargo xtask film shoot <台本>... --side <側>` —— 最初の台本が release を組む(2 本目以降は組まない。`--no-build` で 1 本目も)
   4. <!--cmd:film.out-->`cargo xtask film out` —— 同じ置換を逆に当てて外す。**`git status --short` に自分の変更しか無いのを見てから**木を戻す(`git switch <席の枝>`)
3. **焼く** —— `cargo xtask film bake <台本>` が撮った側を全部、1 本の時計で `target/film/<名前>/<側>.png` にし、**字幕ごとの最後のコマ**の path を名指す。**その PNG を Read で見て、ラベルで言う変化が写っているかを確かめる**(コマには手・字幕の帯が描いてある)。いつ何が変わったかは `cargo xtask film changes <台本> --side <側>`(変わったコマ・時刻・変わった箱)
4. **board へ出す** —— 1 場面 1 run で `cargo xtask shots add --label "<何が変わるか>" --before target/film/<名前>/main.png --after target/film/<名前>/now.png`(順・ラベルの規則は verify-ui スキル)

## 台本

1 行 1 項目。`#` で始まる行と空行は読まない。設定と字幕は語で、手の動きは `<動詞>:<引数>` で書く。

```
# HEAD の行のカードに手を入れたまま、外で commit する
repo eol
prep branch zz-probe
layout sidebar_width 300
view 260 95 1000 420
cap -
m:900,700
w:600
cap HEAD の行に手を置く
g:900,700,600,139,450
w:1500
cap 外で git commit(HEAD が新しいコミットに)
git:commit --allow-empty -q -m more
w:9000
```

| 語 | 意味 |
|---|---|
| `repo <preset>` | 台本の repo。**最初に撮った側の木が 1 度だけ建て、全部の側がその写しから始める**(preset は木によって中身が違うことがあり、別々に建てるとその差が before / after の差に写る)。preset を替えると建て直す。`--fresh-base` で強制 |
| `repo made` | 手で作った repo を写す: `cargo xtask demo-repo <preset> --at target/film/<名前>/pgg-film-base` で建てて手を入れておく |
| `prep <git の引数>` | 窓が開く前に写しの repo で撃つ git(例 `prep branch zz-two`・`prep tag v1`) |
| `layout <key> <value>` | state.toml の `[layout]` の上書き(既定は窓 1440x900・左パネル 260・詳細 400 など、`film/shoot.rs` の `state`) |
| `view <x> <y> <幅> <高さ>` | コマに写す範囲(窓の座標)。複数書くと横に並ぶ |
| `scale <n>` | 整数倍に補間なしで拡大(1px の主張に) |
| `tick <ms>` | 何もしていない間のコマの間隔(既定 80)。手を動かした瞬間は別に 1 コマ撮る |
| `cap <字幕>` | ここからの字幕。**`-` で始まる字幕は段取り** —— そのコマは焼く時に落ち、各側の時計はその後の最初のコマから数える。台本は字幕で始める |

手の動き(座標は窓の座標):

| 動詞 | |
|---|---|
| `w:ms` | 待つ |
| `m:x,y` / `g:x0,y0,x1,y1,ms` | 動かす / 滑らせる |
| `l:x,y` / `r:x,y` | 左 / 右クリック |
| `d:x,y` / `u:x,y` | 左ボタンを押す / 離す(間の `m` / `g` はボタンを運ぶ = ドラッグ。押している間の手は橙) |
| `wh:n` / `wr:d` | ホイールを n 段 / 角度 d |
| `k:<キー>` / `kc:<修飾>,<キー>` | キー(esc menu enter up down left right tab f f2 f10 space delete backspace home end pageup pagedown)/ 修飾付き(ctrl shift alt) |
| `t:<文字>` | 文字を打つ |
| `fw:<path>\|<中身>` / `cp:<元>\|<先>` | repo のファイルを外から書く(`\n` で改行)/ 別のファイルで上書きする(エディタの保存) |
| `git:<引数>` | 外から git を撃つ(空白で区切る) |
| `rt:<関数>\|<引数>` / `pg:<関数>\|<引数>` | この窓のタブの書き込み(`@head` = HEAD の commit)/ ページの関数(`pg:toggleRecover`・`pg:openDiff\|unstaged\|notes.txt\|`) |

打ち間違いは release を組む前に `shoot` が行番号で止める。

## 罠

- **外からの変更の読み直しには数秒かかる** —— `git:` / `fw:` / `cp:` の後の `w:` は 9〜12 秒取る。短いと読み直しの前で台本が終わり、片側だけ変化が写らない(側ごとに読み直しの時機が揺れる)
- **台本を直したら全部の側を撮り直す** —— `bake` は別の版の台本で撮った側を混ぜると止まる
- **注入中は commit できない**(hook が止める)。**側を替える前に必ず <!--cmd:film.out-->`cargo xtask film out`** —— 入れたまま `git switch` すると置換が向こうの木へ運ばれる
- **<!--cmd:film.in-->`cargo xtask film in` が「〜 times」と止まったら** —— 置換の元の字が製品側で動いた。`crates/xtask/src/film/inject.rs` の表を今のファイルへ合わせる(`every_edit_finds_its_text_once_in_this_tree` が gate で同じことを言う)
- **部品を直す時は `crates/xtask/src/film/WindowFilmActs.qml.in`** —— `shoot` が木の中の部品を自分の版へ書き直し、`--no-build` でもビルドする。QML のメソッド名に `unescape` は使えない(JS の既定関数と重なり、ハーネスごと載らない)
- **オフスクリーンのカーソルは合成のイベントで動かない** —— 引数なしの `popup()` のメニューは `KeyMenu.filmHand`(注入が足す)で手の位置に開く。手の届かない所で開くメニューはこの経路の外
- **詳細ペインの名前の上を通るドラッグは author カードを開く** —— 押している間も待ち(600ms)が進み、名前の上で離すと即座に開いて日付の行を隠す。名前の外(アバターとの 4px、x=1084)で離し、次の手は名前の上を通らない経路で回る
- **チップの一覧は名前が 2 つ以上のチップでしか開かない** —— 名前が 1 つの preset なら `prep branch …` / `prep tag …` で足す。**一覧は Esc で閉じない**(フォーカスを取らない popup)。**畳んだレールのセルは enter でしか頼まない**ので、押下で閉じた覗きは揺れても歩いても開かない(出て入り直すか押すだけ)。見切りの違いを撮るなら、席が頼みを預かっている場面(手の下でグラフが戻った直後のチップなど)を選ぶ
- **段取りで開いた物は段取りの `w:` で待つ** —— 部品はページが落ち着く(`PageSettled.settled()`)まで待って演じ始めるが、`pg:` で開いた面・段取りの手で出したカードはその後の `w:` で待つ
