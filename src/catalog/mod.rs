//! Repositories (base): GitHub repositories that hold plugins, themes and controller mappings, listed and installed from
//! PREFERENCES in one click.
//!
//! A [`Repo`] is `owner/repo`. Its files are one zip, `https://github.com/<owner>/<repo>/archive/HEAD.zip` (the default
//! branch; `ureq` follows the redirect to codeload), kept in the cache folder (`<cache>/repos/`) until the user asks for a fresh
//! copy. [`read`] fetches it and lists what it holds: plugins (any folder with a `plugin.json`, at most two folders deep),
//! theme files (`*.json`, at most two folders deep, outside the plugins; the app decides which of them are themes) and
//! mapping files (`*.xml` whose root is `<OpenFadeRMapping`; the app decides which of them parse).
//!
//! Nothing here touches the window and everything may block: callers run it on a worker thread. Installing goes through the
//! installers a manual drop uses ([`crate::plugin::install`]; the app's skin store for themes and mapping parser for mappings),
//! so nothing installs that a drop would refuse. Children: [`list`] (the user's repositories, `<config>/repositories.json`),
//! [`plugins`] (scan and install, with the prebuilt-library download of native plugins), [`themes`] and [`mappings`] (scan).

pub mod list;
pub mod mappings;
pub mod plugins;
pub mod themes;

pub(crate) mod check;

use crate::zip::Zip;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A repository on GitHub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

/// `owner/repo`.
impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

impl Repo {
    /// A repository from what the user typed: `owner/repo`, or its GitHub address with or without `https://`, `.git` and
    /// a closing `/`.
    pub fn parse(text: &str) -> Result<Repo, String> {
        let t = text.trim();
        let t = ["https://", "http://"]
            .iter()
            .find_map(|p| t.strip_prefix(p))
            .unwrap_or(t);
        let t = t.strip_prefix("www.").unwrap_or(t);
        let t = t.strip_prefix("github.com/").unwrap_or(t);
        let t = t.trim_end_matches('/');
        let t = t.strip_suffix(".git").unwrap_or(t);
        let bad = || format!("{text:?} is not a GitHub repository: write it as owner/repo");
        let (owner, name) = t.split_once('/').ok_or_else(bad)?;
        let word = |s: &str, extra: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
        };
        if !word(owner, "-") || !word(name, "-_.") || name == "." || name == ".." {
            return Err(bad());
        }
        Ok(Repo {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    /// The zip of the repository's default branch.
    pub fn zip_url(&self) -> String {
        format!("https://github.com/{self}/archive/HEAD.zip")
    }

    /// [`zip_url`]'s entry name in the repository's SHA256SUMS (the name the browser saves the archive as).
    pub fn zip_name(&self) -> String {
        format!("{}-HEAD.zip", self.name)
    }

    /// Where a plugin of this repository that needs a native library finds it for this machine: the asset of the latest
    /// release called `<plugin>-<os>-<arch>.zip` (`std::env::consts`, e.g. `my-fx-linux-x86_64.zip`), whose root holds the
    /// library files as `plugin.json` names them.
    pub fn asset_url(&self, plugin: &str) -> String {
        let name: String = plugin
            .bytes()
            .map(|b| match b {
                b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect();
        format!(
            "https://github.com/{self}/releases/latest/download/{name}-{}-{}.zip",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }

    /// The checksum list of the repository's latest release ([`check`]): one `SHA256SUMS` the repository maintains,
    /// which its downloads verify against. A repository that has not made one yet keeps working (the list is opt-in);
    /// a list that holds the download refuses a tampered or replaced one.
    pub fn sha_url(&self) -> String {
        format!("https://github.com/{self}/releases/latest/download/SHA256SUMS")
    }

    /// The cached zip: `<cache>/repos/<owner>__<repo>.zip` (lower case: GitHub names are not case-sensitive).
    fn cache_file(&self) -> Result<PathBuf, String> {
        let root = crate::analysis::cache::root()
            .ok_or("no cache directory (set XDG_CACHE_HOME or HOME)")?;
        Ok(root
            .join("repos")
            .join(format!("{}__{}.zip", self.owner, self.name).to_lowercase()))
    }
}

/// Why a download failed: the address does not exist (HTTP 404), or anything else.
pub(crate) enum Get {
    NotFound,
    Failed(String),
}

/// Most a download may be (a whole repository zip), and most its `SHA256SUMS` list may be.
const MAX_DOWNLOAD: u64 = 512 << 20;
const MAX_LIST: u64 = 1 << 20;

/// Downloads `url` to `to` through a `.part` file, so a failure never leaves a truncated file. `checks` is `(the
/// SHA256SUMS list to read, the download's entry name in it)`: an entry that holds the download's hash installs it, an
/// entry that does not (or none) refuses the file, and no reachable list (repositories publish one when they start to)
/// warns on stderr and keeps the download. The caller maps the errors to its own messages.
pub(crate) fn get(url: &str, to: &Path, checks: Option<(&str, &str)>) -> Result<(), Get> {
    let failed = |e: &dyn fmt::Display| Get::Failed(e.to_string());
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into();
    let resp = agent.get(url).call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => Get::NotFound,
        e => failed(&e),
    })?;
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| failed(&e))?;
    }
    let part = to.with_extension("part");
    let mut out = std::fs::File::create(&part).map_err(|e| failed(&e))?;
    let done = std::io::copy(
        &mut resp.into_body().into_reader().take(MAX_DOWNLOAD),
        &mut out,
    )
    .map_err(|e| failed(&e))
    .and_then(|_| std::fs::rename(&part, to).map_err(|e| failed(&e)));
    if done.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    done?;
    match checks {
        None => Ok(()),
        Some((list_url, entry)) => match checked(&agent, list_url, entry, to) {
            Ok(()) => Ok(()),
            Err(Get::NotFound) => {
                // no list yet: the download stands, with the warning so the opt-in is seen
                eprintln!(
                    "catalog: {list_url} has no SHA256SUMS yet (its releases are not checksummed); continuing without it"
                );
                Ok(())
            }
            Err(e) => {
                let _ = std::fs::remove_file(to);
                Err(e)
            }
        },
    }
}

/// Where [`get`]'s second answer on the list itself goes: a reachable list is checked, a missing one is [`Get::NotFound`]
/// (no list yet), anything else is a failure that refuses the download (a list that half-arrives is not trusted).
fn checked(
    agent: &ureq::Agent,
    list_url: &str,
    entry: &str,
    file: &Path,
) -> Result<(), Get> {
    let resp = agent.get(list_url).call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => Get::NotFound,
        e => Get::Failed(format!("reading the SHA256SUMS failed ({e})")),
    })?;
    let mut list = String::new();
    resp.into_body()
        .into_reader()
        .take(MAX_LIST)
        .read_to_string(&mut list)
        .map_err(|e| Get::Failed(format!("reading the SHA256SUMS failed ({e})")))?;
    check::verify(&list, entry, file).map_err(Get::Failed)
}

/// What a repository holds.
pub struct Found {
    /// The cached zip, which [`plugins::install`] reads again.
    pub zip: PathBuf,
    pub plugins: Vec<plugins::Entry>,
    pub themes: Vec<themes::File>,
    pub mappings: Vec<mappings::File>,
}

/// The cached zip of `repo`, if one was downloaded.
pub fn cached(repo: &Repo) -> Option<PathBuf> {
    repo.cache_file().ok().filter(|z| z.is_file())
}

/// The cached zip of `repo`, downloaded first when there is none yet or `refresh` is set (verified, see [`check`]).
pub fn fetch(repo: &Repo, refresh: bool) -> Result<PathBuf, String> {
    let zip = repo.cache_file()?;
    if refresh || !zip.is_file() {
        let (list, entry) = (repo.sha_url(), repo.zip_name());
        get(&repo.zip_url(), &zip, Some((list.as_str(), entry.as_str())))
            .map_err(|e| match e {
            Get::NotFound => format!(
                "{repo}: no such repository on GitHub (check the name; a private repository cannot be read)"
            ),
            Get::Failed(e) => format!("{repo}: download failed ({e})"),
        })?;
    }
    Ok(zip)
}

/// Fetches `repo` ([`fetch`]) and lists its plugins, theme files and mapping files.
pub fn read(repo: &Repo, refresh: bool) -> Result<Found, String> {
    let zip = fetch(repo, refresh)?;
    let mut z = Zip::open(&zip).map_err(|e| format!("{repo}: not a readable zip ({e:?})"))?;
    let plugins = plugins::scan(&mut z);
    let themes = themes::scan(&mut z, &plugins);
    let mappings = mappings::scan(&mut z);
    Ok(Found {
        zip,
        plugins,
        themes,
        mappings,
    })
}

/// How many folders deep `plugin.json` and theme files are looked for (below the repository's top folder).
const DEPTH: usize = 2;

/// An entry name split into the zip's top folder and the path below it (`None` for the top folder itself).
fn below_top(name: &str) -> Option<(&str, &str)> {
    name.split_once('/').filter(|(_, rest)| !rest.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repositories_are_read_from_owner_repo_or_the_github_address() {
        for ok in [
            "ronellcl/MyOpenFadeR-Plugins",
            " ronellcl/MyOpenFadeR-Plugins ",
            "https://github.com/ronellcl/MyOpenFadeR-Plugins",
            "https://github.com/ronellcl/MyOpenFadeR-Plugins/",
            "https://github.com/ronellcl/MyOpenFadeR-Plugins.git",
            "http://www.github.com/ronellcl/MyOpenFadeR-Plugins.git/",
            "github.com/ronellcl/MyOpenFadeR-Plugins",
        ] {
            let r = Repo::parse(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
            assert_eq!(r.to_string(), "ronellcl/MyOpenFadeR-Plugins", "{ok}");
        }
        assert_eq!(Repo::parse("a/b.c_d").unwrap().name, "b.c_d");
        for bad in [
            "",
            "owner",
            "a/b/c",
            "/b",
            "a/",
            "a/..",
            "a b/c",
            "https://gitlab.com/a/b",
            "https://github.com/a",
        ] {
            assert!(Repo::parse(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn urls_follow_the_release_asset_contract() {
        let r = Repo::parse("a/b").unwrap();
        assert_eq!(r.zip_url(), "https://github.com/a/b/archive/HEAD.zip");
        let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
        assert_eq!(
            r.asset_url("my-fx"),
            format!("https://github.com/a/b/releases/latest/download/my-fx-{os}-{arch}.zip")
        );
        assert!(r.asset_url("my plugin").contains("/my%20plugin-"));
    }
}
