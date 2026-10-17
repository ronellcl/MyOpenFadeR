//! Installing and removing plugins. A plugin folder, a `.zip` of one (the folder's files at the archive root, or one top
//! folder holding them) or the `plugin.json` of a folder is copied into the user's plugin folder
//! ([`user_dir`]: `<config dir>/extension/plugin/<name>/`), replacing an older copy of the same plugin. The copy is
//! checked as [`super::discover`] would read it before it replaces anything. Uninstalling deletes the user's copy and
//! lists the name as removed ([`super::config`]), so a copy bundled with the app is not read either until an install.

use super::{read_manifest, Plugin};
use crate::zip::Zip;
use std::path::{Path, PathBuf};

/// Folders and files an install leaves out (build and editor leftovers).
const SKIP: [&str; 4] = [".git", "target", "__pycache__", ".DS_Store"];

/// `<config dir>/extension/plugin`, where plugins are installed.
pub fn user_dir() -> Option<PathBuf> {
    crate::paths::config_dir().map(|d| d.join("extension").join("plugin"))
}

/// Whether `path` looks like a plugin to install (a folder with or around a `plugin.json`, that file, or a `.zip`
/// holding one).
pub fn is_plugin(path: &Path) -> bool {
    if path.is_dir() {
        return source_dir(path).is_ok();
    }
    if path.file_name().is_some_and(|n| n == "plugin.json") {
        return true;
    }
    is_zip(path) && Zip::open(path).is_ok_and(|z| zip_prefix(&z).is_ok())
}

fn is_zip(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

/// Installs the plugin at `from` into [`user_dir`] and switches it on; returns its name. An uninstalled plugin of that
/// name is back.
pub fn install(from: &Path) -> Result<String, String> {
    let name = install_into(from, &user_dir().ok_or("no config folder (set HOME)")?)?;
    super::config::update(|c| {
        c.removed.retain(|n| *n != name);
        c.set_enabled(&name, true);
    })?;
    Ok(name)
}

/// [`install`] into `root`.
pub fn install_into(from: &Path, root: &Path) -> Result<String, String> {
    crate::paths::at(std::fs::create_dir_all(root), root)?;
    // Leftovers of previous sessions: a `.old-<pid>` whose plugin is not run
    // from there by this app (a fresh install starts all over anyway).
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if e.file_name().to_str().is_some_and(|n| n.starts_with(".old-")) {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    let staging = root.join(format!(".installing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    let result = stage(from, &staging).and_then(|()| {
        let m = read_manifest(&staging)?;
        let folder = folder_name(&m.name)
            .ok_or_else(|| format!("{:?}: not a plugin-folder name", m.name))?;
        let dest = root.join(&folder);
        let old = dest.with_file_name(format!(".old-{}", std::process::id()));
        // CORE-07: the destination is set aside, not destroyed. Anything failing
        // between here and the finisher leaves the old copy usable: it goes back
        // in place (a Windows DLL of a loaded plugin is fine here: the rename
        // works; it is the overwrite that the system refuses, see CORE-02).
        if dest.exists() {
            crate::paths::at(std::fs::rename(&dest, &old), &dest)?;
        }
        // CORE-02: the rename puts the DLL aside while it stays loadable (a
        // `.old-*` folder of a running session is purged by the next install).
        let put_back = |e| {
            if old.exists() {
                let _ = std::fs::rename(&old, &dest);
            }
            e
        };
        crate::paths::at(std::fs::rename(&staging, &dest).map_err(put_back), &dest)?;
        let _ = std::fs::remove_dir_all(&old); // happy path: no leftovers
        Ok(m.name)
    });
    let _ = std::fs::remove_dir_all(&staging);
    result
}

/// Windows' console-device names: a folder so named cannot be opened once made.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The plugin's name as folder name, or `None` when it cannot make a sane
/// folder. Without a real name (only punctuation, `..`, emptiness) or with a
/// Windows' reserved device name, reject. Everything troublesome, separators
/// and dots included, goes `_`: never walk out of the root with a manifest
/// name (CORE-01).
fn folder_name(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || !name.chars().any(|c| c.is_alphanumeric()) {
        return None;
    }
    let out: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || "-_ ".contains(c) { c } else { '_' })
        .collect();
    if RESERVED.contains(&out.to_uppercase().as_str()) {
        return None;
    }
    Some(out)
}

/// Copies the plugin's files into `to`.
fn stage(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_dir() {
        return copy_tree(&source_dir(from)?, to);
    }
    if from.file_name().is_some_and(|n| n == "plugin.json") {
        return copy_tree(from.parent().ok_or("plugin.json has no folder")?, to);
    }
    if is_zip(from) {
        return unzip(from, to);
    }
    Err(format!(
        "{}: not a plugin (a folder with a plugin.json, that file, or a .zip of the folder)",
        from.display()
    ))
}

/// The folder holding `plugin.json`: `dir` itself or its only sub-folder that has one.
fn source_dir(dir: &Path) -> Result<PathBuf, String> {
    if dir.join("plugin.json").is_file() {
        return Ok(dir.to_owned());
    }
    let inner: Vec<PathBuf> = crate::paths::at(std::fs::read_dir(dir), dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("plugin.json").is_file())
        .collect();
    match &inner[..] {
        [one] => Ok(one.clone()),
        [] => Err(format!("{}: no plugin.json", dir.display())),
        _ => Err(format!(
            "{}: holds several plugins; drop them one by one",
            dir.display()
        )),
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    crate::paths::at(std::fs::create_dir_all(to), to)?;
    for e in crate::paths::at(std::fs::read_dir(from), from)?.flatten() {
        let name = e.file_name();
        if SKIP.iter().any(|s| name == *s) {
            continue;
        }
        let (src, dst) = (e.path(), to.join(&name));
        let kind = crate::paths::at(e.file_type(), &src)?;
        if kind.is_dir() {
            copy_tree(&src, &dst)?;
        } else if kind.is_file() {
            crate::paths::at(std::fs::copy(&src, &dst), &src)?;
        }
    }
    Ok(())
}

/// The entry prefix the plugin's files sit under in `z`: `""` (plugin.json at the root) or `"top/"` (one top folder).
fn zip_prefix(z: &Zip) -> Result<String, String> {
    if z.has("plugin.json") {
        return Ok(String::new());
    }
    let tops: Vec<&str> = z
        .names()
        .filter_map(|n| {
            n.strip_suffix("plugin.json")
                .filter(|p| p.ends_with('/') && p.matches('/').count() == 1)
        })
        .collect();
    match &tops[..] {
        [one] => Ok(one.to_string()),
        [] => Err("the zip holds no plugin.json".into()),
        _ => Err("the zip holds several plugins".into()),
    }
}

fn unzip(from: &Path, to: &Path) -> Result<(), String> {
    let mut z = Zip::open(from).map_err(|e| format!("{}: {e:?}", from.display()))?;
    let prefix = zip_prefix(&z)?;
    z.unpack(&prefix, to, &SKIP)
        .map_err(|e| format!("{}: {e}", from.display()))
}

/// Whether `p` was installed by the user (it lives in [`user_dir`]); a bundled one is hidden by [`uninstall`] instead.
pub fn removable(p: &Plugin) -> bool {
    user_dir().is_some_and(|u| p.dir.starts_with(u))
}

/// Uninstalls a plugin: the user's copy is deleted and the name listed as removed, which hides a bundled copy too.
pub fn uninstall(p: &Plugin) -> Result<(), String> {
    if removable(p) {
        crate::paths::at(std::fs::remove_dir_all(&p.dir), &p.dir)?;
    }
    let name = &p.manifest.name;
    super::config::update(|c| {
        if !c.removed.contains(name) {
            c.removed.push(name.clone());
            c.removed.sort();
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rk-install-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    const JSON: &str = r#"{"name": "hello", "version": "1", "script": "main.lua"}"#;

    #[test]
    fn a_folder_a_zip_or_its_plugin_json_installs_and_replaces_an_older_copy() {
        let t = temp("ok");
        let src = t.join("download/hello-plugin");
        fs::create_dir_all(src.join("lib")).unwrap();
        fs::create_dir_all(src.join(".git")).unwrap();
        fs::write(src.join("plugin.json"), JSON).unwrap();
        fs::write(src.join("main.lua"), "function handle() end").unwrap();
        fs::write(src.join("lib/util.lua"), "return {}").unwrap();
        let root = t.join("plugins");
        // The folder around the plugin folder works too.
        assert_eq!(install_into(&t.join("download"), &root).unwrap(), "hello");
        assert!(root.join("hello/lib/util.lua").is_file() && !root.join("hello/.git").exists());
        fs::write(src.join("plugin.json"), JSON.replace("\"1\"", "\"2\"")).unwrap();
        fs::remove_file(src.join("lib/util.lua")).unwrap();
        assert_eq!(
            install_into(&src.join("plugin.json"), &root).unwrap(),
            "hello"
        );
        assert!(fs::read_to_string(root.join("hello/plugin.json"))
            .unwrap()
            .contains("\"2\""));
        assert!(
            !root.join("hello/lib/util.lua").exists(),
            "the old copy is replaced, not merged"
        );
        let zip = t.join("hello.zip");
        fs::write(
            &zip,
            crate::zip::write(&[
                ("hello/plugin.json", JSON.into()),
                ("hello/main.lua", b"x".to_vec()),
            ]),
        )
        .unwrap();
        assert!(is_plugin(&zip) && is_plugin(&src) && !is_plugin(&t));
        assert_eq!(install_into(&zip, &root).unwrap(), "hello");
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            1,
            "no staging folder is left"
        );
        fs::remove_dir_all(t).ok();
    }

    #[test]
    fn a_broken_plugin_or_an_escaping_zip_changes_nothing() {
        let t = temp("bad");
        let root = t.join("plugins");
        let zip = t.join("evil.zip");
        fs::write(
            &zip,
            crate::zip::write(&[("plugin.json", JSON.into()), ("../../x.lua", b"x".to_vec())]),
        )
        .unwrap();
        assert!(install_into(&zip, &root)
            .unwrap_err()
            .contains("leaves the folder"));
        let src = t.join("p");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("plugin.json"), JSON).unwrap(); // main.lua is missing
        assert!(install_into(&src, &root).unwrap_err().contains("main.lua"));
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir_all(t).ok();
    }

    #[test]
    fn a_manifest_name_cannot_walk_out_of_the_extension_root() {
        let t = temp("traverse");
        let root = t.join("plugins");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep"), "kept").unwrap();
        fs::create_dir_all(root.join("extension/plugin/installed")).unwrap();
        fs::write(root.join("extension/plugin/installed/x"), "x").unwrap();
        let src = t.join("p");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("plugin.json"), JSON).unwrap();
        fs::write(
            src.join("main.lua"),
            "function original() end",
        ).unwrap();
        let install_named = |name: &str| {
            fs::write(
                src.join("plugin.json"),
                JSON.replace("hello", &name),
            ).unwrap();
            install_into(&src, &root)
        };
        // Before the fix, a manifiesto `..` made the installer erase the whole
        // extension folder on the way to its failure.
        for bad in ["..", ".", "​", "NUL", "con", "​..."] {
            assert!(install_named(bad).is_err(), "{bad:?} must not install");
        }
        assert!(
            root.join("keep").is_file()
                && root.join("extension/plugin/installed/x").is_file(),
            "nothing outside the plugin name is touched by a rejected name"
        );
        // A Windows' name in the wild unchanged.
        assert!(install_named("Hi-Fi_IPod_v2").is_ok());
        fs::remove_dir_all(t).ok();
    }

    #[test]
    fn folder_names_sanitize_reject_walk_and_devices() {
        assert_eq!(folder_name("hello"), Some("hello".to_owned()));
        assert_eq!(folder_name("Hello World-v2"), Some("Hello World-v2".to_owned()));
        assert_eq!(folder_name("a/b"), Some("a_b".to_owned()));
        assert_eq!(folder_name(".git"), Some("_git".to_owned()));
        for bad in ["..", ".", "​", "", "NUL", "com3", "LPT1", "­..."] {
            assert_eq!(folder_name(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_failed_update_leaves_the_old_copy_in_place() {
        let t = temp("transactional");
        let root = t.join("plugins");
        let src = t.join("p");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("plugin.json"), JSON).unwrap();
        fs::write(src.join("main.lua"), "x").unwrap();
        install_into(&src, &root).unwrap();
        fs::write(root.join("hello/main.lua"), "v2").unwrap();
        // Exactly what the updater does now: set the old copy aside with a
        // rename — allowed on Windows even when a DLL inside stays loaded,
        // where overwriting or deleting would refuse (CORE-02).
        let old = root.join(format!(".old-{}", std::process::id()));
        assert!(std::fs::rename(root.join("hello"), &old).is_ok());
        // And what a failed updater does on top: put it back (CORE-07).
        assert!(std::fs::rename(&old, root.join("hello")).is_ok());
        assert!(
            fs::read_to_string(root.join("hello/main.lua"))
                .is_ok_and(|c| c == "v2"),
            "the previous version survives any failed update attempt"
        );
        fs::remove_dir_all(t).ok();
    }
}
