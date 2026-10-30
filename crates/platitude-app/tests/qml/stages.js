.pragma library

// Reads the lines the harness's machinery said (`Harness.said`, the stand-in's) the way `xtask::verify::contract`
// reads a run's: by the word before `run=`, and each `key=value` word.

function field(line, key) {
    for (const word of line.split(" ")) {
        if (word.startsWith(key + "="))
            return word.substring(key.length + 1)
    }
    return undefined
}

function kind(line) {
    const at = line.indexOf(" run=")
    return at < 0 ? "" : line.substring(0, at).split(" ").pop()
}

/// Completion reports alone: notes and stage history cannot satisfy a report's assertion.
function reports(said) {
    return said.filter(line => kind(line) === "report").map(line => line.split(" text=").slice(1).join(" text="))
}

/// The stages generation `gen` of `owner/op` moved to, in order.
function chain(said, owner, op, gen) {
    return said.filter(line => kind(line) === "stage" && field(line, "owner") === owner && field(line, "op") === op
                               && field(line, "gen") === String(gen))
               .map(line => field(line, "to"))
}

/// The stage lines said from `from` on.
function stageLines(said, from) {
    return said.slice(from).filter(line => kind(line) === "stage")
}

/// The breaches the machinery said.
function breaches(said) {
    return said.filter(line => kind(line) === "stage_breach")
}
