use reqwest::Url;

use super::bounded_text;

pub(super) fn package_mutability(target: &str) -> Option<bool> {
    if !bounded_text(target, 4096)
        || target.chars().any(char::is_whitespace)
        || target.starts_with(['.', '/', '~', '-'])
        || target.starts_with("opencode.")
    {
        return None;
    }
    let (repository, revision) = target
        .split_once('#')
        .map_or((target, None), |(repository, revision)| {
            (repository, Some(revision))
        });
    if revision.is_some_and(|revision| {
        revision.is_empty()
            || !revision
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-/+=".contains(&byte))
    }) {
        return None;
    }
    let shorthand = ["github:", "gitlab:", "bitbucket:", "gist:"]
        .into_iter()
        .find_map(|prefix| repository.strip_prefix(prefix));
    let git = if let Some(path) = shorthand {
        repository_path(path)
    } else if repository.contains("://") {
        let url = Url::parse(repository.strip_prefix("git+").unwrap_or(repository)).ok()?;
        if !matches!(url.scheme(), "http" | "https" | "ssh" | "git")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || !repository_path(url.path().trim_start_matches('/'))
        {
            return None;
        }
        true
    } else {
        !repository.starts_with('@') && repository_path(repository)
    };
    if git {
        return Some(!revision.is_some_and(|revision| {
            exact_version(revision)
                || (matches!(revision.len(), 40 | 64)
                    && revision.bytes().all(|byte| byte.is_ascii_hexdigit()))
        }));
    }
    if shorthand.is_some() || revision.is_some() {
        return None;
    }
    let npm = target.strip_prefix("npm:").unwrap_or(target);
    let (name, version) = npm
        .rsplit_once('@')
        .filter(|(name, _)| !name.is_empty())
        .map_or((npm, None), |(name, version)| (name, Some(version)));
    let valid_name = match name.strip_prefix('@') {
        Some(scoped) => scoped
            .split_once('/')
            .is_some_and(|(scope, name)| npm_name(scope) && npm_name(name)),
        None => npm_name(name),
    };
    if !valid_name
        || version.is_some_and(|version| {
            version.is_empty()
                || !version
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-+^~*<>=|".contains(&byte))
        })
    {
        return None;
    }
    Some(version.is_none_or(|version| !exact_version(version)))
}

fn npm_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(['.', '_'])
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn repository_path(value: &str) -> bool {
    value.contains('/')
        && value.split('/').all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
}

fn exact_version(value: &str) -> bool {
    let value = value.strip_prefix('=').unwrap_or(value);
    let value = value.strip_prefix('v').unwrap_or(value);
    let base = value.split(['-', '+']).next().unwrap_or_default();
    let mut parts = base.split('.');
    (0..3).all(|_| {
        parts
            .next()
            .is_some_and(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    }) && parts.next().is_none()
}
