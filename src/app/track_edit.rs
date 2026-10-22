//! Edit track details dialog (`src/app/track-edit.slint`): title, artist, album, genre, label and
//! comment of the chosen track, saved to the Collection and shown at once in the track list and the
//! track panel. Only Collection tracks (`t.id` set) can be edited.

use crate::app::{App, BrowserState, TrackEditState};
use openfader::{library::Track, store};
use slint::ComponentHandle;

thread_local! {
    static EDITING: std::cell::RefCell<Option<Track>> = const { std::cell::RefCell::new(None) };
}

/// Opens the dialog for `t`. A track that is not in the Collection cannot be edited.
pub fn show(app: &App, t: &Track) {
    if t.id.is_none() {
        app.global::<BrowserState>()
            .set_status("this track is not in the Collection".into());
        return;
    }
    EDITING.with(|e| *e.borrow_mut() = Some(t.clone()));
    let state = app.global::<TrackEditState>();
    state.set_title(t.title.as_str().into());
    state.set_artist(t.artist.as_str().into());
    state.set_album(t.album.as_str().into());
    state.set_genre(t.genre.as_str().into());
    state.set_label(t.label.as_str().into());
    state.set_comment(t.comment.as_str().into());
    state.set_open(true);
}

pub fn wire(app: &App) {
    let state = app.global::<TrackEditState>();
    let weak = app.as_weak();
    state.on_save(move |title, artist, album, genre, label, comment| {
        let app = weak.unwrap();
        app.global::<TrackEditState>().set_open(false);
        let Some(t) = EDITING.with(|e| e.borrow().clone()) else { return };
        let Some(id) = t.id else {
            app.global::<BrowserState>()
                .set_status("this track is not in the Collection".into());
            return;
        };
        let key = store::id_key(id);
        let s = store::global();
        let result = (|| {
            s.set_title(&key, title.as_str())?;
            s.set_artist(&key, artist.as_str())?;
            s.set_album(&key, album.as_str())?;
            s.set_genre(&key, genre.as_str())?;
            s.set_label(&key, label.as_str())?;
            s.set_comment(&key, Some(comment.as_str()))?;
            Ok::<(), String>(())
        })();
        match result {
            Ok(()) => {
                let mut nt = t;
                nt.title = title.into();
                nt.artist = artist.into();
                nt.album = album.into();
                nt.genre = genre.into();
                nt.label = label.into();
                nt.comment = comment.into();
                super::browser::track_edited(&app, &nt);
                super::trackinfo::show(&app, &nt);
                app.global::<BrowserState>()
                    .set_status(format!("details of “{}” saved", nt.title).into());
            }
            Err(e) => app.global::<BrowserState>().set_status(format!("not saved: {e}").into()),
        }
    });
}
