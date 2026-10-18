.pragma library

// What the screen shows, read off the item that paints it rather than off the property meant to drive it: a label
// left on a colour of its own passes a test of the property.

/// The label inside `item` that paints `words`, or null.
function labelSaying(item, words) {
    for (const child of item.children) {
        if (child.text === words && child.color !== undefined)
            return child
        const found = labelSaying(child, words)
        if (found !== null)
            return found
    }
    return null
}

/// The colour `words` are painted in inside `item`; undefined where nothing paints them.
function inkOf(item, words) {
    const label = labelSaying(item, words)
    return label === null ? undefined : label.color
}
