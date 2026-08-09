use std::collections::HashSet;

pub fn unique_group_name(base_name: &str, existing_names: &[&str]) -> String {
    let base = match base_name.trim() {
        "" => "Untitled Group",
        value => value,
    };
    let existing = existing_names
        .iter()
        .map(|name| normalized_name(name))
        .collect::<HashSet<_>>();
    if !existing.contains(&normalized_name(base)) {
        return base.to_owned();
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base} {suffix}");
        if !existing.contains(&normalized_name(&candidate)) {
            return candidate;
        }
        suffix += 1;
    }
}

fn normalized_name(name: &str) -> String {
    name.trim().to_lowercase()
}
