//! One deck's picture source: ffprobe, the decode stream and the latest frame.

use super::frame_buffer;
use crate::app::deck::Deck;
use openfader::video::{
    self, Frame, clock,
    probe::{self, Info},
    stream::Stream,
};
use slint::Image;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{Receiver, TryRecvError, channel},
    },
};

/// What a deck's track gives the picture side.
#[derive(Default)]
pub(super) enum Source {
    /// Nothing loaded, or an audio track.
    #[default]
    None,
    /// `ffprobe` runs on a thread; its answer decides between [`Source::Ready`] and [`Source::Failed`].
    Probing(Receiver<Result<Info, String>>, PathBuf),
    Ready(Stream, Info),
    Failed(String),
}

#[derive(Default)]
pub(super) struct Slot {
    /// `Deck::track` load counter this slot was set up for (0 = none).
    pub serial: u32,
    /// The file this slot was set up for: a source track gets it when its download ends, with the serial unchanged.
    file: Option<PathBuf>,
    pub title: String,
    pub src: Source,
    pub frame: Option<Arc<Frame>>,
    /// `frame` as an image, and the frame it was made from (its address).
    pub thumb: Image,
    pub thumb_of: usize,
    /// Playhead as a fraction of the track.
    pub pos: f32,
}

/// Identity of a frame for change detection (0 = no frame).
pub(super) fn addr(f: &Option<Arc<Frame>>) -> usize {
    f.as_ref().map_or(0, |f| Arc::as_ptr(f) as usize)
}

pub(super) fn picture(rgb: &[u8]) -> Image {
    let mut buf = frame_buffer();
    buf.make_mut_bytes().copy_from_slice(rgb);
    Image::from_rgb8(buf)
}

impl Slot {
    /// Follows the deck's loads: a new track resets the slot, a video one starts probing.
    pub fn follow(&mut self, deck: &Deck) {
        let serial = deck.serial().unwrap_or(0);
        let (title, file) = deck
            .with_track(|t| (t.title.clone(), t.file.clone()))
            .unwrap_or_default();
        // The file is compared too: a track from a plugin source arrives without one and gets it when its download ends,
        // and the deck keeps the same load counter for that (a downloaded video would otherwise stay "audio only").
        if serial != self.serial || file != self.file {
            *self = Slot {
                serial,
                title,
                file: file.clone(),
                ..Slot::default()
            };
            if let Some(path) = file.filter(|p| video::is_video(p)) {
                let (tx, rx) = channel();
                let p = path.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(probe::probe(&p));
                });
                self.src = Source::Probing(rx, path);
            }
        }
        let Source::Probing(rx, path) = &self.src else {
            return;
        };
        let next = match rx.try_recv() {
            Err(TryRecvError::Empty) => return,
            Ok(Ok(info)) => Source::Ready(Stream::open(path.clone(), info), info),
            Ok(Err(e)) => Source::Failed(e),
            Err(TryRecvError::Disconnected) => Source::Failed("ffprobe stopped".into()),
        };
        self.src = next;
    }

    /// Points the stream at the frame under the playhead and picks up what it has decoded.
    pub fn show(&mut self, pos_ms: f64) {
        let Source::Ready(stream, info) = &self.src else {
            return;
        };
        stream.show(clock::frame_index(
            clock::video_time(pos_ms, 0.0),
            info.fps,
            info.frames(),
        ));
        self.frame = stream.frame();
        if let Some(e) = stream.error() {
            self.src = Source::Failed(e);
        }
    }

    pub fn info(&self) -> String {
        match &self.src {
            Source::None => "audio only — no picture".into(),
            Source::Probing(..) => "reading the video…".into(),
            Source::Ready(_, i) => format!(
                "{:.2} fps · {}:{:02}",
                i.fps,
                i.duration_ms / 60_000,
                i.duration_ms / 1000 % 60
            ),
            Source::Failed(e) => e.clone(),
        }
    }
}
