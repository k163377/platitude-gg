//! The usage page's pictures: the shots board and the films.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str = "  shots <add|crop|diff|prune|open|list|path>
      The shot board: pictures land on a page that magnifies them to exact
      whole ratios without smoothing. A chat pane fits a picture to its
      width and cannot zoom it, so a 1440x900 window is unreadable there
      whatever size it is shot at. `verify-ui` puts its own pair on the
      board as it goes, so `add` is for a crop or a comparison sheet made
      by hand. All six seats write to one board beside the primary
      checkout's .git (.shots/, gitignored), and every run carries the
      seat that took it — a picture from another tree proves nothing about
      this one, which is also why `prune` reaches this seat's runs and no
      others unless told to. `open` shows the board, `list` its runs and
      `path` where the page is; `shots` alone prints every subcommand's
      options.

      `crop <png> --at <x>:<y>:<width>:<height> [--scale <n>]` cuts a
      region out and magnifies it a whole number of times (three by
      default) without smoothing, beside the picture it came from — for
      reading the file itself, where there is nothing to zoom with; a
      fractional ratio would interpolate the pixel a crop is read for.

      `diff <before.png> <after.png>` draws where two pictures of one
      place differ — the after in grey, lighter pixels green, darker ones
      red, each place framed in yellow — for the move of a pixel or two
      that two pictures side by side do not show.

      A before/after goes on with `add --before <png> --after <png>`,
      which puts the two on one view side by side under one magnifier, as
      one run: read one after the other, the difference is whatever the
      reader remembered.

      The board keeps itself to what is being looked at now
      (shots/sweep.rs): a picture retaken replaces the one before it, and
      a seat's runs go when its work does (its branch lands, or the seat
      is handed to fresh work). A session ending removes none of the runs.
      `prune` reaches the rest — an approach abandoned under a name nobody
      retakes (--label), or a seat whose session is long gone.

  film <in|out|shoot|bake|changes>
      Behaviour as an animation, for the board (film skill). A scene file
      says what the frames show and what the hand does; `shoot` plays it
      into the headless window with real input events — hover is Qt's own
      — and grabs the views, the hand and a caption band every tick into
      target/film/<scene>/<side>/. Each side of a comparison is shot from
      its own tree (main, or the commit before a fix, against the branch)
      from one repository the scene builds once and copies, since a preset
      can differ between two trees. The verb that plays it is not the
      harness's own: `in` puts it into this tree and `out` takes it out by
      the same substitutions backwards, and a commit is refused while it
      stands. `shoot` takes --side (now unless told), --no-build and
      --fresh-base. `bake` (--sides to choose) puts the sides on one clock
      — the setup cut, each side counted from its first frame after it,
      each as long as the longest — as one APNG each, made of the frames
      themselves, never compressed again, and names the last frame under
      each caption to read. `changes` (--side) says when a side's picture
      changed and where.

";
