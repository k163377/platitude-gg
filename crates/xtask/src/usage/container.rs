//! The usage page's Linux container.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str = "  linux [--rebuild] [--shell] [--stage core|app] [--runner <name>
        [--container <name> --step <mark>]] <command…>
      Run a command against this checkout on Ubuntu, in a container built
      from ci/linux/Dockerfile — by WSL's own wslc on Windows (WSL 3.0 or
      later, no Docker Desktop), by docker elsewhere. On Linux it skips
      the container and runs the command where it stands.
        cargo xtask linux test -p platitude-core --test it
        cargo xtask linux verify-ui commit --preset basic
        cargo xtask linux bare
        cargo xtask linux --container <name> --step <mark> stop
      A cargo command goes to cargo; an xtask verb goes to cargo xtask.
      `--runner <name>` starts an xtask verb from the copy a gate
      prepared; with `--container <name> --step <mark>` beside it the verb
      goes into the gate's own container by the engine's exec, every
      process of it carrying the mark. `stop` ends what still carries a
      mark in that container — what the gate does at a ceiling, by hand —
      and says what went and what would not.
      `runner <name> --gate <pid>` is the gate's own step and not one to
      type: it builds in the checkout's volume, so it runs only when the
      pid it names is the live gate of this tree, and one held up past its
      gate cannot reach a later run's copy.
      `bare` is neither a cargo command nor a verb, and stays in the
      container on Linux too: it builds the release workspace-wide and
      starts it on an Ubuntu carrying only what a package would declare,
      the only check that the thing runs somewhere it was not built;
      `bare --discover` works that package list out afresh, whatever the
      one that stands says.
      An engine that gives no answer is not an answer: what is asked of it
      is asked under a ceiling, and a line that cannot learn whether its
      image is built stops there and builds nothing. The first line to
      meet a silence keeps what stood on the host — the engine's
      processes, its session process's threads and memory, the disks' last
      events — under pgg-engine in the primary checkout's target/tmp, and
      says where; `witness` takes the same by hand, for a wslc seen
      hanging anywhere else. Take it BEFORE anything ends the session
      (`wslc system session terminate`, ending wslcsession.exe, a reboot):
      the README it leaves says what an administrator can add.
        cargo xtask linux witness
      Which image it runs in follows what the command needs: core is
      Ubuntu and the toolchain, app adds Qt, a software GL stack and the
      fonts デザイン規約 names for Ubuntu. The build directory is a volume
      of the engine's, so this target/ is untouched, and a verify-ui run
      is handed a host directory to leave its screenshot in. Its pictures
      stay off the board unless the line says --board: the board shows
      the Windows runs, and a Linux one when it was asked for. On Linux,
      where the run is the host's own, it files them as any run there
      does, and --board changes nothing.
        cargo xtask linux verify-ui commit --preset basic --board
      options:
        --rebuild        build the image again even if one already matches
        --shell          open a shell in the container instead
        --stage <name>   core or app, when the guess is not the one wanted

";
