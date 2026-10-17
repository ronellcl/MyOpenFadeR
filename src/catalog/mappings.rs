//! Controller mapping files of a repository: every `*.xml` up to [`super::DEPTH`] folders below the top folder whose root is
//! `<OpenFadeRMapping`, plus `audio-cards.json` beside the mappings (the controller audio interface cards their channels
//! 3/4 hold; see `docs/mappings.md`). Whether a mapping is valid is up to the app's mapping parser; this only reads them.

use super::below_top;
use crate::zip::Zip;
use std::path::Path;

/// A candidate mapping file (or the repository's `audio-cards.json`).
#[derive(Clone, Debug)]
pub struct File {
    /// The file name, with `.xml` (it is the name the file is installed under).
    pub name: String,
    pub text: String,
}

/// The biggest mapping file read.
const MAX_FILE: u64 = 4 << 20;
/// The repository's audio card list beside the mappings.
const AUDIO_CARDS: &str = "audio-cards.json";

/// The mapping files (and the audio card list) in `z`, in archive order.
pub fn scan(z: &mut Zip) -> Vec<File> {
    let names: Vec<String> = z
        .names()
        .filter(|n| {
            below_top(n).is_some_and(|(_, rest)| {
                rest == AUDIO_CARDS || (rest.ends_with(".xml") && rest.matches('/').count() <= super::DEPTH)
            })
        })
        .map(str::to_owned)
        .collect();
    names
        .into_iter()
        .filter_map(|n| {
            if z.size(&n)? > MAX_FILE {
                return None;
            }
            let text = String::from_utf8(z.read(&n).ok()?).ok()?;
            let name = Path::new(&n).file_name()?.to_string_lossy().into_owned();
            (name == AUDIO_CARDS || text.contains("<OpenFadeRMapping"))
                .then_some(File { name, text })
        })
        .collect()
}

/// The mapping files of the repository zip `zip` (empty when it cannot be read).
pub fn read(zip: &Path) -> Vec<File> {
    Zip::open(zip).map(|mut z| scan(&mut z)).unwrap_or_default()
}
