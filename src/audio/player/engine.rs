//! The one cpal stream the `Player`s share and its lifecycle.
//!
//! The first `Player::new` opens the stream, later ones take the next free slot of the mixer ([`SLOTS`]: decks A to D).
//! The callback is in [`super::mix`]. The device, rate, buffer and layout come from [`super::output`]; [`configure`]
//! moves the running stream when the Preferences change them, [`set_output`] when a controller asks for its own
//! interface. `Player::bus` gives the shared knobs. A watchdog thread notices a dead device, a failed headphone stream
//! or a stream that stopped calling back; [`recover`] retries the lost output when the deck tick pumps it.

use super::super::bus::{self, Bus};
use super::super::cue;
use super::super::route::{self, EXTERNAL, Settings};
use super::Shared;
use super::mix::{Mix, callback};
use super::output::{self, Output, choose};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FromSample, SizedSample, Stream};
use std::cell::{Cell, RefCell};
use std::rc::Weak;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool, AtomicU8, AtomicU64,
    Ordering::{AcqRel, Relaxed},
};
use std::time::{Duration, Instant};

const RUNNING: u8 = 0;
const DEVICE_GONE: u8 = 1;
const BACKEND_ERROR: u8 = 2;
const STALLED: u8 = 3;
const RECOVERING: u8 = 4;
const RETRY: Duration = Duration::from_secs(1);
const STALL: Duration = Duration::from_secs(2);
/// How long a new stream must last before its device counts as healthy again (see [`Engine::recover`]).
const HEALTHY: Duration = Duration::from_secs(10);

/// What the watchdog and the stream callbacks agree on: the state of the output, how many callbacks it made, which
/// stream those belong to, and the headphone stream's own failure latch.
#[derive(Default)]
struct Health {
    state: AtomicU8,
    callbacks: AtomicU64,
    generation: AtomicU64,
    cue_failed: parking_lot::Mutex<Option<Arc<AtomicBool>>>,
}

impl Health {
    fn arm(&self) {
        self.generation.fetch_add(1, Relaxed);
        self.state.store(RUNNING, Relaxed);
    }

    fn fail(&self, reason: u8) {
        let _ = self
            .state
            .compare_exchange(RUNNING, reason, AcqRel, Relaxed);
    }

    fn failed(&self) -> bool {
        self.state.load(Relaxed) != RUNNING
    }
}

struct Watchdog {
    callbacks: u64,
    generation: u64,
    last_callback: Instant,
    running: bool,
}

impl Watchdog {
    fn new(now: Instant) -> Self {
        Self {
            callbacks: 0,
            generation: 0,
            last_callback: now,
            running: false,
        }
    }

    /// One turn: a failed cue stream is a backend error, and a running stream that made no callback for [`STALL`] is
    /// stalled. A new generation or a callback resets the deadline, so a reconnect gets its full grace period.
    fn poll(&mut self, health: &Health, now: Instant) {
        if health
            .cue_failed
            .lock()
            .as_ref()
            .is_some_and(|failed| failed.load(Relaxed))
        {
            health.fail(BACKEND_ERROR);
        }
        let callbacks = health.callbacks.load(Relaxed);
        let running = !health.failed();
        let generation = health.generation.load(Relaxed);
        if callbacks != self.callbacks || generation != self.generation || (running && !self.running)
        {
            self.last_callback = now;
            self.callbacks = callbacks;
            self.generation = generation;
        }
        self.running = running;
        if running && now.duration_since(self.last_callback) >= STALL {
            health.fail(STALLED);
        }
    }
}

/// Polls a [`Health`] from its own thread until its engine is dropped. No UI work runs here: the deck tick pumps
/// [`recover`] instead.
fn watch(health: &Arc<Health>) {
    let health = Arc::downgrade(health);
    std::thread::spawn(move || {
        let mut watchdog = Watchdog::new(Instant::now());
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let Some(health) = health.upgrade() else {
                break;
            };
            watchdog.poll(&health, Instant::now());
        }
    });
}

/// Mixer channels one stream carries.
pub(super) const SLOTS: usize = bus::CHANNELS;
const _: () = assert!(
    EXTERNAL == SLOTS + 2,
    "External outputs = the decks, the sampler and the preview"
);

fn open<T: SizedSample + FromSample<f32> + Send + 'static>(
    dev: &cpal::Device,
    cfg: &cpal::StreamConfig,
    mut mix: Mix,
    health: &Arc<Health>,
) -> Result<Stream, cpal::BuildStreamError> {
    let (rate, ch) = (cfg.sample_rate.0, cfg.channels as usize);
    let (data, error) = (health.clone(), health.clone());
    dev.build_output_stream(
        cfg,
        move |o: &mut [T], i| {
            if data.failed() {
                o.fill(T::from_sample(0.0));
                return;
            }
            data.callbacks.fetch_add(1, Relaxed);
            callback(&mut mix, rate, ch, o, i);
        },
        move |e| {
            error.fail(match e {
                cpal::StreamError::DeviceNotAvailable => DEVICE_GONE,
                _ => BACKEND_ERROR,
            });
        },
        None,
    )
}

/// The audio thread's state for a stream on `out`.
fn mix_for(out: &Output, shared: &[Arc<Shared>; SLOTS], bus: &Arc<Bus>) -> Mix {
    let cue = out.cue.as_ref().map(|c| c.ring.clone());
    Mix::new(
        out.cfg.sample_rate.0,
        out.cfg.channels as usize,
        out.plan.clone(),
        cue,
        shared,
        bus,
    )
}

fn build(
    out: &Output,
    cfg: &cpal::StreamConfig,
    mix: Mix,
    health: &Arc<Health>,
) -> Result<Stream, String> {
    let stream = match out.fmt {
        cpal::SampleFormat::F32 => open::<f32>(&out.dev, cfg, mix, health),
        cpal::SampleFormat::I16 => open::<i16>(&out.dev, cfg, mix, health),
        cpal::SampleFormat::U16 => open::<u16>(&out.dev, cfg, mix, health),
        f => return Err(format!("unsupported sample format {f:?}")),
    };
    stream.map_err(|e| e.to_string())
}

/// A running stream on `out` rendering the channels `shared` through `bus`. A fixed buffer size the driver
/// refuses is dropped (automatic buffer) and noted in `out.status`.
fn start(
    out: &mut Output,
    shared: &[Arc<Shared>; SLOTS],
    bus: &Arc<Bus>,
    health: &Arc<Health>,
) -> Result<Stream, String> {
    for s in shared {
        s.raw.store(out.plan.external, Relaxed);
    }
    *health.cue_failed.lock() = out.cue.as_ref().map(|cue| cue.failed.clone());
    health.arm();
    let cfg = out.cfg.clone();
    let stream = match build(out, &cfg, mix_for(out, shared, bus), health) {
        Err(e) if matches!(cfg.buffer_size, cpal::BufferSize::Fixed(_)) => {
            out.cfg.buffer_size = cpal::BufferSize::Default;
            out.status.notes.push(format!(
                "the driver refused the buffer size ({e}): automatic"
            ));
            out.status.buffer = None;
            health.arm();
            build(out, &out.cfg, mix_for(out, shared, bus), health)
        }
        r => r,
    }
    .inspect_err(|_| health.state.store(RECOVERING, Relaxed))?;
    if let Err(e) = stream.play() {
        health.state.store(RECOVERING, Relaxed);
        return Err(e.to_string());
    }
    Ok(stream)
}

/// The one output stream and the channels it carries.
pub(super) struct Engine {
    pub(super) shared: [Arc<Shared>; SLOTS],
    pub(super) bus: Arc<Bus>,
    /// Next unclaimed slot. ponytail: slots are never recycled; a dropped `Player` leaves its channel silent.
    pub(super) claimed: Cell<usize>,
    /// Absent while an output is unavailable; the same channel state survives every retry.
    stream: RefCell<Option<Stream>>,
    /// The headphone device's stream, kept alive with the main one.
    cue: RefCell<Option<cue::Cue>>,
    health: Arc<Health>,
    recovery: RefCell<Option<Recovery>>,
    /// Recoveries since the output last called back. A stream can start and die at once, so this counts attempts
    /// across `install` successes; it resets only after the output ran for a while ([`HEALTHY`]).
    attempts: Cell<u32>,
    /// When the last recovery attempt opened a stream: a long healthy run clears [`Engine::attempts`].
    last_attempt_at: Cell<Option<Instant>>,
}

/// A device an output stopped playing on, waiting for its next reconnect attempt.
struct Recovery {
    name: String,
    source: &'static str,
    retry_at: Instant,
}

thread_local! {
    /// The stream every `Player` of this (UI) thread shares; gone when the last `Player` is dropped.
    pub(super) static ENGINE: RefCell<Weak<Engine>> = RefCell::new(Weak::new());
}

impl Engine {
    /// Opens the stream the settings ask for; when that fails and the settings are not the defaults, the defaults
    /// play instead and the status says what failed (a saved device that no longer works must not silence the app).
    pub(super) fn open() -> Result<Engine, String> {
        output::set_recovery(None); // a new stream starts from the settings, not from an earlier engine's retry
        let shared = [(); SLOTS].map(|_| Arc::new(Shared::new()));
        match Engine::build(&shared) {
            Ok(e) => Ok(e),
            Err(first)
                if !output::settings().same_stream(&Settings::default())
                    || output::controller().is_some() =>
            {
                // A controller's audio interface can fail while the settings themselves
                // are the defaults: drop that preference too before retrying.
                output::set_wanted(None);
                let keep = output::settings();
                output::replace_settings(Settings {
                    device: String::new(),
                    rate: 0,
                    buffer: 0,
                    mixer: route::Mixer::Internal,
                    master: 1,
                    phones: 0,
                    booth: 0,
                    external: Settings::default().external,
                    ..keep
                });
                let e = Engine::build(&shared).map_err(|again| {
                    format!("{first}; and the default output failed too: {again}")
                })?;
                output::note_error(first);
                Ok(e)
            }
            Err(e) => Err(e),
        }
    }

    fn build(shared: &[Arc<Shared>; SLOTS]) -> Result<Engine, String> {
        let mut out = choose()?;
        let bus = Arc::new(Bus::new(out.cfg.sample_rate.0, out.headphones.clone()));
        bus.apply(&output::settings());
        let health = Arc::new(Health::default());
        let stream = start(&mut out, shared, &bus, &health)?;
        watch(&health);
        output::publish(out.status.clone());
        Ok(Engine {
            shared: shared.clone(),
            bus,
            claimed: Cell::new(0),
            stream: RefCell::new(Some(stream)),
            cue: RefCell::new(out.cue.take()),
            health,
            recovery: RefCell::new(None),
            attempts: Cell::new(0),
            last_attempt_at: Cell::new(None),
        })
    }

    /// Moves the channels to the device [`choose`] now picks, keeping their state and the bus knobs; returns its name.
    /// The old stream stops first: two callbacks must never render the same channels. On an error the engine has no stream.
    fn reopen(&self) -> Result<String, String> {
        self.stop();
        output::set_recovery(None);
        self.install(choose()?)
    }

    /// Stops both streams and freezes every position clock at the last written frame: a reconnect must not extrapolate
    /// the old hardware's anchors.
    fn stop(&self) {
        self.health.state.store(RECOVERING, Relaxed);
        drop(self.stream.borrow_mut().take());
        drop(self.cue.borrow_mut().take());
        for shared in &self.shared {
            shared
                .clock
                .lock()
                .freeze(f64::from_bits(shared.pos.load(Relaxed)));
        }
    }

    /// Plays `out`, keeping the channels and bus knobs; on success it is the output until the next move.
    fn install(&self, mut out: Output) -> Result<String, String> {
        let stream = start(&mut out, &self.shared, &self.bus, &self.health)?;
        self.bus
            .route(out.cfg.sample_rate.0, out.headphones.clone());
        output::publish(out.status.clone());
        *self.stream.borrow_mut() = Some(stream);
        *self.cue.borrow_mut() = out.cue.take();
        self.recovery.borrow_mut().take();
        output::set_recovery(None);
        Ok(out.status.device)
    }

    /// Whether the stream is running and calling back (a lost output is reconnected by [`recover`]).
    pub(super) fn available(&self) -> bool {
        !self.health.failed()
    }

    /// Reconnects the output the status names when the health says it is gone. The first retry keeps the same device;
    /// after two consecutive failures it stops pinning it and lets the system default play in stereo, so a device that
    /// advertises a layout it cannot run (a controller's 4-channel interface, say) does not silence the app.
    fn recover(&self, now: Instant) {
        // A stream that has run for a while is healthy: forget earlier recoveries. Counting only short-lived
        // streams is what lets a device that starts and dies at once reach the fallback.
        if self
            .last_attempt_at
            .get()
            .is_none_or(|t| now.duration_since(t) >= HEALTHY)
        {
            self.attempts.set(0);
        }
        if self.available() {
            self.recovery.borrow_mut().take();
            output::set_recovery(None);
            return;
        }
        let mut recovery = self.recovery.borrow_mut().take().unwrap_or_else(|| {
            let status = output::status();
            let reason = match self.health.state.load(Relaxed) {
                DEVICE_GONE => "device disconnected",
                STALLED => "no audio callbacks for two seconds",
                _ => "audio backend stopped",
            };
            self.stop();
            output::note_error(format!("{}: {reason}; reconnecting", status.device));
            Recovery {
                name: status.device,
                source: status.source,
                retry_at: now,
            }
        });
        if now < recovery.retry_at {
            self.recovery.replace(Some(recovery));
            return;
        }
        let attempts = self.attempts.get().saturating_add(1);
        self.attempts.set(attempts);
        self.last_attempt_at.set(Some(now));
        // The first two retries keep the device (a cable may have been reseated). A
        // third consecutive recovery means the device or its layout does not run here:
        // stop pinning it and let the system default play. The preferences stay
        // untouched; a later CONFIGURE or controller connect can try them again.
        let fallback = attempts >= 3;
        if fallback {
            output::set_recovery(None);
            output::set_wanted(None);
            output::replace_settings(Settings {
                device: String::new(),
                rate: 0,
                buffer: 0,
                mixer: route::Mixer::Internal,
                master: 1,
                phones: 0,
                booth: 0,
                ..output::settings()
            });
            output::note_error(format!(
                "{}: the output did not start after two retries; using the system default",
                recovery.name
            ));
        } else {
            output::set_recovery(Some((recovery.name.clone(), recovery.source)));
        }
        output::set_force_stereo(fallback);
        let result = choose().and_then(|out| self.install(out));
        output::set_force_stereo(false);
        match result {
            Ok(_) => {}
            Err(e) => {
                self.health.state.store(RECOVERING, Relaxed);
                output::note_error(format!("{}: {e}; reconnecting", recovery.name));
                recovery.retry_at = now + RETRY;
                self.recovery.replace(Some(recovery));
            }
        }
    }
}

/// Pumps a pending output recovery on the UI thread; the deck tick calls this.
pub(super) fn recover() {
    if let Some(engine) = ENGINE.with_borrow(Weak::upgrade) {
        engine.recover(Instant::now());
    }
}

/// Asks for the output device named like `want` (a controller's name, matched without punctuation or case) or, with `None`, for the
/// original one (`OPENFADER_OUTPUT_DEVICE`, else the Preferences' device, else the system default); the stream moves there at once,
/// without a restart. Returns the device name now in use. An error leaves the previous device playing; `OPENFADER_OUTPUT_DEVICE` set
/// keeps its device whatever is asked.
pub fn set_output(want: Option<&str>) -> Result<String, String> {
    output::set_recovery(None);
    if std::env::var_os(bus::OUTPUT_ENV).is_some_and(|v| !v.is_empty()) {
        return choose().map(|o| o.status.device); // the env device is already the one playing
    }
    let previous = output::replace_wanted(want.map(str::to_owned));
    let Some(engine) = ENGINE.with_borrow(Weak::upgrade) else {
        return choose()
            .map(|o| o.status.device)
            .inspect_err(|_| _ = output::replace_wanted(previous)); // no stream yet: the first `Player` opens it on this device
    };
    engine.reopen().or_else(|e| {
        output::set_wanted(previous);
        match engine.reopen() {
            Ok(was) => Err(format!("{e}; staying on {was}")),
            Err(again) => Err(format!("{e}; and the previous device failed too: {again}")),
        }
    })
}

/// Applies the Preferences' audio settings ([`Settings`]): the live knobs at once ([`Bus::apply`]), the device, rate, buffer
/// and channel layout by moving the running stream (decks keep their state; there is a short gap). With no stream yet the first
/// `Player` opens it with these. An error leaves the previous stream playing (what failed is in `status().error`).
pub fn configure(s: Settings) -> Result<(), String> {
    output::set_recovery(None);
    let old = output::replace_settings(s.clone());
    let Some(engine) = ENGINE.with_borrow(Weak::upgrade) else {
        return Ok(());
    };
    engine.bus.apply(&s);
    if s.same_stream(&old) {
        return Ok(());
    }
    engine.reopen().map(drop).or_else(|e| {
        output::replace_settings(old);
        let back = engine.reopen();
        output::note_error(e.clone());
        match back {
            Ok(was) => Err(format!("{e}; staying on {was}")),
            Err(again) => Err(format!("{e}; and the previous output failed too: {again}")),
        }
    })
}

/// The bus of the running stream (`None` before the first `Player` opened it): the knobs, the preview player
/// (`Bus::preview`) and the recorder tap, for code that has no `Player` at hand.
pub fn live_bus() -> Option<Arc<Bus>> {
    ENGINE.with_borrow(Weak::upgrade).map(|e| e.bus.clone())
}

#[cfg(test)]
mod tests {
    use super::super::Player;
    use super::*;

    #[test]
    fn a_failure_keeps_the_first_reason_until_the_stream_is_armed_again() {
        let health = Health::default();
        assert!(!health.failed(), "a fresh health is running");
        health.arm();
        health.fail(DEVICE_GONE);
        health.fail(BACKEND_ERROR);
        assert!(health.failed());
        assert_eq!(
            health.state.load(Relaxed),
            DEVICE_GONE,
            "an error storm does not overwrite the first reason"
        );
        health.arm();
        assert!(!health.failed());
        assert_eq!(health.generation.load(Relaxed), 2, "arming starts a new run");
    }

    #[test]
    fn a_failed_cue_stream_fails_the_health() {
        let (health, now) = (Health::default(), Instant::now());
        let failed = Arc::new(AtomicBool::new(false));
        *health.cue_failed.lock() = Some(failed.clone());
        let mut watchdog = Watchdog::new(now);
        watchdog.poll(&health, now);
        assert!(!health.failed(), "the cue stream is still alive");
        failed.store(true, Relaxed);
        watchdog.poll(&health, now + Duration::from_millis(100));
        assert_eq!(health.state.load(Relaxed), BACKEND_ERROR);
        *health.cue_failed.lock() = None;
        health.arm();
        watchdog.poll(&health, now + RETRY * 5);
        assert!(!health.failed());
    }

    #[test]
    fn a_lost_stream_reports_the_written_position_and_stop_freezes_the_clock() {
        use super::super::clock::{Anchor, heard_frames};
        use std::rc::Rc;

        let shared = [(); SLOTS].map(|_| Arc::new(Shared::new()));
        let t0 = Instant::now();
        shared[0].playing.store(true, Relaxed);
        shared[0].rate.store(1.0f64.to_bits(), Relaxed);
        shared[0].pos.store(8000f64.to_bits(), Relaxed);
        shared[0].clock.lock().push(
            Anchor {
                at: t0 - STALL,
                pos: 7500.0,
                latency: Duration::from_millis(40),
                fps: 44100.0,
                lp: None,
            },
            true,
            true,
        );
        let engine = Rc::new(Engine {
            shared,
            bus: Arc::new(Bus::new(44100, bus::Headphones::Split)),
            claimed: Cell::new(0),
            stream: RefCell::new(None),
            cue: RefCell::new(None),
            health: Arc::new(Health::default()),
            recovery: RefCell::new(None),
            attempts: Cell::new(0),
            last_attempt_at: Cell::new(None),
        });
        let player = Player {
            shared: engine.shared[0].clone(),
            engine: engine.clone(),
            src_rate: 44100,
            frames: 44100,
            has_stems: false,
        };
        engine.health.fail(BACKEND_ERROR);
        assert_eq!(player.position_ms(), 8000.0 * 1000.0 / 44100.0);
        assert!(!player.settled(), "a dead stream hears nothing");
        engine.stop();
        let sh = &engine.shared[0];
        assert!(!engine.available());
        assert_eq!(
            heard_frames(&sh.clock.lock(), t0 + STALL * 10, 44100.0),
            8000.0,
            "the frozen clock holds at the last written frame"
        );
    }

    #[test]
    fn missing_callbacks_trip_the_watchdog_but_a_new_stream_gets_a_fresh_deadline() {
        let (health, t0) = (Health::default(), Instant::now());
        let mut watchdog = Watchdog::new(t0);
        health.arm();
        watchdog.poll(&health, t0);
        health.callbacks.fetch_add(1, Relaxed);
        watchdog.poll(&health, t0 + STALL);
        watchdog.poll(&health, t0 + STALL * 2 - Duration::from_millis(1));
        assert!(!health.failed(), "the callback moved the deadline");
        watchdog.poll(&health, t0 + STALL * 2);
        assert_eq!(health.state.load(Relaxed), STALLED);
        // A reconnect can finish entirely between two watchdog turns: its generation resets the deadline.
        health.state.store(RECOVERING, Relaxed);
        health.arm();
        watchdog.poll(&health, t0 + STALL * 10);
        assert!(!health.failed());
        watchdog.poll(&health, t0 + STALL * 11);
        assert_eq!(health.state.load(Relaxed), STALLED);
    }
}
