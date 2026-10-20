//! The 30 Hz loop: playheads → frames → crossfader mix → view, output window and deck rows.

use super::{
    App, DECKS, MixerState, Video, VideoDeckView, VideoState, frame_buffer, mix, mixer,
    slot::{Slot, Source, addr, picture},
};
use crate::app::deck::active;
use mix::Transition;
use openfader::video;
use slint::{ComponentHandle, Image, Model};

/// Per-deck picture weights from the crossfader and, with FOLLOW FADER, the channel faders.
fn weights(app: &App, st: &VideoState<'_>, n: usize) -> [f32; DECKS] {
    let sides = mixer::sides(app);
    let channels = app.global::<MixerState>().get_channels();
    let follow = st.get_follow_fader();
    let transition = if st.get_transition() == 1 {
        Transition::Cut
    } else {
        Transition::Fade
    };
    mix::weights(
        std::array::from_fn(|i| if i < n { sides[i] } else { 0.0 }),
        std::array::from_fn(|i| {
            if follow {
                channels.row_data(i).map_or(1.0, |c| c.fader)
            } else {
                1.0
            }
        }),
        transition,
    )
}

/// The status line: the last note, else the first failed source, else how many decks have a picture.
fn status(slots: &[Slot], note: &str) -> String {
    if !note.is_empty() {
        return note.to_owned();
    }
    let failed = slots.iter().find_map(|s| match &s.src {
        Source::Failed(e) => Some(e),
        _ => None,
    });
    match failed {
        Some(e) => e.clone(),
        None => {
            let with_video = slots
                .iter()
                .filter(|s| matches!(s.src, Source::Ready(..)))
                .count();
            format!("{with_video} of {} decks with a picture", slots.len())
        }
    }
}

impl Video {
    pub(super) fn tick(&self) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        let st = app.global::<VideoState>();
        let window_open = self.window_open();
        st.set_window_open(window_open);
        let mut slots = self.slots.borrow_mut();
        // A video file landing in a deck brings the picture up by itself: the master screen (and, in the VDJ stage, each
        // deck's own) appears without hunting for the chip, which then turns it off until the next video load.
        let mut preview = st.get_preview();
        if !preview && st.get_enabled() && self.video_landed(active()) {
            st.set_preview(true);
            preview = true;
        }
        if !st.get_visible() && !window_open && !preview {
            slots.iter_mut().for_each(|s| *s = Slot::default());
            return;
        }
        let n = active();
        let w = weights(&app, &st, n);
        for (i, slot) in slots.iter_mut().enumerate() {
            if i >= n {
                *slot = Slot::default();
                continue;
            }
            slot.follow(&self.decks[i]);
            let pos = self.decks[i]
                .with_player(|p| p.position_ms())
                .unwrap_or(0.0);
            let dur = self.decks[i].with_analysis(|_, d| d);
            slot.show(pos);
            slot.pos = if dur > 0 {
                (pos / f64::from(dur)).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
        }
        self.remix(&slots[..], w);
        let out = self.out.borrow().clone();
        if let Some(win) = self.window.borrow().as_ref().filter(|_| window_open) {
            win.set_frame(out.clone());
            win.set_overlay_on(st.get_overlay_on());
            win.set_overlay_mode(st.get_overlay_mode());
            win.set_text(st.get_text());
            win.set_overlay_image(st.get_overlay_image());
            win.set_alpha(st.get_opacity());
            win.set_size(st.get_size());
        }
        if !st.get_visible() && !preview {
            return;
        }
        // The master picture feeds the master screen under the mixer column; the deck rows feed the decks' own screens
        // (deck-panel.slint, the VDJ stage) and the VIDEO view.
        st.set_out(out);
        self.show_rows(&mut slots[..n], &w);
        if st.get_visible() {
            st.set_status(status(&slots[..n], &self.note.borrow()).into());
        }
    }

    /// Whether a deck has a video track it did not have at the last look: the load that turns VIDEO PREVIEW on by
    /// itself. Each deck's load counter is remembered, so a video loaded while the chip is off brings the picture back,
    /// and turning the chip off afterwards stays off.
    fn video_landed(&self, n: usize) -> bool {
        let mut seen = self.auto.borrow_mut();
        let mut landed = false;
        for (i, last) in seen.iter_mut().enumerate().take(n) {
            // The file is part of it: a source track lands without one and gets it when its download ends.
            let now = (
                self.decks[i].serial().unwrap_or(0),
                self.decks[i].with_track(|t| t.file.clone()).flatten(),
            );
            if *last == now {
                continue;
            }
            landed |= now.1.as_deref().is_some_and(video::is_video);
            *last = now;
        }
        landed
    }

    /// Re-blends `out` when a frame or a weight changed since the last mix.
    fn remix(&self, slots: &[Slot], w: [f32; DECKS]) {
        let key: [(usize, u32); DECKS] =
            std::array::from_fn(|i| (addr(&slots[i].frame), w[i].to_bits()));
        if key == self.mixed_from.get() {
            return;
        }
        let layers: Vec<(&[u8], f32)> = slots
            .iter()
            .zip(w)
            .filter_map(|(s, w)| Some((s.frame.as_ref()?.rgb.as_slice(), w)))
            .filter(|l| l.1 > 0.0)
            .collect();
        *self.out.borrow_mut() = if layers.is_empty() {
            Image::default() // black
        } else {
            let mut buf = frame_buffer();
            mix::blend(buf.make_mut_bytes(), &layers);
            Image::from_rgb8(buf)
        };
        self.mixed_from.set(key);
    }

    /// One `VideoDeckView` per active deck; thumbnails are rebuilt only when the deck's frame changed.
    fn show_rows(&self, slots: &mut [Slot], w: &[f32; DECKS]) {
        for (i, slot) in slots.iter_mut().enumerate() {
            if addr(&slot.frame) != slot.thumb_of {
                slot.thumb = slot
                    .frame
                    .as_ref()
                    .map_or_else(Image::default, |f| picture(&f.rgb));
                slot.thumb_of = addr(&slot.frame);
            }
            let row = VideoDeckView {
                title: if slot.serial == 0 {
                    "empty".into()
                } else {
                    slot.title.as_str().into()
                },
                info: slot.info().into(),
                thumb: slot.thumb.clone(),
                has_video: slot.frame.is_some(),
                weight: w[i],
                pos: slot.pos,
            };
            if i < self.rows.row_count() {
                self.rows.set_row_data(i, row);
            } else {
                self.rows.push(row);
            }
        }
        while self.rows.row_count() > slots.len() {
            self.rows.remove(self.rows.row_count() - 1);
        }
    }
}
