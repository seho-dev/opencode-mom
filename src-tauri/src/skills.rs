use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agents_md::MarkdownAgentDocument;
use crate::document::{read_text, JsoncDoc};
use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::resource_file::{self, Directory, Snapshot};

const MAX_BODY: u64 = 2 * 1024 * 1024;
const SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const CACHE_TTL: Duration = Duration::from_secs(30);
const CACHE_BYTES: usize = 8 * 1024 * 1024;
const CACHE_ENTRIES: usize = 64;
// Reserve fixed entry/container storage as well as URL and text bytes.
const CACHE_PAYLOAD_BYTES: usize = CACHE_BYTES
    - CACHE_ENTRIES * std::mem::size_of::<CachedText>()
    - std::mem::size_of::<OnceLock<Mutex<TextCache>>>();

struct CachedText {
    url: Box<str>,
    text: Box<str>,
    stored: Instant,
}

struct TextCache {
    entries: VecDeque<CachedText>,
    bytes: usize,
}

impl Default for TextCache {
    fn default() -> Self {
        Self {
            entries: VecDeque::with_capacity(CACHE_ENTRIES),
            bytes: 0,
        }
    }
}

impl TextCache {
    fn expire(&mut self, now: Instant) {
        self.entries
            .retain(|entry| now.saturating_duration_since(entry.stored) < CACHE_TTL);
        self.bytes = self
            .entries
            .iter()
            .map(|entry| entry.url.len() + entry.text.len())
            .sum();
    }

    fn get(&mut self, url: &str, now: Instant) -> Option<String> {
        self.expire(now);
        self.entries
            .iter()
            .find(|entry| entry.url.as_ref() == url)
            .map(|entry| entry.text.to_string())
    }

    fn insert(&mut self, url: &str, text: &str, now: Instant) {
        let size = url.len().saturating_add(text.len());
        if size > CACHE_PAYLOAD_BYTES || text.len() as u64 > MAX_BODY {
            return;
        }
        self.expire(now);
        // Cache hits must not extend TTL; a busy list still refreshes after 30 seconds.
        if self.entries.iter().any(|entry| entry.url.as_ref() == url) {
            return;
        }
        // ponytail: FIFO eviction, at most 64 entries; no persistent cache or single-flight network lock.
        while self.entries.len() >= CACHE_ENTRIES || self.bytes + size > CACHE_PAYLOAD_BYTES {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.url.len() + entry.text.len();
            }
        }
        self.bytes += size;
        self.entries.push_back(CachedText {
            url: url.into(),
            text: text.into(),
            stored: now,
        });
    }
}

fn list_cache() -> &'static Mutex<TextCache> {
    static CACHE: OnceLock<Mutex<TextCache>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

fn cache_text(cache: Option<&Mutex<TextCache>>, url: &Url, text: &str) {
    if let Some(mut cache) = cache.and_then(|cache| cache.lock().ok()) {
        cache.insert(url.as_str(), text, Instant::now());
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkillEntry {
    pub id: String,
    pub kind: SkillKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub autoinvoke: bool,
    pub path: String,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillKind {
    Local,
    Remote,
}

#[derive(Debug, Deserialize)]
pub struct SkillDraft {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdate {
    pub id: String,
    pub content: String,
    pub expected_content: String,
    pub expected_path: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SkillList {
    pub data: Vec<SkillEntry>,
    pub diagnostics: Vec<String>,
}

pub fn list(paths: &ConfigPaths) -> Result<SkillList, AppError> {
    discover(paths, None, Some(list_cache())).map(|(list, _)| list)
}

fn discover(
    paths: &ConfigPaths,
    only: Option<&str>,
    cache: Option<&Mutex<TextCache>>,
) -> Result<(SkillList, BTreeMap<String, Snapshot>), AppError> {
    discover_until(paths, only, cache, Instant::now() + SCAN_TIMEOUT)
}

fn discover_until(
    paths: &ConfigPaths,
    only: Option<&str>,
    cache: Option<&Mutex<TextCache>>,
    deadline: Instant,
) -> Result<(SkillList, BTreeMap<String, Snapshot>), AppError> {
    let mut result = SkillList::default();
    let mut entries = BTreeMap::new();
    let mut snapshots = BTreeMap::new();
    for root in paths.global_skill_dirs() {
        scan_root(
            root,
            only,
            &mut entries,
            &mut snapshots,
            &mut result.diagnostics,
        );
    }
    for source in configured_sources(paths, &mut result.diagnostics) {
        if source
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
            || source
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        {
            match scan_catalog(&source, only, cache, deadline) {
                Ok(skills) => {
                    for (id, skill) in skills {
                        if let Err(error) = &skill {
                            result.diagnostics.push(error.to_string());
                        }
                        if skill.is_ok() || only.is_some() {
                            snapshots.remove(&id);
                            entries.insert(id, skill);
                        }
                    }
                }
                Err(error) => {
                    let error = AppError::new(error.code, format!("{source}: {error}"));
                    result.diagnostics.push(error.to_string());
                    if let Some(id) = only {
                        // An unavailable index may contain a higher-priority definition.
                        snapshots.remove(id);
                        entries.insert(id.to_owned(), Err(error));
                    }
                }
            }
        } else {
            scan_root(
                &paths.resolve_skill_path(&source),
                only,
                &mut entries,
                &mut snapshots,
                &mut result.diagnostics,
            );
        }
    }
    result.data = entries.into_values().collect::<Result<Vec<_>, _>>()?;
    Ok((result, snapshots))
}

pub fn get(paths: &ConfigPaths, id: &str) -> Result<SkillEntry, AppError> {
    // IDs are resolved only from permitted discovery; no IPC argument becomes a path.
    discover(paths, Some(id), None)?
        .0
        .data
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| AppError::not_found(format!("global skill '{id}' was not found")))
}

fn validate_new_id(id: &str) -> Result<(), AppError> {
    let base = id
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
            })
        })
    {
        return Err(AppError::validation("New skill ID must be 1–128 ASCII letters, digits, hyphens or underscores, not a reserved Windows name"));
    }
    Ok(())
}

fn validate_content(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() || content.len() as u64 > MAX_BODY || content.contains('\0') {
        return Err(AppError::validation(
            "Skill content must be nonempty Markdown, without NUL, at most 2 MiB",
        ));
    }
    Ok(())
}

pub fn create(paths: &ConfigPaths, draft: SkillDraft) -> Result<SkillEntry, AppError> {
    create_inner(paths, draft, |_| Ok(()))
}

fn ensure_create_available(paths: &ConfigPaths, id: &str) -> Result<(), AppError> {
    let existing = discover(paths, None, None)?.0;
    if !existing.diagnostics.is_empty() {
        return Err(AppError::configuration("Skill discovery is incomplete; resolve unavailable or invalid sources before creating a skill"));
    }
    if existing.data.iter().any(|entry| entry.id == id) {
        return Err(AppError::validation("A skill with this ID already exists"));
    }
    Ok(())
}

fn create_inner(
    paths: &ConfigPaths,
    draft: SkillDraft,
    before_publish: impl FnOnce(&Directory) -> Result<(), AppError>,
) -> Result<SkillEntry, AppError> {
    validate_new_id(&draft.id)?;
    validate_content(&draft.content)?;
    let root = paths.native_global_skills_dir();
    let path = root.join(&draft.id).join("SKILL.md");
    let entry = parse_entry(
        &draft.id,
        &path.display().to_string(),
        &root.display().to_string(),
        SkillKind::Local,
        draft.content,
    )
    .map_err(|_| AppError::validation("Invalid skill Markdown frontmatter"))?;
    ensure_create_available(paths, &draft.id)?;
    resource_file::absent(&root.join(&draft.id))?;
    let root = Directory::ensure(&root)?;
    let directory = root.create_child(&draft.id)?;
    if let Err(error) = directory.create_file_checked("SKILL.md", &entry.content, || {
        before_publish(&directory)?;
        ensure_create_available(paths, &draft.id)
    }) {
        if directory.validate().is_ok() {
            let _ = fs::remove_dir(&directory.resolved);
        }
        return Err(error);
    }
    Ok(entry)
}

pub fn update(paths: &ConfigPaths, draft: SkillUpdate) -> Result<SkillEntry, AppError> {
    update_inner(paths, draft, || {})
}

fn update_inner(
    paths: &ConfigPaths,
    draft: SkillUpdate,
    after_discovery: impl FnOnce(),
) -> Result<SkillEntry, AppError> {
    validate_content(&draft.content)?;
    let (discovered, mut snapshots) = discover(paths, Some(&draft.id), None)?;
    let current = discovered
        .data
        .into_iter()
        .find(|entry| entry.id == draft.id)
        .ok_or_else(|| AppError::not_found("Skill no longer exists"))?;
    if current.kind != SkillKind::Local {
        return Err(AppError::validation("Remote skills are read-only"));
    }
    if !discovered.diagnostics.is_empty() {
        return Err(AppError::configuration(
            "Skill discovery is incomplete; resolve invalid or unavailable sources before saving",
        ));
    }
    if current.path != draft.expected_path || current.content != draft.expected_content {
        return Err(AppError::configuration(
            "Skill changed; reload before saving",
        ));
    }
    let entry = parse_entry(
        &current.id,
        &current.path,
        &current.source,
        SkillKind::Local,
        draft.content,
    )
    .map_err(|_| AppError::validation("Invalid skill Markdown frontmatter"))?;
    after_discovery();
    let snapshot = snapshots
        .remove(&draft.id)
        .ok_or_else(|| AppError::configuration("Local skill snapshot is unavailable"))?;
    resource_file::replace(&[(snapshot, entry.content.clone())], |_| {
        let (discovered, _) = discover(paths, Some(&draft.id), None)?;
        if !discovered.diagnostics.is_empty() {
            return Err(AppError::configuration(
                "Skill discovery became incomplete; reload before saving",
            ));
        }
        let winner = discovered
            .data
            .into_iter()
            .find(|entry| entry.id == draft.id)
            .ok_or_else(|| AppError::configuration("Skill disappeared; reload before saving"))?;
        if winner.kind != SkillKind::Local
            || winner.path != current.path
            || winner.content != current.content
        {
            return Err(AppError::configuration(
                "Skill source changed; reload before saving",
            ));
        }
        Ok(())
    })?;
    Ok(entry)
}

fn configured_sources(paths: &ConfigPaths, diagnostics: &mut Vec<String>) -> Vec<String> {
    let mut sources = Vec::new();
    for path in paths.global_config_files() {
        let parsed = (|| {
            let Some(content) = read_text(path)? else {
                return Ok(Vec::new());
            };
            let document = JsoncDoc::parse(&content)?;
            let Some(skills) = document.raw().get("skills") else {
                return Ok(Vec::new());
            };
            let skills = skills
                .as_array()
                .ok_or_else(|| AppError::validation("skills must be an array"))?;
            skills
                .iter()
                .map(|source| {
                    source
                        .as_str()
                        .filter(|source| !source.is_empty())
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            AppError::validation("skills entries must be nonempty strings")
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })();
        match parsed {
            Ok(entries) => sources.extend(entries),
            Err(error) => diagnostics.push(format!("{}: {error}", path.display())),
        }
    }
    sources
}

fn scan_root(
    root: &Path,
    only: Option<&str>,
    entries: &mut BTreeMap<String, Result<SkillEntry, AppError>>,
    snapshots: &mut BTreeMap<String, Snapshot>,
    diagnostics: &mut Vec<String>,
) {
    let mut files = Vec::new();
    collect_files(root, root, &mut BTreeSet::new(), &mut files, diagnostics);
    files.sort();
    for path in files {
        let id = if path.parent() == Some(root) {
            path.file_stem()
        } else {
            path.parent().and_then(Path::file_name)
        };
        let Some(id) = id.and_then(|id| id.to_str()).filter(|id| !id.is_empty()) else {
            diagnostics.push(format!("{}: invalid skill ID", path.display()));
            continue;
        };
        if only.is_some_and(|requested| requested != id) {
            continue;
        }
        let mut snapshot = None;
        let parsed = Snapshot::read(&path)
            .and_then(|value| {
                value.ok_or_else(|| AppError::configuration("Skill disappeared during discovery"))
            })
            .and_then(|value| {
                let content = value.content.clone();
                snapshot = Some(value);
                parse_entry(
                    id,
                    &path.display().to_string(),
                    &root.display().to_string(),
                    SkillKind::Local,
                    content,
                )
            });
        let parsed = parsed
            .map_err(|error| AppError::new(error.code, format!("{}: {error}", path.display())));
        if let Err(error) = &parsed {
            diagnostics.push(error.to_string());
        }
        if parsed.is_ok() || only.is_some() {
            if only.is_some() {
                if let Some(snapshot) = snapshot {
                    snapshots.insert(id.to_owned(), snapshot);
                } else {
                    snapshots.remove(id);
                }
            }
            entries.insert(id.to_owned(), parsed);
        }
    }
}

fn collect_files(
    root: &Path,
    directory: &Path,
    ancestors: &mut BTreeSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<String>,
) {
    let canonical = match fs::canonicalize(directory) {
        Ok(path) => path,
        Err(error) if directory == root && error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            diagnostics.push(AppError::io("resolve", directory, error).to_string());
            return;
        }
    };
    if ancestors.contains(&canonical) {
        diagnostics.push(format!(
            "{}: skipped symlink recursion loop",
            directory.display()
        ));
        return;
    }
    // ponytail: cap pathological nesting; use an iterative walker if deeper layouts matter.
    if ancestors.len() >= 128 {
        diagnostics.push(format!(
            "{}: skill directory nesting exceeds 128",
            directory.display()
        ));
        return;
    }
    let children = match fs::read_dir(directory) {
        Ok(children) => children,
        Err(error) => {
            diagnostics.push(AppError::io("read", directory, error).to_string());
            return;
        }
    };
    ancestors.insert(canonical.clone());
    let mut children = children
        .filter_map(|child| match child {
            Ok(child) => Some(child.path()),
            Err(error) => {
                diagnostics.push(AppError::io("read", directory, error).to_string());
                None
            }
        })
        .collect::<Vec<_>>();
    children.sort();
    for path in children {
        let is_entry = path.file_name().is_some_and(|name| name == "SKILL.md")
            || (directory == root && path.extension().is_some_and(|extension| extension == "md"));
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {
                collect_files(root, &path, ancestors, files, diagnostics)
            }
            Ok(metadata) if metadata.is_file() && is_entry => files.push(path),
            Ok(_) => {}
            Err(_) if is_entry => files.push(path),
            Err(error) => diagnostics.push(AppError::io("read", &path, error).to_string()),
        }
    }
    ancestors.remove(&canonical);
}

fn parse_entry(
    id: &str,
    path: &str,
    source: &str,
    kind: SkillKind,
    content: String,
) -> Result<SkillEntry, AppError> {
    let document = MarkdownAgentDocument::parse(&content)?;
    if content.trim_start_matches('\u{feff}').starts_with("---\n")
        || content
            .trim_start_matches('\u{feff}')
            .starts_with("---\r\n")
    {
        let closed = content
            .lines()
            .skip(1)
            .any(|line| matches!(line, "---" | "..."));
        if !closed {
            return Err(AppError::validation("unclosed skill frontmatter"));
        }
    }
    let name = document
        .frontmatter
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .unwrap_or(id)
        .to_owned();
    let description = document
        .frontmatter
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let autoinvoke = match document
        .frontmatter
        .get("metadata")
        .and_then(|metadata| metadata.get("opencode/autoinvoke"))
    {
        Some(Value::Bool(false)) => false,
        Some(Value::String(value)) if value.eq_ignore_ascii_case("false") => false,
        None => document.frontmatter.get("disable-model-invocation") != Some(&Value::Bool(true)),
        _ => true,
    };
    Ok(SkillEntry {
        id: id.to_owned(),
        kind,
        name,
        description,
        autoinvoke,
        path: path.to_owned(),
        content,
        source: source.to_owned(),
    })
}

#[derive(Deserialize)]
struct Catalog {
    skills: Vec<CatalogSkill>,
}

#[derive(Deserialize)]
struct CatalogSkill {
    name: String,
    files: Vec<String>,
}

fn catalog_client() -> Result<&'static Client, AppError> {
    // Keep the last handle alive: reqwest shutdown can wait for system DNS past the request deadline.
    static CLIENT: OnceLock<Result<Client, AppError>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .timeout(Duration::from_secs(10))
                .connect_timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|error| AppError::configuration(format!("create catalog client: {error}")))
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn scan_catalog(
    source: &str,
    only: Option<&str>,
    cache: Option<&Mutex<TextCache>>,
    deadline: Instant,
) -> Result<Vec<(String, Result<SkillEntry, AppError>)>, AppError> {
    if Instant::now() >= deadline {
        return Err(AppError::configuration("Skill scan deadline exceeded"));
    }
    let mut base = Url::parse(source).map_err(|error| AppError::validation(error.to_string()))?;
    if !matches!(base.scheme(), "http" | "https")
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(AppError::validation(
            "catalog must be an HTTP(S) base URL without credentials, query, or fragment",
        ));
    }
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    let client = catalog_client()?;
    let index = catalog_url(&base, "index.json")?;
    let index_text = fetch_text(client, &index, cache, deadline)?;
    let catalog: Catalog = serde_json::from_str(&index_text)
        .map_err(|error| AppError::validation(format!("invalid catalog index: {error}")))?;
    let mut targets = Vec::new();
    for skill in catalog.skills {
        let flat = format!("{}.md", skill.name);
        let file = skill
            .files
            .iter()
            .find(|file| file.as_str() == "SKILL.md")
            .or_else(|| skill.files.iter().find(|file| *file == &flat));
        let id = file.and_then(|file| Path::new(file).file_stem()?.to_str());
        if only.is_some_and(|requested| Some(requested) != id) {
            continue;
        }
        validate_relative(&skill.name, true)?;
        for file in &skill.files {
            validate_relative(file, false)?;
        }
        if let (Some(file), Some(id)) = (file, id) {
            targets.push((
                id.to_owned(),
                catalog_url(&base, &format!("{}/{file}", skill.name))?,
            ));
        }
    }
    if Instant::now() >= deadline {
        return Err(AppError::configuration("Skill scan deadline exceeded"));
    }
    let mut entries = Vec::new();
    for chunk in targets.chunks(4) {
        if Instant::now() >= deadline {
            for (id, url) in chunk {
                entries.push((
                    id.clone(),
                    Err(AppError::configuration(format!(
                        "fetch {url}: skill scan deadline exceeded"
                    ))),
                ));
            }
            continue;
        }
        std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|(id, url)| {
                    scope.spawn(move || {
                        fetch_text(client, url, cache, deadline)
                            .and_then(|content| {
                                parse_entry(id, url.as_str(), source, SkillKind::Remote, content)
                            })
                            .map_err(|error| AppError::new(error.code, format!("{url}: {error}")))
                    })
                })
                .collect::<Vec<_>>();
            // Join in index order, not completion order, to preserve duplicate precedence.
            for ((id, _), handle) in chunk.iter().zip(handles) {
                let entry = handle.join().unwrap_or_else(|_| {
                    Err(AppError::configuration("Catalog fetch worker failed"))
                });
                entries.push((id.clone(), entry));
            }
        });
    }
    if entries.iter().all(|(_, entry)| entry.is_ok()) {
        cache_text(cache, &index, &index_text);
        for ((_, url), (_, entry)) in targets.iter().zip(&entries) {
            if let Ok(entry) = entry {
                cache_text(cache, url, &entry.content);
            }
        }
    }
    Ok(entries)
}

fn validate_relative(path: &str, single: bool) -> Result<(), AppError> {
    let decoded = percent_encoding::percent_decode_str(path)
        .decode_utf8()
        .map_err(|_| AppError::validation("invalid encoded catalog path"))?;
    if decoded.is_empty()
        || decoded.contains(['\\', ':', '?', '#', '%'])
        || decoded.chars().any(char::is_control)
        || (single && decoded.contains('/'))
        || decoded
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(AppError::validation(format!(
            "unsafe catalog path '{path}'"
        )));
    }
    Ok(())
}

fn catalog_url(base: &Url, relative: &str) -> Result<Url, AppError> {
    validate_relative(relative, false)?;
    let url = base
        .join(relative)
        .map_err(|error| AppError::validation(error.to_string()))?;
    if url.origin() != base.origin() || !url.path().starts_with(base.path()) {
        return Err(AppError::validation("catalog entry escapes its source"));
    }
    Ok(url)
}

fn fetch_text(
    client: &Client,
    url: &Url,
    cache: Option<&Mutex<TextCache>>,
    deadline: Instant,
) -> Result<String, AppError> {
    if Instant::now() >= deadline {
        return Err(AppError::configuration(format!(
            "fetch {url}: skill scan deadline exceeded"
        )));
    }
    if let Some(text) = cache
        .and_then(|cache| cache.lock().ok())
        .and_then(|mut cache| cache.get(url.as_str(), Instant::now()))
    {
        return Ok(text);
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(AppError::configuration(format!(
            "fetch {url}: skill scan deadline exceeded"
        )));
    }
    let response = client
        .get(url.clone())
        .timeout(remaining.min(Duration::from_secs(10)))
        .send()
        .map_err(|error| AppError::configuration(format!("fetch {url}: {error}")))?;
    if !response.status().is_success() {
        return Err(AppError::configuration(format!(
            "fetch {url}: HTTP {} (redirects are not followed)",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY)
    {
        return Err(AppError::validation(format!("{url}: body exceeds 2 MiB")));
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_BODY + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::configuration(format!("read {url}: {error}")))?;
    if bytes.len() as u64 > MAX_BODY {
        return Err(AppError::validation(format!("{url}: body exceeds 2 MiB")));
    }
    if Instant::now() >= deadline {
        return Err(AppError::configuration(format!(
            "fetch {url}: skill scan deadline exceeded"
        )));
    }
    String::from_utf8(bytes).map_err(|_| AppError::validation(format!("{url}: body is not UTF-8")))
}

#[cfg(all(test, unix))]
#[path = "skills/tests.rs"]
mod management_safety_tests;

#[cfg(test)]
#[path = "skills/remote_tests.rs"]
mod remote_tests;
