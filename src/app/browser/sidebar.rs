//! The playlist tree beside the track table: what every sidebar row is ([`Entry`]) and the label it shows.
//!
//! Top to bottom: All tracks, Tag List, Session history, Duplicate tracks (the first three rows keep their
//! numbers, `BrowserState.tag-row` is row 1), then **My Playlists** (our folders / playlists / smart playlists from
//! the store), **History** (one entry per day the store logged), **Tags** (categories, their tags) and **Library**
//! (the opened source's own playlists, read-only; its "Histories" folder is among them).

use openfader::{
    library::{Playlist, Track},
    plugin::device::MAX_BANKS,
    store::{HotCueBank, Tag, TagGroup, Node, NodeKind},
};
use std::collections::{HashMap, HashSet};

/// Sidebar row of the session Tag List (row 0 is "All tracks").
pub const TAG_ROW: usize = 1;
/// Sidebar row of the Session history, right after the Tag List.
pub const HISTORY_ROW: usize = 2;

#[derive(Clone, Debug, PartialEq)]
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
    DownloadedRoot,
    /// "Música Local" heading above the system folders (Music, Videos, Desktop).
    LocalMusicRoot,
    /// A system folder of Windows: Music, Videos, Desktop.
    SystemFolder(usize),
    /// "Música en Línea" heading above the cloud plugins.
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
fn playlist_label(base: usize, p: &Playlist) -> String {
    indent(
        base + p.depth,
        &format!("{}{}", if p.is_folder { "▸ " } else { "" }, p.name),
    )
}

/// Every row's entry and label, in display order.
pub fn build(i: &Inputs) -> (Vec<Entry>, Vec<String>) {
    let mut rows = vec![
        (Entry::All, "All tracks".to_owned()),
        (Entry::Tags, format!("Tag List ({})", i.tag_list)),
        (Entry::Session, format!("Session history ({})", i.session)),
        (Entry::Dupes, "Duplicate tracks".to_owned()),
        (Entry::UserRoot, "▸ My Playlists".to_owned()),
    ];
    if let Some(n) = i.cue_analysis {
        rows.insert(4, (Entry::CueAnalysis, format!("CUE Analysis ({n})")));
    }
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
    if let Some(banks) = i.banks {
        rows.push((Entry::BankRoot, "▸ Cue Bank".to_owned()));
        rows.extend(banks.iter().map(|b| {
            let label = format!("{} ({}/{MAX_BANKS})", b.name, b.slots.len());
            (Entry::Bank(b.id), indent(1, &label))
        }));
    }
    rows.push((Entry::HistoryRoot, "▸ History".to_owned()));
    rows.extend(
        i.days
            .iter()
            .map(|(day, n)| (Entry::Day(day.clone()), indent(1, &format!("{day} ({n})")))),
    );
    rows.push((Entry::TagsRoot, "▸ Tags".to_owned()));
    for c in i.cats {
        rows.push((Entry::Category(c.id), indent(1, &format!("▸ {}", c.name))));
        rows.extend(
            i.tags
                .iter()
                .filter(|t| t.category == c.id)
                .map(|t| (Entry::Tag(t.id), indent(2, &t.name))),
        );
    }
    rows.push((Entry::LibRoot, "▸ Library (read-only)".to_owned()));
    // The library's own playlists: a list with lists under it folds them (the plugin sources come this way).
    let mut at = 0;
    while at < i.lib.len() {
        let p = &i.lib[at];
        let children = i.lib[at + 1..]
            .iter()
            .take_while(|c| c.depth > p.depth)
            .count();
        rows.push((Entry::Lib(at), playlist_label(1, p)));
        if children > 0 && !i.folded_lib.contains(&at) {
            rows.extend(
                i.lib[at + 1..at + 1 + children]
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (Entry::Lib(at + 1 + k), playlist_label(1, c))),
            );
        }
        at += 1 + children;
    }
    // ── Música Local: system folders (Music, Videos, Desktop) ──────────────────
    if i.sections.local_music {
        rows.push((Entry::LocalMusicRoot, "▸ Música Local".to_owned()));
        let system: Vec<usize> = i
            .sources
            .iter()
            .enumerate()
            .filter(|(_, s)| s.system_folder)
            .map(|(n, _)| n)
            .collect();
        for n in &system {
            rows.push((Entry::SystemFolder(*n), indent(1, &format!("▸ {}", i.sources[*n].name))));
        }
        // Discos: devices and local volumes under their own heading.
        let disks: Vec<usize> = i
            .sources
            .iter()
            .enumerate()
            .filter(|(_, s)| s.device || s.local)
            .map(|(n, _)| n)
            .collect();
        if !disks.is_empty() {
            rows.push((Entry::DevicesRoot, indent(1, "▸ Discos")));
            for n in &disks {
                source_rows(&mut rows, i, *n, 2);
            }
        }
    }
    // ── Música en Línea: cloud plugins (Deezer, YouTube, SoundCloud) with sub-entries ─
    if i.sections.online_music {
        rows.push((Entry::OnlineMusicRoot, "▸ Música en Línea".to_owned()));
        let online: Vec<usize> = i
            .sources
            .iter()
            .enumerate()
            .filter(|(_, s)| s.online_plugin)
            .map(|(n, _)| n)
            .collect();
        for n in &online {
            rows.push((Entry::OnlinePlugin(*n), indent(1, &format!("▸ {}", i.sources[*n].name))));
            // Sub-entries: Search and Downloads (and SoundCloud's example playlists).
            rows.push((Entry::OnlineSearch(*n, 0), indent(2, "Búsqueda")));
            rows.push((Entry::OnlineDownloads(*n, 0), indent(2, "Descargas")));
            // SoundCloud's example playlists (Major Lazer, Diplo …) come from the library plugin's lists.
            if let Some(s) = i.sources.get(*n) {
                if s.playlists.iter().any(|p| !p.is_folder) {
                    for (j, p) in s.playlists.iter().enumerate() {
                        if !p.is_folder {
                            rows.push((Entry::OnlinePlaylist(*n, j), indent(2, &p.name)));
                        }
                    }
                }
            }
        }
    }
    // ── Favorites (carpetas fijadas) ─────────────────────────────────────────────
    let pinned: Vec<usize> = i
        .sources
        .iter()
        .enumerate()
        .filter(|(_, s)| s.pinned)
        .map(|(n, _)| n)
        .collect();
    if i.sections.favorites {
        rows.push((Entry::FavoritesRoot, "▸ Carpetas Favoritas".to_owned()));
        for n in &pinned {
            rows.push((Entry::Pinned(*n), indent(1, &format!("★ {}", i.sources[*n].name))));
        }
    }
    // ── Downloaded (lo que los plugins bajaron) ──────────────────────────────────
    let downloaded: Vec<usize> = i
        .sources
        .iter()
        .enumerate()
        .filter(|(_, s)| s.downloaded)
        .map(|(n, _)| n)
        .collect();
    if i.sections.downloaded {
        rows.push((Entry::DownloadedRoot, "▸ Downloaded".to_owned()));
        for n in downloaded {
            source_rows(&mut rows, i, n, 1);
        }
    }
    // ── Other sources (Explorer, iTunes, xml plugins) on their own ───────────────
    for (n, s) in i.sources.iter().enumerate() {
        if s.downloaded || s.pinned || s.system_folder || s.online_plugin || s.device || s.local {
            continue; // already listed above
        }
        source_rows(&mut rows, i, n, 0);
    }
    rows.into_iter().unzip()
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
    rows.extend(s.folders.iter().enumerate().flat_map(|(j, f)| {
        let mut r = vec![(
            Entry::DeviceFolder(n, j),
            indent(d + 1, &format!("▸ {}", f.name)),
        )];
        r.extend(
            f.playlists
                .iter()
                .enumerate()
                .map(|(k, p)| (Entry::DeviceFolderList(n, j, k), playlist_label(d + 2, p))),
        );
        r
    }));
    if s.open {
        rows.push((Entry::SourceAll(n), indent(d + 1, "All tracks")));
        rows.extend(
            s.playlists
                .iter()
                .enumerate()
                .map(|(j, p)| (Entry::SourceList(n, j), playlist_label(d + 1, p))),
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
        });
        assert_eq!(entries[TAG_ROW], Entry::Tags);
        assert_eq!(entries[HISTORY_ROW], Entry::Session);
        assert_eq!(
            labels,
            [
                "All tracks",
                "Tag List (2)",
                "Session history (4)",
                "Duplicate tracks",
                "▸ My Playlists",
                "    ▸ Set",
                "        Warmup",
                "    House (smart)",
                "▸ Cue Bank",
                "    Peak (0/8)",
                "▸ History",
                "    2026-10-13 (3)",
                "    2026-10-12 (1)",
                "▸ Tags",
                "    ▸ Genre",
                "        Deep",
                "    ▸ Mood",
                "        Dark",
                "▸ Library (read-only)",
                "    ▸ Folder",
                "        Inner",
                "▸ Favorites",
                "▸ Downloaded",
                "▸ Devices",
            ]
        );
        assert_eq!(entries.len(), labels.len());
        assert_eq!(entries[6], Entry::User(2));
        assert_eq!(entries[9], Entry::Bank(7));
        assert_eq!(entries[11], Entry::Day("2026-10-13".into()));
        assert_eq!(entries[15], Entry::Tag(9));
        assert_eq!(entries[20], Entry::Lib(1));
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
        });
        assert_eq!(
            e,
            [
                Entry::All,
                Entry::Tags,
                Entry::Session,
                Entry::Dupes,
                Entry::UserRoot,
                Entry::HistoryRoot,
                Entry::TagsRoot,
                Entry::LibRoot,
                // The sections of the tree are always there, empty or not: the user sees them and can hide them.
                Entry::FavoritesRoot,
                Entry::DownloadedRoot,
                Entry::DevicesRoot,
            ]
        );
    }

    /// The downloads have their own section, before the devices, and an opened one lists its tracks and what the
    /// Collection does not have.
    #[test]
    fn the_downloads_have_their_own_section_with_their_tracks() {
        let src = |name: &str, device, downloaded, open, missing| SourceRows {
            device,
            local: false,
            downloaded,
            pinned: false,
            name: name.into(),
            open,
            playlists: Vec::new(),
            missing: vec![Track::default(); missing],
            folders: Vec::new(),
        };
        let sources = [
            src("YouTube", false, true, true, 3),
            src("Explorer", false, false, false, 0),
            src("USB", true, false, false, 0),
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
        });
        let at = labels
            .iter()
            .position(|l| l == "▸ Downloaded")
            .expect("the downloads have their own section");
        assert_eq!(entries[at], Entry::DownloadedRoot);
        // The section comes before the devices, and the download lists its tracks and what the Collection lacks.
        assert_eq!(entries[at + 1], Entry::Source(0));
        assert_eq!(labels[at + 1], "    ▸ YouTube");
        assert_eq!(entries[at + 2], Entry::SourceAll(0));
        assert_eq!(labels[at + 2], "        All tracks");
        assert_eq!(entries[at + 3], Entry::SourceMissing(0));
        assert_eq!(labels[at + 3], "        Not in the Collection");
        // The other sources keep their place.
        let devices = labels
            .iter()
            .position(|l| l == "▸ Devices")
            .expect("the devices have their own section");
        assert!(devices > at);
        assert!(!labels.iter().any(|l| l == "    ▸ Explorer"));
    }

    /// A device lists its top-level folders (the ones with audio) under it, each with its own subfolders.
    #[test]
    fn a_device_lists_its_own_folders() {
        let sources = [SourceRows {
            device: true,
            local: false,
            downloaded: false,
            pinned: false,
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
        });
        let at = labels
            .iter()
            .position(|l| l == "▸ Devices")
            .expect("the devices have their own section");
        assert_eq!(entries[at + 1], Entry::Source(0));
        assert_eq!(labels[at + 1], "    USB (vfat)");
        // The folder and the folder inside it.
        assert_eq!(entries[at + 2], Entry::DeviceFolder(0, 0));
        assert_eq!(labels[at + 2], "        ▸ Musica");
        assert_eq!(entries[at + 3], Entry::DeviceFolderList(0, 0, 0));
        assert_eq!(labels[at + 3], "            Rock");
    }

    /// The local disk (the system volume) hangs under the "Devices" heading with its own folders, read-only.
    #[test]
    fn the_local_disk_hangs_under_the_devices_with_its_folders() {
        let sources = [SourceRows {
            device: false,
            local: true,
            downloaded: false,
            pinned: false,
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
        });
        let at = labels
            .iter()
            .position(|l| l == "▸ Devices")
            .expect("the devices have their own section");
        assert_eq!(entries[at + 1], Entry::Source(0));
        assert_eq!(labels[at + 1], "    ▸ C: (OS)");
        // Its folder, under it.
        assert_eq!(entries[at + 2], Entry::DeviceFolder(0, 0));
        assert_eq!(labels[at + 2], "        ▸ Users");
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
        });
        assert_eq!(l[4], "CUE Analysis (3)");
        assert_eq!(l[6], "    All Tracks");
        let at = l.iter().position(|x| x == "▸ Devices").unwrap();
        assert_eq!(
            &l[at..],
            [
                "▸ Devices",
                "    USB (vfat)",
                "        All tracks",
                "        ▸ Set",
                "            Inner",
                "    SD",
                "▸ iTunes",
                "    All tracks",
                "    ▸ Set",
                "        Inner"
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
