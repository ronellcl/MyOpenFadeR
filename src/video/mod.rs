//! VIDEO: a deck can hold a video file (mp4 / mov / m4v) as its track. The audio goes through
//! the normal [`crate::audio::decode`] path; the pictures are decoded here, off the UI thread, by an `ffmpeg` child
//! process that streams raw RGB frames over a pipe (no FFmpeg library is linked: the binaries `ffmpeg` / `ffprobe`
//! must be on `PATH`). Each deck's picture follows its audible playhead, so tempo, pause, seek, loops, reverse and
//! scratching need no special cases: the picture is whatever frame the playhead is on.
//!
//! The panel has a master output that can go full screen, a VIDEO CROSSFADER that can be synced to the audio one,
//! and brightness that follows the channel fader. Implemented: the crossfader transition
//! ([`mix::Transition`]: FADE or CUT), the channel-fader brightness, and text / image overlays (drawn by the UI on
//! top of the mixed picture). Camera, transitions beyond fade/cut and linked videos are not implemented.
//!
//! Children: [`clock`] (playhead → frame index), [`probe`] (`ffprobe` parse), [`stream`] (the decode worker: seek, read
//! ahead, a short cache for loops and scratching), [`mix`] (crossfader weights and the blend), [`audio`] (the sound
//! of a video through `ffmpeg`, when the native decoder lacks its codec).

pub mod audio;
pub mod clock;
pub mod mix;
pub mod probe;
pub mod stream;

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

/// A console program (`ffmpeg` / `ffprobe`) launched from the window would flash a terminal on Windows: every video
/// load would open one for each child. The child keeps its pipes and its output; only the console is dropped.
///
/// `CREATE_NO_WINDOW` alone is not enough on Windows 11 when Windows Terminal is the system's default terminal: the
/// child still gets a `conhost`, and a terminal window pops up for it. `DETACHED_PROCESS` gives it no console at all
/// (checked: no `conhost` child), which is what keeps the terminal quiet; the flag is kept too for older Windows.
pub fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x080_0000 | 0x0000_0008); // CREATE_NO_WINDOW | DETACHED_PROCESS
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Output picture size: every frame is scaled to fit (letterboxed, never distorted) into this.
pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 360;
pub const BYTES: usize = WIDTH * HEIGHT * 3;

/// Container extensions that carry a video track.
pub const EXTS: [&str; 3] = ["mp4", "mov", "m4v"];

pub fn is_video(path: &Path) -> bool {
    crate::paths::has_extension(path, &EXTS)
}

/// One frame of `path` at `position_ms`, RGB24, [`WIDTH`] × [`HEIGHT`] (letterboxed, never distorted).
/// A fast keyframe seek: good enough for a still preview, not frame-accurate.
pub fn frame_at(path: &Path, position_ms: u32) -> Result<Frame, String> {
    let ss = position_ms as f64 / 1000.0;
    let vf = format!("scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2:black");
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner", "-loglevel", "error", "-nostdin",
        "-ss", &format!("{ss:.4}"),
        "-i",
    ])
    .arg(path)
    .args(["-frames:v", "1", "-vf", &vf, "-pix_fmt", "rgb24", "-f", "rawvideo", "-"])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    no_window(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("ffmpeg: {e} (install ffmpeg to decode video frames)"))?;
    let mut out = child.stdout.take().ok_or("ffmpeg: no stdout")?;
    let mut rgb = vec![0; BYTES];
    out.read_exact(&mut rgb).map_err(|e| format!("ffmpeg: {e}"))?;
    drop(out);
    let status = child.wait().map_err(|e| format!("ffmpeg: {e}"))?;
    if !status.success() {
        return Err(format!("ffmpeg: exit {}", status));
    }
    Ok(Frame { rgb })
}

/// One decoded picture, RGB24, [`WIDTH`] × [`HEIGHT`].
pub struct Frame {
    pub rgb: Vec<u8>,
}
