//! Fetch refspecs, read the way git reads them to name the remote-tracking
//! ref a remote's ref is fetched into (`remote.c`'s `query_refspecs`).

/// The ref `refspecs` — one remote's `remote.<name>.fetch` values, in the
/// order git reads them — fetch `src` into: the first that takes it in, a
/// `*` on each side standing for the same run, passing over any that
/// fetches into no ref (no `:<dst>`). `None` where none takes it, or where
/// a negative one (`^<src>`) leaves it out, whatever the order.
pub(super) fn fetched_into<'a>(
    refspecs: impl Iterator<Item = &'a str> + Clone,
    src: &str,
) -> Option<String> {
    let specs = refspecs.map(|spec| spec.strip_prefix('+').unwrap_or(spec));
    if specs
        .clone()
        .filter_map(|spec| spec.strip_prefix('^'))
        .any(|negative| matched(negative, src).is_some())
    {
        return None;
    }
    specs
        .filter(|spec| !spec.starts_with('^'))
        .find_map(|spec| {
            let (from, to) = spec.split_once(':')?;
            if to.is_empty() {
                return None;
            }
            match (from.contains('*'), to.split_once('*')) {
                (true, Some((head, tail))) => {
                    matched(from, src).map(|run| format!("{head}{run}{tail}"))
                }
                (false, None) => (from == src).then(|| to.to_string()),
                // A `*` on one side only, which git refuses to read.
                _ => None,
            }
        })
}

/// The run `pattern`'s one `*` stands for in `name` — the empty run where
/// `pattern` has none and is `name` itself.
fn matched<'a>(pattern: &str, name: &'a str) -> Option<&'a str> {
    match pattern.split_once('*') {
        Some((head, tail)) => name.strip_prefix(head)?.strip_suffix(tail),
        None => (pattern == name).then_some(""),
    }
}

#[cfg(test)]
mod tests {
    use super::fetched_into;

    fn into(refspecs: &[&str], src: &str) -> Option<String> {
        fetched_into(refspecs.iter().copied(), src)
    }

    const DEFAULT: &str = "+refs/heads/*:refs/remotes/origin/*";

    #[test]
    fn a_branch_goes_where_the_default_refspec_puts_it() {
        assert_eq!(
            into(&[DEFAULT], "refs/heads/feature/x").as_deref(),
            Some("refs/remotes/origin/feature/x")
        );
        assert_eq!(into(&[DEFAULT], "refs/pull/12/head"), None);
        assert_eq!(into(&[], "refs/heads/main"), None, "no refspec, no ref");
    }

    /// `gh`'s pull-request refspec beside the default, and a remote fetched
    /// into another name altogether.
    #[test]
    fn a_refspec_of_its_own_names_where_it_goes() {
        let specs = [DEFAULT, "+refs/pull/*/head:refs/remotes/origin/pr/*"];
        assert_eq!(
            into(&specs, "refs/pull/12/head").as_deref(),
            Some("refs/remotes/origin/pr/12")
        );
        assert_eq!(
            into(&["+refs/heads/*:refs/remotes/mirror/*"], "refs/heads/main").as_deref(),
            Some("refs/remotes/mirror/main")
        );
    }

    #[test]
    fn the_first_refspec_that_takes_it_in_decides() {
        assert_eq!(
            into(
                &["refs/heads/main:refs/remotes/origin/trunk", DEFAULT],
                "refs/heads/main"
            )
            .as_deref(),
            Some("refs/remotes/origin/trunk")
        );
        assert_eq!(
            into(&["refs/heads/main", DEFAULT], "refs/heads/main").as_deref(),
            Some("refs/remotes/origin/main"),
            "one fetching into no ref is passed over"
        );
    }

    #[test]
    fn a_negative_refspec_leaves_it_out_wherever_it_stands() {
        assert_eq!(
            into(&[DEFAULT, "^refs/heads/wip/*"], "refs/heads/wip/x"),
            None
        );
        assert_eq!(into(&["^refs/heads/old", DEFAULT], "refs/heads/old"), None);
        assert_eq!(
            into(&[DEFAULT, "^refs/heads/wip/*"], "refs/heads/main").as_deref(),
            Some("refs/remotes/origin/main")
        );
    }
}
