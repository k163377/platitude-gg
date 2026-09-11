//! Fixed interaction inputs. Tabs separate fields so paths may contain spaces.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Case {
    pub(super) name: String,
    pub(super) oid: String,
    pub(super) file: String,
    pub(super) completion: String,
}

pub(super) fn parse(text: &str) -> Result<Vec<Case>, String> {
    let mut cases = Vec::new();
    for (line, row) in text.lines().enumerate() {
        if row.trim().is_empty() || row.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() != 4
            || !fields[0]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || fields[0].is_empty()
            || !matches!(fields[1].len(), 40 | 64)
            || !fields[1].bytes().all(|b| b.is_ascii_hexdigit())
            || fields[2].is_empty()
            || fields.iter().any(|s| s.chars().any(char::is_control))
            || !["raw", "coloured"].contains(&fields[3])
        {
            return Err(format!(
                "cases line {}: expected name<TAB>full-oid<TAB>path<TAB>raw|coloured",
                line + 1
            ));
        }
        if cases.iter().any(|c: &Case| c.name == fields[0]) {
            return Err(format!("duplicate case name: {}", fields[0]));
        }
        cases.push(Case {
            name: fields[0].into(),
            oid: fields[1].to_ascii_lowercase(),
            file: fields[2].into(),
            completion: fields[3].into(),
        });
    }
    if cases.is_empty() {
        return Err("cases file contains no cases".into());
    }
    Ok(cases)
}

pub(super) fn encode(cases: &[Case]) -> String {
    cases
        .iter()
        .map(|c| format!("{}\t{}\t{}\t{}", c.name, c.oid, c.file, c.completion))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_fixed_and_names_are_unambiguous() {
        let text = format!(
            "# cases\nsource\t{}\tsrc/a b.kt\tcoloured\n",
            "a".repeat(40)
        );
        let cases = parse(&text).unwrap();
        assert_eq!(parse(&encode(&cases)).unwrap(), cases);
        assert!(
            parse(&format!("{text}{text}"))
                .unwrap_err()
                .contains("duplicate")
        );
        assert!(parse("moving\tHEAD\tx.kt\tcoloured").is_err());
        assert!(parse(&text.replace("coloured", "unknown")).is_err());
        assert!(parse("# empty").is_err());
    }
}
