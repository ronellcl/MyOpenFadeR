//! `ffprobe` of a video file: size, frame rate and length of its first video stream.

use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub fps: f64,
    pub duration_ms: u32,
    /// Timestamps of the video stream's keyframes, in seconds, ascending (empty when ffprobe could not list them). The
    /// decode worker shows the previous one while it reaches the exact frame of a seek.
    pub keys: Vec<f64>,
}

impl Info {
    /// Frames in the file (the ones [`super::clock::frame_index`] may name).
    pub fn frames(&self) -> i64 {
        (self.duration_ms as f64 * self.fps / 1000.0).round() as i64
    }
}

/// Frame rates above this are decoded at this rate (the output is a preview-sized picture mixed at the UI's pace).
const MAX_FPS: f64 = 60.0;

pub fn probe(path: &Path) -> Result<Info, String> {
    let mut info = probe_stream(path)?;
    info.keys = probe_keys(path);
    Ok(info)
}

fn probe_stream(path: &Path) -> Result<Info, String> {
    let mut cmd = Command::new("ffprobe");
    cmd.args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=avg_frame_rate,r_frame_rate,duration:format=duration", "-of", "default=nw=1"])
        .arg(path);
    super::no_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("ffprobe: {e} (install ffmpeg: video needs the ffmpeg and ffprobe programs)"))?;
    if !out.status.success() {
        return Err(format!("ffprobe: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// The keyframes of the video stream, decoding only those (`-skip_frame nokey`): a seek can show the frame at or before
/// the target at once instead of waiting for ffmpeg to decode up to it. Empty when ffprobe fails: the seek still works.
fn probe_keys(path: &Path) -> Vec<f64> {
    let mut cmd = Command::new("ffprobe");
    cmd.args(["-v", "error", "-select_streams", "v:0", "-skip_frame", "nokey", "-show_entries", "frame=pts_time", "-of", "csv=p=0"])
        .arg(path);
    super::no_window(&mut cmd);
    let Ok(out) = cmd.output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    parse_keys(&String::from_utf8_lossy(&out.stdout))
}

/// The `pts_time` values of ffprobe's `csv=p=0` output ("0.000000,"), sorted, without duplicates.
fn parse_keys(text: &str) -> Vec<f64> {
    let mut keys: Vec<f64> = text
        .lines()
        .filter_map(|l| l.trim().trim_end_matches(',').parse::<f64>().ok())
        .filter(|k| k.is_finite() && *k >= 0.0)
        .collect();
    keys.sort_by(f64::total_cmp);
    keys.dedup();
    keys
}

/// `key=value` lines as ffprobe prints them. The stream's `duration` comes first, the container's after it (either may be `N/A`).
fn parse(text: &str) -> Result<Info, String> {
    let get = |key: &str| text.lines().filter_map(|l| l.strip_prefix(key)?.strip_prefix('=')).collect::<Vec<_>>();
    let rate = |s: &&str| {
        let (n, d) = s.split_once('/')?;
        let (n, d) = (n.parse::<f64>().ok()?, d.parse::<f64>().ok()?);
        (n > 0.0 && d > 0.0).then_some(n / d)
    };
    let fps = get("avg_frame_rate").iter().chain(&get("r_frame_rate")).find_map(rate).ok_or("no video stream")?;
    let secs = get("duration").iter().find_map(|s| s.parse::<f64>().ok().filter(|d| *d > 0.0)).ok_or("video has no duration")?;
    Ok(Info { fps: fps.min(MAX_FPS), duration_ms: (secs * 1000.0).round() as u32, keys: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffprobe_output() {
        let i = parse("avg_frame_rate=30000/1001\nr_frame_rate=30000/1001\nduration=N/A\nduration=12.512000\n").unwrap();
        assert!((i.fps - 29.97).abs() < 0.01);
        assert_eq!(i.duration_ms, 12512);
        assert_eq!(i.frames(), 375);
        // average rate 0/0 (some containers): the nominal rate; a fast file is capped
        let i = parse("avg_frame_rate=0/0\nr_frame_rate=120/1\nduration=2.0\nduration=2.0\n").unwrap();
        assert_eq!((i.fps, i.frames()), (60.0, 120));
    }

    #[test]
    fn no_video_stream_is_an_error() {
        assert_eq!(parse("duration=3.0\n"), Err("no video stream".into()));
        assert_eq!(parse("avg_frame_rate=25/1\nduration=N/A\nduration=N/A\n"), Err("video has no duration".into()));
    }

    #[test]
    fn keyframe_times_are_read_sorted_and_cleaned() {
        assert_eq!(parse_keys("2.000000,\n0.000000,\n6.000000,\n"), [0.0, 2.0, 6.0]);
        assert_eq!(parse_keys("4.000000,\n4.000000,\nN/A,\n-1.000000,\n"), [4.0]);
        assert!(parse_keys("").is_empty());
    }
}
