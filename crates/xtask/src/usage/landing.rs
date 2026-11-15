//! The usage page's landing, and the seats and trees a landing works in.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str = "  land [<branch>]
      Put a branch on main — the one sanctioned way (CLAUDE.md Git 運用;
      the land skill). The pre-shell hook lets it through only on the
      permit the user's latest message opened by typing /land, and the
      fast-forward is what spends it: one message, one landing that moved
      main; a landing that stops short spends nothing. In the branch's own
      worktree: rebases the branch onto main when it is behind (a rebase
      that stops is walked back), runs `gate` there (cached steps are not
      paid twice), and only then fast-forwards main — in the primary
      checkout when it sits on main, or on the bare ref (reattaching a
      detached primary) when main is checked out nowhere; the
      reference-transaction hook checks the stamp on the way. History
      stays linear and a landed seat stands at main's tip. Refuses a
      branch checked out nowhere, a dirty seat, a seat that holds main, a
      seat that is not the head of the line of landings (`land line`
      below), and — before anything moves, and again just before main does
      — a seat that put runs on the board after the user's latest message
      (the one the session's permit records): what went up after the ask
      is UI the user has not approved, so the landing takes them off and
      leaves the permit waiting for the user's next message, and the
      session asks for that approval and lands on the next /land. Reads
      the permit again before the rebase and just before main moves: a
      landing run in the background outlives the user's next message,
      which closes the permit, so the landing stops there with nothing
      moved. A landing that stops once it has taken its turn — a rebase
      that stopped, a red gate, runs awaiting approval found after the
      gate, a closed permit, a refused fast-forward — leaves the line of
      landings, and the user's next /land puts the seat back at the end;
      so does a branch with nothing to land, whose /land ends there with
      the permit unspent. A refusal ahead of the turn (a dirty seat, a
      branch checked out nowhere, a seat behind the head) is the session's
      to put right and land again.
      A landing that moved a version the shipped build is made from (Qt's
      pin, the toolchain, a crate the app's binary resolves) tags main
      before and after it (deps/<date>/before, …/after) and pushes the
      pair where main goes (internal-docs/反映前テストの機械化.md
      §版を動かした land のタグ).
      Bare `land` from a seat lands the seat's own branch. Once main has
      moved it clears what the session running it made: the branches its
      git created (the reference-transaction hook writes each one's maker
      down), the branch of the tree the app made for the session to start
      in (written down as the session starts, or else found in its
      transcript), and the trees, seats aside, that have one of them out —
      each when main, or the branch as it stood before its rebase, holds
      all it holds. A tree with uncommitted files, a lock, work of its own
      or a process in it stays, and is named with every branch of the
      session's that stays. A tree the session started in that a process
      still works in is left behind — the terminal tab the app opens there
      outlasts the session's work — and any later landing judges it the
      same way and takes it once it can. Nothing else another session, the
      app or the user made is judged or named.
      The seat's claim goes back to the roster with it, whoever's it was:
      the tree is empty at main's tip, so the letter is free for whoever
      asks next (`seat`). A lock written by hand stays.

  land line
      The line of landings and this session's place in it. The prompt hook
      puts a seat in the line when the user's message is /land
      (`.permits/<session>.line` beside the permit) and takes it out on
      any other message; a seat stands in the line while its permit is
      open, it has not left, and no other conversation's claim is on its
      seat (its own is known by session id or by the desktop app's
      session; a seat nobody claims contradicts nothing). A conversation
      that goes on under a new session id (an interrupted prompt edited,
      /clear) takes its old id's place away at its first message, and
      `seat release` ends the place with the claim. Oldest /land first, to
      the millisecond, never overtaken: the head lands, the next prepares,
      and the rest wait. For the next, it says what the head's landing
      changes among the rule and design documents — a stop (CLAUDE.md, the
      land / review-points / verify-ui / gate SKILL.md: prepare nothing),
      paths to read under .claude/ and internal-docs/, or that the diff
      could not be read — as the land skill's stages go.

  land wait next
  land wait head
      Wait until this seat is next in the line (or already the head), or
      until it is the head, then say the place and what that stage does.
      Spends nothing meanwhile — a look at the line every few seconds — so
      the session runs it in the background and ends its turn; the
      process's end is the wake-up. Ends with an error the moment the seat
      is not in the line: its permit closed, it left, or its /land was
      typed outside a seat; a line it cannot read (git not answering) is
      read again for a minute before that ends it too. A head the awake
      holder found nothing happening for, for two hours after the seat
      reached the head, is let go by whoever reads the line next.

  land leave <why>
      This seat leaves the line of landings, for what stopped it: a
      landing it found it cannot make without the user's word, at any
      stage — the next seat's own code needing more than plain fixes, the
      head's review raising a question, a red unrelated to the diff,
      pictures awaiting approval — before the user is asked. A landing
      that stops past its turn, or has nothing to land, leaves on its own.
      The user's next /land puts the seat back at the end; nothing else
      does.

  land hold
      One background task of this session kept running while it stands in
      the line, started in the background at /land: the desktop app stops
      a session it counts idle under memory pressure, and a session with
      no background task left counts as idle in the first seconds of a
      turn a notice starts. The session's own next tool call after it
      stops standing ends the hold and waits for it, so the notice of its
      end joins that call's result. It also ends when a newer hold of the
      session starts, when the Claude process is gone, and when the
      conversation went on under another session id. It steps out of the
      build slot as a landing does.

  kill
      Reap this tree's app processes — the ones holding this tree's exe
      against the next link, or its store lock against the next window —
      and nobody else's. The pre-shell hook points image-name kills
      (taskkill /IM, Stop-Process -Name), which reach every seat and the
      user's own window, at this instead.

  launch [--no-build]
      Real-window start for 「起動して」 asks (the verify-ui skill's fast
      path): reap this tree's stale runs, build release, start detached,
      and confirm it outlived its first second. Run it with
      PGGDEV_ALLOW_GUI=1 in front — a real window is the user's ask. The app
      keeps a per-tree settings store on its own, so seats never fight
      over one instance lock. Somebody is waiting at the screen for this,
      so its build takes the machine ahead of every test and behind a
      landing (`budget`, and it says so when it waited); the window itself
      holds none of it.

  seat [release [<letter>]|takeover <letter>]
      Hands this session a worktree seat, and takes no argument: the
      letter is the answer (CLAUDE.md ビルド・テスト). Letters are tried
      until a lock is created on one (git's own lock file, made only where
      none stands), so the seat comes back already claimed: the lock alone
      decides between two sessions, and a letter picked off a survey is
      one the other session can read the same way. A letter taken from the
      roster comes back ready for a fresh stretch: claimed, with nothing
      uncommitted, on its own branch at main's tip, its board runs swept,
      created if the roster never made it. The tree this session already
      stands in comes back instead, as it stands, branch and pictures
      alike, whether its claim came off by landing or by a release. Asked
      from the checkout, the letter the session's transcript last put it
      in is asked for first, and the other free letters go quietest first;
      a letter somebody else's claim is on is passed over either way.
      Asking twice gives the same seat. It prints the path EnterWorktree
      wants, the only one the entry hook lets through. When every letter
      is held or carries work it fails, names what stands in the way, and
      that is where the session stops — seats are not added past f, and a
      letter changes hands only on the user's word.
      A claim is a conversation's, and no process is asked about it: the
      app restarting under a session, the machine sleeping (the SessionEnd
      every open conversation is handed), a tab left open — none of it
      lifts a claim. Nor does a new session id: a claim names the desktop
      app's session too, under which a prompt edited after an interrupt,
      or a /clear, goes on with a new id that renames the claim at its
      next touch. Three things do lift one. `land` hands the seat back
      when the branch reaches main, whoever's claim was on it; a session
      that goes on working in the tree takes it again at its next edit — a
      picture takes nothing, and none goes up from a seat the session does
      not hold, nor does a compaction or a resume, which carry a
      conversation on rather than begin one there.
      `seat release [<letter>]` hands a seat back without landing and
      names what it still carries; the letter is needed only when the
      session holds more than one. And `seat takeover <letter>`, behind
      PGGDEV_ALLOW_TAKEOVER=1, moves the letter to this session whoever holds
      it — a live session, one that is gone, nobody — with the tree as it
      stands: nothing moved to main's tip, the board's pictures kept, a
      tree that went away grown back on the letter's own branch. The
      pre-shell hook lets that verb through only behind the flag, which a
      session writes for an instruction that asked and for nothing else.
      Freeing a letter somebody else holds is a takeover followed by a
      release; a person's own `git worktree lock` is nobody's to take, and
      comes off by hand.

  seats
      Where the six worktree seats a-f stand right now, one line each:
      branch, whether HEAD sits at main's tip, commits ahead of main
      (main..HEAD), uncommitted changes (status --porcelain lines), and
      how long since the seat's own index was written. For reading how the
      roster stands — `seat` is what sits down, and nothing here is a
      letter to choose from. A claimed seat says whose it is past the
      columns, session and pid, and that is all a reader is told: whether
      the conversation behind it goes on is the user's to know, and a
      letter the user wants back changes hands with `seat takeover`. A
      letter with a branch but no tree says what that branch still carries
      — work nothing can reach until a takeover grows the tree back. Ends
      with how to read the columns.

";
