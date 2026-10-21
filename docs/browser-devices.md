# The browser: devices, their folders, the pinned folders and the search

> What the Media Browser (the tree on the left) offers, and how each piece works. The spec this was built from is
> the six-phase plan agreed with the user; this document says what the app does now.

## 1. What the tree shows

From top to bottom:

| Section | What it lists | Hidden by |
|---|---|---|
| All tracks | the Collection | — |
| Tag List / Session history | the session's queue and the tracks that played | — |
| Duplicate tracks | the tracks that share title, artist and length | — |
| My Playlists | the user's own folders, playlists and smart playlists | — |
| Cue Bank | the saved cue banks | Preferences > View > Layout > Media Browser |
| History | the stored play history, one row per day | — |
| Tags | the categories and their tags | — |
| Library (read-only) | the opened library's own playlists | — |
| **Favorites** | the folders the user pinned | Preferences > View > Layout > Media Browser |
| **Downloaded** | what the plugin sources downloaded (YouTube, Deezer) | Preferences > View > Layout > Media Browser |
| **Devices** | every mounted volume | Preferences > View > Layout > Media Browser |
| Explorer | a folder of music (off by default) | Preferences > View > Layout > Media Browser |

## 2. Devices and their folders

The "Devices" section lists **every mounted volume**, not only the ones with a DJ library: a USB stick full of
music is the case that matters. A volume shows its label, its file system and its free space.

Choosing a volume opens its window (the DEVICES panel) and reads its top-level folders: the ones that hold audio,
under the same rule as the folder library (`src/library/folder.rs`). Each folder opens as a source of its own, with
its tracks and its subfolders (the subfolders are its "playlists"), and its tracks can be imported into the
Collection with the strip's button.

The tree walks a volume **once**, when the volume is read, and keeps the answer: the poll that watches the mounts
(once a second) never walks the music again.

`OPENFADER_DEVICE_ROOTS=<dir>[:<dir>…]` makes any folder count as a mounted stick (for tests and demos).

## 3. The folders the user pins (FAVORITES)

The FAV button in the toolbar pins the folder the tree has chosen (a volume's folder) and takes the pin off again.
The pinned folders are listed in the "Favorites" section, first in the tree, with a star.

A pin is the volume's **label** and the folder's path **below** it (`USB/Musica/Rock`), never the mount path: a
stick that changes letter (E: → F:) keeps its pins. The settings file holds one `pinned_folder` line per pin, the
value without quotes (the parser does not take them off):

```
pinned_folder = usb/Musica
```

A pin whose volume is not mounted is **not shown**; it comes back when the volume is plugged in again.

## 4. The downloads (DOWNLOADED)

The plugin sources keep what they download in `<data dir>/plugins/<plugin>/<source>/`. The tree has a "Downloaded"
section with one node per plugin source. The files there are real ones, so they load onto a deck, they import and
they are searched like any other song.

An opened download also lists "Not in the Collection": the files it holds that the Collection does not. The strip's
button adds all of them at once. A downloaded file is the Collection's own cloud track (its name is the remote
id), so it counts as in it, and the imports leave the tracks the Collection already holds as cloud tracks out: the
same song never gets two rows.

## 5. The media filter and the FORMAT column

The filter bar has a chip that keeps everything, only the audio or only the video (**ALL / AUDIO / VIDEO**), next
to the BPM, key and rating filters. A track with no file (a cloud track that was not downloaded yet) counts as
audio, so the video filter does not hide it.

The track table has a **FORMATO** column: a note for the audio, a film for the video and a "?" for a cloud track
with no file yet. The column sorts like the others.

## 6. The unified search

The search box looks in everything that is local at once:

* the **Collection** (every row, also the ones of the folders that are open);
* the **index of the pinned folders**;
* the **sources that are read** (the download folders).

The results are a list of their own and each row says where it comes from (the **ORIGIN** column): the Collection,
a folder, a volume. A row whose volume is not mounted says so.

The online searches of YouTube and Deezer stay in their own box: the filter **never** asks a plugin, and in a
plugin's search box the filter leaves the box alone (the plugin's results are not filtered by it).

The index of the pinned folders is read when a folder is pinned and kept in a table of its own (`folder_index`): the
search finds the tracks of a stick without walking it again. The rows are read-only and are not in the Collection.

## 7. The cloud tracks behave like any other song

A track from a plugin source joins the Collection **before it plays** (`src/app/deck/load.rs`): the history, the
"played" colour and the playlists all name it by its row, so the same track is never logged twice and never gets
two rows.

## 8. What is where

| Piece | Where |
|---|---|
| The tree's rows and their labels | `src/app/browser/sidebar.rs` |
| The external sources (volumes, downloads, folders) | `src/app/browser/sources.rs` |
| The pinned folders and their index | `src/app/browser/pinned.rs` |
| The unified search | `src/app/browser/search.rs` |
| The media filter | `src/app/browser/filter.rs` |
| The table's cells (FORMAT, ORIGIN) | `src/app/browser/table.rs` |
| The index of the pinned folders (the table) | `src/store/index.rs` |
| The volumes and their folders | `src/device/mod.rs` |
| The tree and the table | `ui/browser-tree.slint`, `ui/browser-table.slint` |
