//! The playlist tree beside the track table: what every sidebar row is ([`Entry`]) and the label it shows.
//!
//! Top to bottom: All tracks, Tag List, Session history, Duplicate tracks (the first three rows keep their
//! numbers, `BrowserState.tag-row` is row 1), then **My Playlists** (our folders / playlists / smart playlists from
//! the store), **History** (one entry per day the store logged), **Tags** (categories, their tags) and **Library**
//! (the opened source's own playlists, read-only; its "Histories" folder is among them).

use crate::app::TreeRow;
use openfader::{
    library::{Playlist, Track},
    plugin::device::MAX_BANKS,
    store::{HotCueBank, Tag, TagGroup, Node, NodeKind},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Entry {
    All,
    Tags,
    Session,
    Dupes,
    UserRoot,
    /// A node of My Playlists (store id).
    User(i64),
    /// "Cue Bank" heading above the lists (Preferences > View > Layout > Media Browser).
    BankRoot,
    /// A Cue Bank of the store (id).
    Bank(i64),
    HistoryRoot,
    /// A day of the stored play history, `YYYY-MM-DD`.
    Day(String),
    TagsRoot,
    Category(i64),
    Tag(i64),
    LibRoot,
    /// Index into the library's `playlists()`.
    Lib(usize),
    /// "All Tracks" first under My Playlists (Preferences > View > Layout > Display All Tracks in the Playlists).
    PlaylistsAll,
    /// The CUE Analysis playlist (Preferences > View > Layout > Browser panel): tracks CUE Analysis set cues on.
    CueAnalysis,
    /// "Devices" heading above the mounted devices.
    DevicesRoot,
    /// "Favorites" heading above the folders the user pinned.
    FavoritesRoot,
    /// A folder the user pinned: its index in the sources.
    Pinned(usize),
    /// "Downloaded" heading above what the plugin sources downloaded.
    /// Kept for the old tree's hints; the redesigned tree lists downloads under each provider.
    #[allow(dead_code)]
    DownloadedRoot,
    /// "Local Music" heading above the system folders (Music, Videos, Desktop).
    LocalMusicRoot,
    /// A system folder of Windows: Music, Videos, Desktop.
    SystemFolder(usize),
    /// "Online Music" heading above the cloud plugins.
    OnlineMusicRoot,
    /// A cloud plugin source (Deezer, YouTube, SoundCloud): its sub-entries (Search, Downloads…) are listed under it.
    OnlinePlugin(usize),
    /// Search entry of a cloud plugin: (plugin index, sub-index).
    OnlineSearch(usize, usize),
    /// Downloads entry of a cloud plugin: (plugin index, sub-index).
    OnlineDownloads(usize, usize),
    /// A playlist entry of a cloud plugin: (plugin index, playlist index).
    OnlinePlaylist(usize, usize),
    /// An external source (a device, iTunes, library xml, Explorer) by index into the browser's sources; it opens when chosen.
    Source(usize),
    /// "All tracks" of an opened source.
    SourceAll(usize),
    /// A top-level folder of a device: (device, index into its folders).
    DeviceFolder(usize, usize),
    /// A folder inside a device's folder: (device, folder, index into its playlists).
    DeviceFolderList(usize, usize, usize),
    /// The tracks of an opened source the Collection does not have.
    SourceMissing(usize),
    /// A playlist of an opened source: (source, index into its playlists).
    SourceList(usize, usize),
}

/// One external source as the tree shows it.
pub struct SourceRows {
    pub device: bool,
    /// A local volume of the machine (the system disk): it is browsed by its folders, like a device but read-only.
    pub local: bool,
    /// The folder a plugin source downloaded into (the "Downloaded" section).
    pub downloaded: bool,
    /// A folder the user pinned (the "Favorites" section).
    pub pinned: bool,
    /// A system folder of Windows: Music, Videos, Desktop.
    pub system_folder: bool,
    /// A cloud plugin source (Deezer, YouTube, SoundCloud): its sub-entries are listed under it.
    pub online_plugin: bool,
    pub name: String,
    /// Opened: its "All tracks" and playlists are listed under it.
    pub open: bool,
    pub playlists: Vec<Playlist>,
    /// The tracks of the source the Collection does not have.
    pub missing: Vec<Track>,
    /// A device's top-level folders (the ones with audio), each with its name and its subfolders.
    pub folders: Vec<FolderRow>,
}

/// A top-level folder of a device, as the tree shows it.
pub struct FolderRow {
    pub name: String,
    /// The folder's own subfolders, once it has been read.
    pub playlists: Vec<Playlist>,
}

pub struct Inputs<'a> {
    pub tag_list: usize,
    pub session: usize,
    pub lib: &'a [Playlist],
    pub nodes: &'a [Node],
    /// Days with plays, newest first, and how many tracks each has.
    pub days: &'a [(String, usize)],
    pub cats: &'a [TagGroup],
    pub tags: &'a [Tag],
    pub sources: &'a [SourceRows],
    pub all_tracks: bool,
    /// Size of the CUE Analysis playlist when it is shown.
    pub cue_analysis: Option<usize>,
    /// The store's Cue Banks when the Media Browser shows them.
    pub banks: Option<&'a [HotCueBank]>,
    /// Which sections of the tree are shown (Preferences > View > Layout > Media Browser).
    pub sections: Sections,
    /// The sources whose children are hidden (the user folded the branch).
    pub folded: &'a HashSet<usize>,
    /// The lists of the library whose children are hidden (the user folded the branch).
    pub folded_lib: &'a HashSet<usize>,
    /// The tree's root sections the user folded (History, Tags, Devices, Downloaded, Local Music, Online Music).
    pub folded_sections: &'a HashSet<Entry>,
    /// The device folders the user folded: (source, folder, relative path; "" for its top-level row).
    pub folded_folders: &'a HashSet<(usize, usize, String)>,
    /// Day → custom display name for a renamed history session.
    pub day_names: &'a HashMap<String, String>,
}

/// The sections of the tree the user can hide. All of them show by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sections {
    pub favorites: bool,
    pub downloaded: bool,
    pub devices: bool,
    pub local_music: bool,
    pub online_music: bool,
}

impl Default for Sections {
    fn default() -> Self {
        Self {
            favorites: true,
            downloaded: true,
            devices: true,
            local_music: true,
            online_music: true,
        }
    }
}

fn indent(depth: usize, label: &str) -> String {
    format!("{}{label}", "    ".repeat(depth))
}

/// Label of a library playlist `base` levels in, plus its own depth; folders get the `▸` marker.
fn playlist_label(base: usize, p: &Playlist, open: bool) -> String {
    indent(
        base + p.depth,
        &format!("{}{}", if p.is_folder {
            if open { "▾ " } else { "▸ " }
        } else {
            ""
        }, p.name),
    )
}

/// Every row's entry and label, in display order.
pub fn build(i: &Inputs) -> (Vec<Entry>, Vec<String>) {
    let mut rows: Vec<(Entry, String)> = Vec::new();
    // Favorites stay at the top, where the user expects them.
    let pinned: Vec<usize> = i
        .sources
        .iter()
        .enumerate()
        .filter(|(_, s)| s.pinned)
        .map(|(n, _)| n)
        .collect();
    if i.sections.favorites {
        rows.push((Entry::FavoritesRoot, "▸ Favorites".to_owned()));
        for n in &pinned {
            rows.push((Entry::Pinned(*n), indent(1, &format!("★ {}", i.sources[*n].name))));
        }
    }
    rows.push((Entry::All, "All tracks".to_owned()));
    rows.push((Entry::Tags, format!("Tag List ({})", i.tag_list)));
    rows.push((Entry::Session, format!("Session history ({})", i.session)));
    rows.push((Entry::Dupes, "Duplicate tracks".to_owned()));
    if let Some(n) = i.cue_analysis {
        rows.push((Entry::CueAnalysis, format!("CUE Analysis ({n})")));
    }
    // My Playlists: a foldable heading over the user's folders / playlists.
    let user_folded = i.folded_sections.contains(&Entry::UserRoot);
    rows.push((
        Entry::UserRoot,
        if user_folded { "▸ My Playlists" } else { "▾ My Playlists" }.to_owned(),
    ));
    if !user_folded {
        if i.all_tracks {
            rows.push((Entry::PlaylistsAll, indent(1, "All Tracks")));
        }
        let mut depth: HashMap<i64, usize> = HashMap::new();
        for n in i.nodes {
            // Nodes come depth-first, so the parent is already known; an unknown parent counts as top level.
            let d = n.parent.and_then(|p| depth.get(&p)).map_or(1, |d| d + 1);
            depth.insert(n.id, d);
            let label = match n.kind {
                NodeKind::Folder => format!("▸ {}", n.name),
                NodeKind::Playlist => n.name.clone(),
                NodeKind::Smart => format!("{} (smart)", n.name),
            };
            rows.push((Entry::User(n.id), indent(d, &label)));
        }
    }
    if let Some(banks) = i.banks {
        rows.push((Entry::BankRoot, "▸ Cue Bank".to_owned()));
        rows.extend(banks.iter().map(|b| {
            let label = format!("{} ({}/{MAX_BANKS})", b.name, b.slots.len());
            (Entry::Bank(b.id), indent(1, &label))
        }));
    }
    let history_folded = i.folded_sections.contains(&Entry::HistoryRoot);
    rows.push((
        Entry::HistoryRoot,
        if history_folded { "▸ History" } else { "▾ History" }.to_owned(),
    ));
    if !history_folded {
        rows.extend(
            i.days
                .iter()
                .map(|(day, n)| {
                    let name = i.day_names.get(day).cloned().unwrap_or_else(|| day.clone());
                    (Entry::Day(day.clone()), indent(1, &format!("{} ({n})", name)))
                }),
        );
    }
    let tags_folded = i.folded_sections.contains(&Entry::TagsRoot);
    rows.push((Entry::TagsRoot, if tags_folded { "▸ Tags" } else { "▾ Tags" }.to_owned()));
    if !tags_folded {
        for c in i.cats {
            rows.push((Entry::Category(c.id), indent(1, &format!("▸ {}", c.name))));
            rows.extend(
                i.tags
                    .iter()
                    .filter(|t| t.category == c.id)
                    .map(|t| (Entry::Tag(t.id), indent(2, &t.name))),
            );
        }
    }
    // ── Local Music: system folders (Music, Videos, Desktop) and the disks ─────
    if i.sections.local_music {
        let local_folded = i.folded_sections.contains(&Entry::LocalMusicRoot);
        rows.push((
            Entry::LocalMusicRoot,
            if local_folded { "▸ Local Music" } else { "▾ Local Music" }.to_owned(),
        ));
        if !local_folded {
            let system: Vec<usize> = i
                .sources
                .iter()
                .enumerate()
                .filter(|(_, s)| s.system_folder)
                .map(|(n, _)| n)
                .collect();
            for n in &system {
                rows.push((
                    Entry::SystemFolder(*n),
                    indent(1, &format!("▸ {}", i.sources[*n].name)),
                ));
            }
            // Disks: devices and local volumes under their own heading.
            let disks: Vec<usize> = i
                .sources
                .iter()
                .enumerate()
                .filter(|(_, s)| s.device || s.local)
                .map(|(n, _)| n)
                .collect();
            if !disks.is_empty() {
                let disks_folded = i.folded_sections.contains(&Entry::DevicesRoot);
                rows.push((
                    Entry::DevicesRoot,
                    indent(1, if disks_folded { "▸ Devices" } else { "▾ Devices" }),
                ));
                if !disks_folded {
                    for n in &disks {
                        source_rows(&mut rows, i, *n, 2);
                    }
                }
            }
        }
    }
    // ── Online Music: cloud plugins (YouTube, SoundCloud, Deezer) ─────────────
    if i.sections.online_music {
        let online_folded = i.folded_sections.contains(&Entry::OnlineMusicRoot);
        rows.push((
            Entry::OnlineMusicRoot,
            if online_folded { "▸ Online Music" } else { "▾ Online Music" }.to_owned(),
        ));
        if !online_folded {
            let online: Vec<usize> = i
                .sources
                .iter()
                .enumerate()
                .filter(|(_, s)| s.online_plugin)
                .map(|(n, _)| n)
                .collect();
            for n in &online {
                rows.push((
                    Entry::OnlinePlugin(*n),
                    indent(1, &format!("▸ {}", i.sources[*n].name)),
                ));
                rows.push((Entry::OnlineSearch(*n, 0), indent(2, "Search")));
                rows.push((Entry::OnlineDownloads(*n, 0), indent(2, "Downloads")));
                // YouTube also offers its popular music videos. When the plugin lists it as a
                // playlist we show that; otherwise the search entry is the closest it offers.
                if i.sources[*n].name.to_lowercase().contains("youtube") {
                    let popular = i.sources[*n]
                        .playlists
                        .iter()
                        .position(|p| !p.is_folder && p.name.to_lowercase().contains("popular"));
                    match popular {
                        Some(j) => rows.push((
                            Entry::OnlinePlaylist(*n, j),
                            indent(2, "Popular Music Videos"),
                        )),
                        None => rows.push((
                            Entry::OnlineSearch(*n, 1),
                            indent(2, "Popular Music Videos"),
                        )),
                    }
                }
                // SoundCloud's example playlists come from the library plugin's lists.
                for (j, p) in i.sources[*n].playlists.iter().enumerate() {
                    if !p.is_folder {
                        rows.push((Entry::OnlinePlaylist(*n, j), indent(2, &p.name)));
                    }
                }
            }
        }
    }
    // ── Library (read-only): only when it has something to show ───────────────
    if !i.lib.is_empty() {
        rows.push((Entry::LibRoot, "▸ Library (read-only)".to_owned()));
        // A list with lists under it folds them (the plugin sources come this way).
        let mut at = 0;
        while at < i.lib.len() {
            let p = &i.lib[at];
            let children = i.lib[at + 1..]
                .iter()
                .take_while(|c| c.depth > p.depth)
                .count();
            rows.push((Entry::Lib(at), playlist_label(1, p, false)));
            if children > 0 && !i.folded_lib.contains(&at) {
                rows.extend(
                    i.lib[at + 1..at + 1 + children]
                        .iter()
                        .enumerate()
                        .map(|(k, c)| (Entry::Lib(at + 1 + k), playlist_label(1, c, false))),
                );
            }
            at += 1 + children;
        }
    }
    // ── Other sources (Explorer, iTunes, xml plugins) on their own ────────────
    for (n, s) in i.sources.iter().enumerate() {
        if s.downloaded || s.pinned || s.system_folder || s.online_plugin || s.device || s.local {
            continue; // already listed above
        }
        source_rows(&mut rows, i, n, 0);
    }
    rows.into_iter().unzip()
}

/// The icons the tree shows by entry. `icon` is a name the Slint side maps to its `Icon` enum, so a
/// provider can get its own glyph without Rust knowing the enum.
fn provider_icon(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("youtube") {
        "youtube"
    } else if n.contains("soundcloud") {
        "soundcloud"
    } else if n.contains("deezer") {
        "deezer"
    } else {
        "link"
    }
}

fn icon_of(entry: &Entry, i: &Inputs) -> &'static str {
    match entry {
        Entry::All | Entry::PlaylistsAll => "list",
        Entry::Tags | Entry::TagsRoot | Entry::Category(_) | Entry::Tag(_) => "tag",
        Entry::Session | Entry::HistoryRoot | Entry::Day(_) => "loop",
        Entry::Dupes => "grid",
        Entry::CueAnalysis => "star",
        Entry::UserRoot | Entry::SystemFolder(_) => "folder",
        Entry::User(_) => "list",
        Entry::BankRoot | Entry::Bank(_) => "mixer",
        Entry::LibRoot | Entry::Lib(_) => "library",
        Entry::DevicesRoot => "usb",
        Entry::DownloadedRoot => "download",
        Entry::FavoritesRoot | Entry::Pinned(_) => "star",
        Entry::LocalMusicRoot => "folder",
        Entry::OnlineMusicRoot => "link",
        Entry::OnlinePlugin(n) => i.sources.get(*n).map_or("link", |s| provider_icon(&s.name)),
        Entry::OnlineSearch(_, _) => "search",
        Entry::OnlineDownloads(_, _) => "download",
        Entry::OnlinePlaylist(_, _) => "list",
        Entry::Source(n) => match i.sources.get(*n) {
            Some(s) if s.online_plugin => provider_icon(&s.name),
            Some(s) if s.device => "usb",
            Some(s) if s.system_folder => "folder",
            _ => "link",
        },
        Entry::SourceAll(_) | Entry::SourceList(_, _) | Entry::SourceMissing(_) => "list",
        Entry::DeviceFolder(_, _) | Entry::DeviceFolderList(_, _, _) => "folder",
    }
}

/// Turns the labels `build` made into UI rows: depth from the leading spaces, branch/open from the
/// `▸`/`▾` marker, and the icon for each entry.
pub fn tree_rows(entries: &[Entry], labels: &[String], i: &Inputs) -> Vec<TreeRow> {
    entries
        .iter()
        .zip(labels)
        .map(|(entry, label)| {
            let depth = (label.len() - label.trim_start_matches(' ').len()) / 4;
            let text = &label[depth * 4..];
            let (branch, open, text) = if let Some(rest) = text.strip_prefix("▾ ") {
                (true, true, rest)
            } else if let Some(rest) = text.strip_prefix("▸ ") {
                (true, false, rest)
            } else {
                (false, false, text)
            };
            let text = text.strip_prefix("★ ").unwrap_or(text);
            TreeRow {
                label: text.into(),
                icon: icon_of(entry, i).into(),
                depth: depth as i32,
                branch,
                open,
            }
        })
        .collect()
}

/// The Tag List row, rebuilt when its count changes.
pub fn tag_row(n: usize) -> TreeRow {
    TreeRow {
        label: format!("Tag List ({n})").into(),
        icon: "tag".into(),
        depth: 0,
        branch: false,
        open: false,
    }
}

/// The Session history row, rebuilt when its count changes.
pub fn session_row(n: usize) -> TreeRow {
    TreeRow {
        label: format!("Session history ({n})").into(),
        icon: "loop".into(),
        depth: 0,
        branch: false,
        open: false,
    }
}

/// Row of `entry` in `entries` (0 when it is not there).
pub fn row_of(entries: &[Entry], entry: &Entry) -> usize {
    entries.iter().position(|e| e == entry).unwrap_or(0)
}

/// The rows of the source `n`: its own row and, when it is open, its tracks, its playlists and the tracks the
/// Collection does not have.
fn source_rows(rows: &mut Vec<(Entry, String)>, i: &Inputs, n: usize, d: usize) {
    let s = &i.sources[n];
    // A folded branch shows only its own row: the user opens it with a click.
    let folded = i.folded.contains(&n);
    rows.push((
        Entry::Source(n),
        indent(
            d,
            &if s.device {
                s.name.clone()
            } else {
                format!("▸ {}", s.name)
            },
        ),
    ));
    if folded {
        return;
    }
    // A device's own folders: each opens as a source, with its tracks and its subfolders.
    for (j, f) in s.folders.iter().enumerate() {
        let root = i.folded_folders.contains(&(n, j, String::new()));
        rows.push((
            Entry::DeviceFolder(n, j),
            indent(
                d + 1,
                &format!("{} {}", if root { "▸" } else { "▾" }, f.name),
            ),
        ));
        if root {
            continue;
        }
        let mut hide_below: Option<usize> = None;
        for (k, p) in f.playlists.iter().enumerate() {
            if hide_below.is_some_and(|depth| p.depth > depth) {
                continue;
            }
            if hide_below.is_some_and(|depth| p.depth <= depth) {
                hide_below = None;
            }
            let folded = i.folded_folders.contains(&(n, j, p.id.clone()));
            rows.push((
                Entry::DeviceFolderList(n, j, k),
                playlist_label(d + 2, p, !folded),
            ));
            if p.is_folder && folded {
                hide_below = Some(p.depth);
            }
        }
    }
    if s.open {
        rows.push((Entry::SourceAll(n), indent(d + 1, "All tracks")));
        rows.extend(
            s.playlists
                .iter()
                .enumerate()
                .map(|(j, p)| (Entry::SourceList(n, j), playlist_label(d + 1, p, false))),
        );
        if !s.missing.is_empty() {
            rows.push((
                Entry::SourceMissing(n),
                indent(d + 1, "Not in the Collection"),
            ));
        }
    }
}

/// The library playlist `at` with everything nested under it, as (index, depth below `at`); `at` itself first.
/// Empty when `at` is out of range.
pub fn subtree(lib: &[Playlist], at: usize) -> Vec<(usize, usize)> {
    let Some(root) = lib.get(at) else {
        return Vec::new();
    };
    let nested = lib[at + 1..]
        .iter()
        .take_while(|p| p.depth > root.depth)
        .enumerate();
    std::iter::once((at, 0))
        .chain(nested.map(|(n, p)| (at + 1 + n, p.depth - root.depth)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pl(name: &str, depth: usize, is_folder: bool) -> Playlist {
        Playlist {
            id: name.into(),
            name: name.into(),
            depth,
            is_folder,
        }
    }

    fn node(id: i64, parent: Option<i64>, name: &str, kind: NodeKind) -> Node {
        Node {
            id,
            parent,
            name: name.into(),
            kind,
            pos: 0,
        }
    }

    #[test]
    fn rows_come_in_the_documented_order_with_indentation() {
        let lib = [pl("Folder", 0, true), pl("Inner", 1, false)];
        let nodes = [
            node(1, None, "Set", NodeKind::Folder),
            node(2, Some(1), "Warmup", NodeKind::Playlist),
            node(3, None, "House", NodeKind::Smart),
        ];
        let days = [("2026-10-13".to_owned(), 3), ("2026-10-12".to_owned(), 1)];
        let cats = [
            TagGroup {
                id: 5,
                name: "Genre".into(),
                pos: 0,
            },
            TagGroup {
                id: 6,
                name: "Mood".into(),
                pos: 1,
            },
        ];
        let tags = [
            Tag {
                id: 9,
                category: 5,
                name: "Deep".into(),
                pos: 0,
            },
            Tag {
                id: 10,
                category: 6,
                name: "Dark".into(),
                pos: 0,
            },
        ];
        let banks = [HotCueBank {
            id: 7,
            name: "Peak".into(),
            slots: Vec::new(),
        }];
        let (entries, labels) = build(&Inputs {
            tag_list: 2,
            session: 4,
            lib: &lib,
            nodes: &nodes,
            days: &days,
            cats: &cats,
            tags: &tags,
            sources: &[],
            all_tracks: false,
            cue_analysis: None,
            banks: Some(&banks),
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &HashSet::new(),
            day_names: &HashMap::new(),
        });
        assert_eq!(entries[2], Entry::Tags);
        assert_eq!(entries[3], Entry::Session);
        assert_eq!(
            labels,
            [
                "▸ Favorites",
                "All tracks",
                "Tag List (2)",
                "Session history (4)",
                "Duplicate tracks",
                "▾ My Playlists",
                "    ▸ Set",
                "        Warmup",
                "    House (smart)",
                "▸ Cue Bank",
                "    Peak (0/8)",
                "▾ History",
                "    2026-10-13 (3)",
                "    2026-10-12 (1)",
                "▾ Tags",
                "    ▸ Genre",
                "        Deep",
                "    ▸ Mood",
                "        Dark",
                "▾ Local Music",
                "▾ Online Music",
                "▸ Library (read-only)",
                "    ▸ Folder",
                "        Inner",
            ]
        );
        assert_eq!(entries.len(), labels.len());
        assert_eq!(entries[6], Entry::User(1));
        assert_eq!(entries[10], Entry::Bank(7));
        assert_eq!(entries[12], Entry::Day("2026-10-13".into()));
        assert_eq!(entries[16], Entry::Tag(9));
        assert_eq!(entries[23], Entry::Lib(1));
    }

    #[test]
    fn an_empty_store_still_shows_every_section() {
        let (e, _) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &[],
            all_tracks: false,
            cue_analysis: None,
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &HashSet::new(),
            day_names: &HashMap::new(),
        });
        assert_eq!(
            e,
            [
                Entry::FavoritesRoot,
                Entry::All,
                Entry::Tags,
                Entry::Session,
                Entry::Dupes,
                Entry::UserRoot,
                Entry::HistoryRoot,
                Entry::TagsRoot,
                Entry::LocalMusicRoot,
                Entry::OnlineMusicRoot,
            ]
        );
    }

    #[test]
    fn tree_rows_carry_depth_icons_and_branch_state() {
        let entries = [Entry::All, Entry::Tags, Entry::DevicesRoot];
        let labels = [
            "All tracks".to_owned(),
            "    Tag List (2)".to_owned(),
            "    ▾ Devices".to_owned(),
        ];
        let rows = tree_rows(
            &entries,
            &labels,
            &Inputs {
                tag_list: 2,
                session: 0,
                lib: &[],
                nodes: &[],
                days: &[],
                cats: &[],
                tags: &[],
                sources: &[],
                all_tracks: false,
                cue_analysis: None,
                banks: None,
                sections: Sections::default(),
                folded: &HashSet::new(),
                folded_lib: &HashSet::new(),
                folded_sections: &HashSet::new(),
                folded_folders: &HashSet::new(),
                day_names: &HashMap::new(),
            },
        );
        let row = |i: usize| (rows[i].label.as_str(), rows[i].depth, rows[i].icon.as_str());
        assert_eq!(row(0), ("All tracks", 0, "list"));
        assert_eq!(row(1), ("Tag List (2)", 1, "tag"));
        assert_eq!(row(2), ("Devices", 1, "usb"));
        assert!(rows[2].branch && rows[2].open);
        assert!(!rows[0].branch);
    }

    /// The global Downloaded section is gone: each provider lists its own downloads, and the
    /// read-only Library only shows when the opened source has playlists.
    #[test]
    fn the_global_downloaded_and_the_empty_library_are_gone() {
        let src = |name: &str, device, downloaded| SourceRows {
            device,
            local: false,
            downloaded,
            pinned: false,
            system_folder: false,
            online_plugin: false,
            name: name.into(),
            open: false,
            playlists: Vec::new(),
            missing: Vec::new(),
            folders: Vec::new(),
        };
        let sources = [
            src("YouTube", false, true),
            src("Explorer", false, false),
            src("USB", true, false),
        ];
        let (entries, labels) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &sources,
            all_tracks: false,
            cue_analysis: None,
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &HashSet::new(),
            day_names: &HashMap::new(),
        });
        assert!(!labels.iter().any(|l| l.contains("Downloaded")));
        assert!(!labels.iter().any(|l| l.contains("Library (read-only)")));
        assert!(entries.contains(&Entry::DevicesRoot));
        assert!(labels.iter().any(|l| l == "▸ Explorer"));
        assert!(!labels.iter().any(|l| l == "▸ YouTube"));
    }

    /// A device lists its top-level folders (the ones with audio) under it, each with its own subfolders.
    #[test]
    fn a_device_lists_its_own_folders() {
        let sources = [SourceRows {
            device: true,
            local: false,
            downloaded: false,
            pinned: false,
            system_folder: false,
            online_plugin: false,
            name: "USB (vfat)".into(),
            open: false,
            playlists: Vec::new(),
            missing: Vec::new(),
            folders: vec![FolderRow {
                name: "Musica".into(),
                playlists: vec![pl("Rock", 0, false)],
            }],
        }];
        let (entries, labels) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &sources,
            all_tracks: false,
            cue_analysis: None,
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &HashSet::new(),
            day_names: &HashMap::new(),
        });
        let at = labels
            .iter()
            .position(|l| l == "    ▾ Devices")
            .expect("the devices hang under Local Music");
        assert_eq!(entries[at + 1], Entry::Source(0));
        assert_eq!(labels[at + 1], "        USB (vfat)");
        // The folder and the folder inside it.
        assert_eq!(entries[at + 2], Entry::DeviceFolder(0, 0));
        assert_eq!(labels[at + 2], "            ▾ Musica");
        assert_eq!(entries[at + 3], Entry::DeviceFolderList(0, 0, 0));
        assert_eq!(labels[at + 3], "                Rock");
    }

    #[test]
    fn a_folder_folds_and_unfolds_its_subtree() {
        let sources = [SourceRows {
            device: true,
            local: false,
            downloaded: false,
            pinned: false,
            system_folder: false,
            online_plugin: false,
            name: "USB (vfat)".into(),
            open: false,
            playlists: Vec::new(),
            missing: Vec::new(),
            folders: vec![FolderRow {
                name: "Musica".into(),
                playlists: vec![pl("Set", 0, true), pl("Inner", 1, false), pl("Other", 0, false)],
            }],
        }];
        let folded: HashSet<(usize, usize, String)> =
            HashSet::from([(0, 0, "Set".to_owned())]);
        let (entries, labels) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &sources,
            all_tracks: false,
            cue_analysis: None,
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &folded,
            day_names: &HashMap::new(),
        });
        let at = labels
            .iter()
            .position(|l| l == "    ▾ Devices")
            .expect("the devices hang under Local Music");
        assert_eq!(entries[at + 2], Entry::DeviceFolder(0, 0));
        assert_eq!(labels[at + 2], "            ▾ Musica");
        assert_eq!(entries[at + 3], Entry::DeviceFolderList(0, 0, 0));
        assert_eq!(labels[at + 3], "                ▸ Set");
        assert_eq!(entries[at + 4], Entry::DeviceFolderList(0, 0, 2));
        assert_eq!(labels[at + 4], "                Other");
        assert!(!labels.iter().any(|l| l.trim() == "Inner"));

        // Folding the top-level folder hides every row under it.
        let folded: HashSet<(usize, usize, String)> =
            HashSet::from([(0, 0, String::new())]);
        let (entries, labels) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &sources,
            all_tracks: false,
            cue_analysis: None,
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &folded,
            day_names: &HashMap::new(),
        });
        let at = labels
            .iter()
            .position(|l| l == "    ▾ Devices")
            .expect("the devices hang under Local Music");
        assert_eq!(entries[at + 2], Entry::DeviceFolder(0, 0));
        assert_eq!(labels[at + 2], "            ▸ Musica");
        assert!(!labels.iter().any(|l| ["Set", "Inner", "Other"].contains(&l.trim())));
    }

    /// The local disk (the system volume) hangs under the "Devices" heading with its own folders, read-only.
    #[test]
    fn the_local_disk_hangs_under_the_devices_with_its_folders() {
        let sources = [SourceRows {
            device: false,
            local: true,
            downloaded: false,
            pinned: false,
            system_folder: false,
            online_plugin: false,
            name: "C: (OS)".into(),
            open: false,
            playlists: Vec::new(),
            missing: Vec::new(),
            folders: vec![FolderRow {
                name: "Users".into(),
                playlists: Vec::new(),
            }],
        }];
        let (entries, labels) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &sources,
            all_tracks: false,
            cue_analysis: None,
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &HashSet::new(),
            day_names: &HashMap::new(),
        });
        let at = labels
            .iter()
            .position(|l| l == "    ▾ Devices")
            .expect("the devices hang under Local Music");
        assert_eq!(entries[at + 1], Entry::Source(0));
        assert_eq!(labels[at + 1], "        ▸ C: (OS)");
        // Its folder, under it.
        assert_eq!(entries[at + 2], Entry::DeviceFolder(0, 0));
        assert_eq!(labels[at + 2], "            ▾ Users");
    }

    /// A list with lists under it folds them: the plugin sources come this way.
    #[test]
    fn a_folded_list_hides_the_lists_under_it() {
        let lib = [
            pl("SoundCloud", 0, true),
            pl("Search", 1, false),
            pl("Diplo", 1, false),
            pl("Explorer", 0, false),
        ];
        let open = |folded: &HashSet<usize>| {
            let (entries, labels) = build(&Inputs {
                tag_list: 0,
                session: 0,
                lib: &lib,
                nodes: &[],
                days: &[],
                cats: &[],
                tags: &[],
                sources: &[],
                all_tracks: false,
                cue_analysis: None,
                banks: None,
                sections: Sections::default(),
                folded: &HashSet::new(),
                folded_lib: folded,
                folded_sections: &HashSet::new(),
                folded_folders: &HashSet::new(),
                day_names: &HashMap::new(),
            });
            (entries, labels)
        };
        // Open: the list and the two under it.
        let (entries, labels) = open(&HashSet::new());
        let at = labels.iter().position(|l| l == "    ▸ SoundCloud").unwrap();
        assert_eq!(labels[at + 1], "        Search");
        assert_eq!(labels[at + 2], "        Diplo");
        assert_eq!(entries[at + 1], Entry::Lib(1));
        // Folded: only the list itself.
        let (entries, labels) = open(&HashSet::from([0]));
        let at = labels.iter().position(|l| l == "    ▸ SoundCloud").unwrap();
        assert_eq!(labels[at + 1], "    Explorer");
        assert_eq!(entries[at + 1], Entry::Lib(3));
    }

    #[test]
    fn subtree_takes_the_folder_and_all_it_nests() {
        let lib = [
            pl("A", 0, true),
            pl("a1", 1, false),
            pl("a2", 1, true),
            pl("a2x", 2, false),
            pl("B", 0, false),
        ];
        assert_eq!(subtree(&lib, 0), [(0, 0), (1, 1), (2, 1), (3, 2)]);
        assert_eq!(subtree(&lib, 2), [(2, 0), (3, 1)]);
        assert_eq!(subtree(&lib, 4), [(4, 0)]);
        assert!(subtree(&lib, 9).is_empty());
    }

    #[test]
    fn devices_and_other_sources_open_into_their_playlists() {
        let src = |device, name: &str, open| SourceRows {
            device,
            local: false,
            downloaded: false,
            pinned: false,
            system_folder: false,
            online_plugin: false,
            name: name.into(),
            open,
            playlists: vec![pl("Set", 0, true), pl("Inner", 1, false)],
            missing: Vec::new(),
            folders: Vec::new(),
        };
        let sources = [
            src(true, "USB (vfat)", true),
            src(true, "SD", false),
            src(false, "iTunes", true),
        ];
        let (e, l) = build(&Inputs {
            tag_list: 0,
            session: 0,
            lib: &[],
            nodes: &[],
            days: &[],
            cats: &[],
            tags: &[],
            sources: &sources,
            all_tracks: true,
            cue_analysis: Some(3),
            banks: None,
            sections: Sections::default(),
            folded: &HashSet::new(),
            folded_lib: &HashSet::new(),
            folded_sections: &HashSet::new(),
            folded_folders: &HashSet::new(),
            day_names: &HashMap::new(),
        });
        assert_eq!(l[5], "CUE Analysis (3)");
        assert_eq!(l[7], "    All Tracks");
        let at = l
            .iter()
            .position(|x| x == "    ▾ Devices")
            .expect("the devices hang under Local Music");
        assert_eq!(
            &l[at..],
            [
                "    ▾ Devices",
                "        USB (vfat)",
                "            All tracks",
                "            ▸ Set",
                "                Inner",
                "        SD",
                "▾ Online Music",
                "▸ iTunes",
                "    All tracks",
                "    ▸ Set",
                "        Inner",
            ]
        );
        assert_eq!(
            (&e[at + 1], &e[at + 2], &e[at + 4], &e[at + 5]),
            (
                &Entry::Source(0),
                &Entry::SourceAll(0),
                &Entry::SourceList(0, 1),
                &Entry::Source(1)
            )
        );
    }
}
