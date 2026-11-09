//! "Preparar Stems" over a favorite folder or a user playlist: separates every track once, in the background, so a
//! session only reads the cache. The dialog estimates the disk it needs, offers to continue until the disk is full,
//! shows progress and can be cancelled. Videos are prepared too: the no-stems rule for videos applies only to live
//! decks.

use super::{App, BrowserState, View};
use super::stems_queue::Queue;
use crate::app::prefs;
use openfader::{
    audio, device,
    library::{Library, Track},
    stems::{self, Quality},
    video,
};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// Four stereo i16 stems at 44.1 kHz.
const BYTES_PER_SEC: u64 = 705_600;
/// Duration assumed for a track whose length is unknown.
const ASSUMED_SECS: u64 = 300;
/// Leave this much free after the last prepared track.
const MARGIN: u64 = 64 * 1024 * 1024;
/// How often the UI reads the worker state.
const POLL: Duration = Duration::from_millis(300);

struct Plan {
    title: String,
    tracks: Vec<Track>,
    estimate: u64,
    free: Option<u64>,
}

struct State {
    total: usize,
    done: AtomicU64,
    skipped: AtomicU64,
    failed: AtomicU64,
    /// Permille of the track being separated.
    part: AtomicU64,
    current: Mutex<String>,
    cancel: AtomicBool,
    /// Paused after the current track (the worker waits before starting the next one).
    paused: AtomicBool,
    finished: AtomicBool,
    stopped_for_space: AtomicBool,
    error: Mutex<Option<String>>,
    /// When the batch started, for the remaining-time estimate.
    started: Instant,
}

impl State {
    fn new(total: usize) -> Self {
        Self {
            total,
            done: AtomicU64::new(0),
            skipped: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            part: AtomicU64::new(0),
            current: Mutex::new(String::new()),
            cancel: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            stopped_for_space: AtomicBool::new(false),
            error: Mutex::new(None),
            started: Instant::now(),
        }
    }
}

struct Running {
    state: Arc<State>,
    queue: Arc<Mutex<Queue>>,
    _timer: Rc<Timer>,
}

thread_local! {
    static PENDING: RefCell<Option<Plan>> = const { RefCell::new(None) };
    static RUNNING: RefCell<Option<Running>> = const { RefCell::new(None) };
    static RESUME: RefCell<Option<Queue>> = const { RefCell::new(None) };
}

fn bytes(t: &Track) -> u64 {
    let secs = if t.length_s > 0 {
        t.length_s as u64
    } else {
        ASSUMED_SECS
    };
    // A video is copied to the local media cache too, unless that copy is already there.
    let media = t
        .file
        .as_deref()
        .filter(|f| video::is_video(f) && video::local::cached(f).is_none())
        .and_then(|f| std::fs::metadata(f).ok())
        .map_or(0, |m| m.len());
    secs.saturating_mul(BYTES_PER_SEC).saturating_add(media)
}

fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

fn cache_dir() -> Option<std::path::PathBuf> {
    let dir = stems::cache::dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn free_space() -> Option<u64> {
    let dir = cache_dir()?;
    device::forget(&dir);
    device::space(&dir).map(|(_, free)| free)
}

/// Opens the confirmation for `title`'s `tracks`. The per-track cache check and the size/estimate
/// pass run on a worker: doing them here would block the UI on a slow USB disk.
pub(super) fn prepare(app: &App, title: String, tracks: Vec<Track>) {
    let weak = app.as_weak();
    app.global::<BrowserState>().set_status(
        format!("{title}: contando pistas, espacio y duración…").into(),
    );
    std::thread::spawn(move || {
        let mut cached = 0;
        let mut estimate: u64 = 0;
        let mut audio_s: u64 = 0;
        let mut with_file = 0;
        let mut todo = Vec::new();
        for t in tracks {
            let Some(file) = t.file.clone() else { continue };
            with_file += 1;
            let secs = if t.length_s > 0 {
                t.length_s as u64
            } else {
                ASSUMED_SECS
            };
            if stems::cache::has(&file, Quality::Quality) {
                cached += 1;
                super::stems_index::mark_ready(&file);
            } else {
                estimate += bytes(&t);
                audio_s = audio_s.saturating_add(secs);
                todo.push(t);
            }
        }
        let free = free_space();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(app) = weak.upgrade() else { return };
            show_plan(&app, title, cached, estimate, audio_s, with_file, todo, free);
        });
    });
}

/// Shows the confirmation once the worker has measured the collection.
fn show_plan(
    app: &App,
    title: String,
    cached: usize,
    estimate: u64,
    audio_s: u64,
    with_file: usize,
    todo: Vec<Track>,
    free: Option<u64>,
) {
    eprintln!(
        "stems: “{title}”: {} to prepare, {} cached, ~{} GB, {}",
        todo.len(),
        cached,
        estimate as f64 / 1e9,
        eta(audio_s)
    );
    if cached > 0 {
        // The cached files may belong to a list loaded after the first index pass (a Deezer source,
        // a playlist opened later); redraw the table now that they are marked.
        if let Some(view) = super::view() {
            view.borrow_mut().refresh(app);
        }
    }
    if todo.is_empty() {
        let msg = if with_file == 0 {
            format!("“{title}” no tiene pistas con archivo para preparar.")
        } else {
            format!("Todas las pistas de “{title}” ya tienen sus stems.")
        };
        app.global::<BrowserState>().set_status(msg.into());
        return;
    }
    let enough = free.is_none_or(|f| f >= estimate + MARGIN);
    let s = app.global::<BrowserState>();
    // Keep a summary in the status bar too: it stays visible even if the popup is dismissed.
    s.set_status(
        format!(
            "{title}: {} por preparar · ~{} · {}",
            todo.len(),
            gb(estimate),
            eta(audio_s)
        )
        .into(),
    );
    s.set_stems_open(true);
    s.set_stems_minimized(false);
    s.set_stems_busy(false);
    s.set_stems_progress(-1.0);
    s.set_stems_title(format!("Preparar stems · {title}").into());
    s.set_stems_message(format!("Se separarán {} canciones.", todo.len()).into());
    let mut detail = if cached > 0 {
        format!("{cached} ya están listas y se omitirán. ")
    } else {
        String::new()
    };
    detail += &format!(
        "Necesitas ~{} y tienes {}. Duración estimada: {}.",
        gb(estimate),
        free.map_or_else(|| "espacio libre desconocido".to_owned(), gb),
        eta(audio_s)
    );
    s.set_stems_detail(detail.into());
    s.set_stems_warning(
        "Edita tags, nombre o ubicación ahora: los stems se guardan por ruta, tamaño y fecha."
            .into(),
    );
    if enough {
        s.set_stems_primary("Preparar stems".into());
        s.set_stems_secondary("Cancelar".into());
    } else {
        s.set_stems_message(
            format!(
                "No hay espacio suficiente: faltan {}.",
                gb(estimate + MARGIN - free.unwrap_or(0))
            )
            .into(),
        );
        s.set_stems_detail(
            "Puedes continuar: se prepararán solo las canciones que quepan.".into(),
        );
        s.set_stems_primary("Continuar de todos modos".into());
        s.set_stems_secondary("Ok, haré espacio".into());
    }
    PENDING.with(|p| {
        *p.borrow_mut() = Some(Plan {
            title,
            tracks: todo,
            estimate,
            free,
        });
    });
}

/// Rough wall-clock estimate for a QUALITY batch (four threads). The dialog labels it as an
/// estimate; the cached files are measured by the batch itself.
fn eta(audio_s: u64) -> String {
    eta_secs(audio_s.saturating_mul(2))
}

fn eta_secs(secs: u64) -> String {
    let mins = secs.div_ceil(60).max(1);
    if mins < 60 {
        format!("~{mins} min")
    } else {
        format!("~{} h {} min", mins / 60, mins % 60)
    }
}

/// The primary button: start the plan, or close the summary when there is none.
pub(super) fn primary(app: &App) {
    let Some(plan) = PENDING.with(|p| p.borrow_mut().take()) else {
        return close(app);
    };
    start(app, plan);
}

/// The secondary button: cancel a running batch, or dismiss the confirmation.
pub(super) fn secondary(app: &App) {
    if app.global::<BrowserState>().get_stems_busy() {
        RUNNING.with(|r| {
            if let Some(run) = r.borrow().as_ref() {
                run.state.cancel.store(true, Ordering::Relaxed);
            }
        });
        app.global::<BrowserState>()
            .set_stems_detail("Cancelando…".into());
    } else {
        PENDING.with(|p| *p.borrow_mut() = None);
        close(app);
    }
}

fn close(app: &App) {
    let s = app.global::<BrowserState>();
    s.set_stems_open(false);
    s.set_stems_minimized(false);
    s.set_stems_done(0);
    s.set_stems_total(0);
    s.set_stems_busy(false);
    s.set_stems_progress(-1.0);
    s.set_stems_title("".into());
    s.set_stems_message("".into());
    s.set_stems_detail("".into());
    s.set_stems_warning("".into());
    s.set_stems_primary("".into());
    s.set_stems_secondary("".into());
    RUNNING.with(|r| *r.borrow_mut() = None);
}

/// Hides the confirmation/progress window while the worker keeps separating.
pub(super) fn minimize(app: &App) {
    let s = app.global::<BrowserState>();
    s.set_stems_open(false);
    s.set_stems_minimized(true);
}

/// Brings the hidden stems window back (from the bottom status bar).
pub(super) fn reopen(app: &App) {
    let s = app.global::<BrowserState>();
    s.set_stems_open(true);
    s.set_stems_minimized(false);
}

fn start(app: &App, plan: Plan) {
    eprintln!(
        "stems: batch “{}”: {} tracks",
        plan.title,
        plan.tracks.len()
    );
    let s = app.global::<BrowserState>();
    s.set_stems_open(true);
    s.set_stems_minimized(false);
    s.set_stems_done(0);
    s.set_stems_total(plan.tracks.len() as i32);
    s.set_stems_busy(true);
    s.set_stems_paused(false);
    s.set_stems_progress(0.0);
    s.set_stems_title(format!("Preparando stems · {}", plan.title).into());
    s.set_stems_message("0 preparadas · 0 omitidas · 0 fallidas".into());
    s.set_stems_detail("".into());
    s.set_stems_warning("".into());
    s.set_stems_primary("".into());
    s.set_stems_secondary("Cancelar".into());
    let queue = Arc::new(Mutex::new(Queue::new(&plan.title, &plan.tracks)));
    if let Ok(q) = queue.lock() {
        super::stems_queue::save(&q);
    }
    let state = Arc::new(State::new(plan.tracks.len()));
    let worker = state.clone();
    let worker_queue = queue.clone();
    let tracks = plan.tracks;
    let multithread = prefs::with(|p| p.extensions.stems_multithread);
    std::thread::spawn(move || work(tracks, multithread, worker, worker_queue));
    let weak = app.as_weak();
    let polled = state.clone();
    let timer = Rc::new(Timer::default());
    let stop = timer.clone();
    timer.start(TimerMode::Repeated, POLL, move || {
        let Some(app) = weak.upgrade() else { return };
        update(&app, &polled);
        if polled.finished.load(Ordering::Relaxed) {
            stop.stop();
        }
    });
    RUNNING.with(|r| *r.borrow_mut() = Some(Running { state, queue, _timer: timer }));
    let _ = plan.estimate;
    let _ = plan.free;
}

fn update(app: &App, state: &State) {
    let s = app.global::<BrowserState>();
    let (done, skipped, failed) = (
        state.done.load(Ordering::Relaxed),
        state.skipped.load(Ordering::Relaxed),
        state.failed.load(Ordering::Relaxed),
    );
    s.set_stems_done((done + skipped + failed) as i32);
    s.set_stems_total(state.total as i32);
    if state.finished.load(Ordering::Relaxed) {
        let cancelled = state.cancel.load(Ordering::Relaxed);
        let stopped = state.stopped_for_space.load(Ordering::Relaxed);
        let title = if cancelled {
            "Preparación cancelada"
        } else if stopped {
            "Sin espacio para más stems"
        } else {
            "Stems preparados"
        };
        let mut message = format!("{done} preparadas · {skipped} omitidas · {failed} fallidas");
        if stopped {
            message += " · se detuvo al llenarse el disco";
        }
        if s.get_stems_minimized() {
            s.set_status(format!("{title}: {message}").into());
        }
        s.set_stems_title(title.into());
        s.set_stems_message(message.into());
        let error = state.error.lock().ok().and_then(|e| e.clone());
        s.set_stems_detail(
            error.unwrap_or_else(|| {
                "Si mueves, renombras o editas un archivo, sus stems se perderán.".to_owned()
            })
            .into(),
        );
        s.set_stems_warning("".into());
        s.set_stems_progress(-1.0);
        s.set_stems_busy(false);
        s.set_stems_primary("Cerrar".into());
        s.set_stems_secondary("".into());
        return;
    }
    let total = state.total.max(1) as f64;
    let part = state.part.load(Ordering::Relaxed) as f64 / 1000.0;
    let finished = (done + skipped + failed) as f64;
    s.set_stems_progress(((finished + part) / total).clamp(0.0, 1.0) as f32);
    s.set_stems_message(
        format!("{done} preparadas · {skipped} omitidas · {failed} fallidas").into(),
    );
    let mut detail = state.current.lock().map(|c| c.clone()).unwrap_or_default();
    let worked = (done + failed) as f64;
    let left = state
        .total
        .saturating_sub((done + skipped + failed) as usize) as f64;
    if worked > 0.0 && left > 0.0 {
        let remain = state.started.elapsed().as_secs_f64() / worked * left;
        let eta = eta_secs(remain as u64);
        detail = if detail.is_empty() {
            format!("Quedan {eta}")
        } else {
            format!("{detail} · quedan {eta}")
        };
    }
    if state.paused.load(Ordering::Relaxed) {
        // The timer must not overwrite the pause message with an ETA while the worker
        // finishes the current track and waits before the next one.
        detail = if state.part.load(Ordering::Relaxed) > 0 {
            "Pausa solicitada: termina el tema actual…".into()
        } else {
            "Pausado. Pulsa Reanudar para continuar.".into()
        };
    }
    s.set_stems_detail(detail.into());
}

fn work(tracks: Vec<Track>, multithread: bool, state: Arc<State>, queue: Arc<Mutex<Queue>>) {
    for t in tracks {
        while state.paused.load(Ordering::Relaxed) && !state.cancel.load(Ordering::Relaxed) {
            std::thread::sleep(POLL);
        }
        if state.cancel.load(Ordering::Relaxed) {
            break;
        }
        let Some(file) = t.file.clone() else {
            state.failed.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        if let Some(free) = free_space()
            && free < bytes(&t) + MARGIN
        {
            state.stopped_for_space.store(true, Ordering::Relaxed);
            state.paused.store(true, Ordering::Relaxed);
            note(&queue, &state, None);
            break;
        }
        if stems::cache::has(&file, Quality::Quality) {
            state.skipped.fetch_add(1, Ordering::Relaxed);
            note(&queue, &state, Some(&file));
            continue;
        }
        if let Ok(mut current) = state.current.lock() {
            *current = t.title.clone();
        }
        state.part.store(0, Ordering::Relaxed);
        // A video is also kept in the local media cache, so a later session survives the drive going away. The
        // separation still reads the PCM once; `original` keeps the stems cache keyed by the original file.
        let original = file.clone();
        let play = if video::is_video(&file) {
            video::local::ensure(&file).unwrap_or_else(|e| {
                eprintln!("video cache: {e}");
                file.clone()
            })
        } else {
            file.clone()
        };
        let copied = play != original;
        if copied {
            video::local::pin(&play);
        }
        let decoded =
            audio::decode_track16_cancellable(&play, &|| state.cancel.load(Ordering::Relaxed));
        if copied {
            video::local::unpin(&play);
        }
        match decoded {
            Ok((pcm, _)) => {
                let job = stems::Job::start(
                    original.clone(),
                    Arc::new(pcm),
                    Quality::Quality,
                    multithread,
                );
                loop {
                    if state.cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Some(result) = job.poll() {
                        match result {
                            Ok(()) => {
                                super::stems_index::mark_ready(&file);
                                state.done.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(e) => {
                                state.failed.fetch_add(1, Ordering::Relaxed);
                                if let Ok(mut error) = state.error.lock() {
                                    *error = Some(format!("{}: {e}", t.title));
                                }
                            }
                        }
                        note(&queue, &state, Some(&file));
                        break;
                    }
                    let (_, part) = job.progress.get();
                    state
                        .part
                        .store((f64::from(part) * 1000.0) as u64, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(200));
                }
                state.part.store(0, Ordering::Relaxed);
                if state.cancel.load(Ordering::Relaxed) {
                    break;
                }
            }
            Err(e) => {
                if !state.cancel.load(Ordering::Relaxed) {
                    state.failed.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut error) = state.error.lock() {
                        *error = Some(format!("{}: {e}", t.title));
                    }
                    note(&queue, &state, Some(&file));
                }
            }
        }
    }
    if state.cancel.load(Ordering::Relaxed) {
        super::stems_queue::clear();
    }
    if let Ok(mut current) = state.current.lock() {
        current.clear();
    }
}

fn note(queue: &Arc<Mutex<Queue>>, state: &State, done: Option<&std::path::Path>) {
    let Ok(mut q) = queue.lock() else { return; };
    if let Some(file) = done {
        q.pending.retain(|i| i.file.as_path() != file);
    }
    q.done = state.done.load(Ordering::Relaxed);
    q.skipped = state.skipped.load(Ordering::Relaxed);
    q.failed = state.failed.load(Ordering::Relaxed);
    q.paused = state.paused.load(Ordering::Relaxed);
    if q.pending.is_empty() {
        super::stems_queue::clear();
    } else {
        super::stems_queue::save(&q);
    }
}

/// Pauses or resumes the running batch (a pause waits for the current track to end).
pub(super) fn toggle_pause(app: &App) {
    let s = app.global::<BrowserState>();
    RUNNING.with(|r| {
        let binding = r.borrow();
        let Some(run) = binding.as_ref() else { return; };
        let paused = !run.state.paused.load(Ordering::Relaxed);
        run.state.paused.store(paused, Ordering::Relaxed);
        if let Ok(mut q) = run.queue.lock() {
            q.paused = paused;
            if q.pending.is_empty() {
                super::stems_queue::clear();
            } else {
                super::stems_queue::save(&q);
            }
        }
        s.set_stems_paused(paused);
        if paused {
            s.set_stems_detail("Pausando al terminar el tema actual…".into());
        } else {
            s.set_stems_detail("".into());
        }
    });
}

/// Offers the persisted queue after a restart.
pub(super) fn startup(app: &App) {
    let Some(queue) = super::stems_queue::load() else { return; };
    let n = queue.pending.len();
    let s = app.global::<BrowserState>();
    s.set_stems_queue_message(
        format!("Hay una operación de stems por lotes pausada ({n} pendientes).").into(),
    );
    s.set_stems_queue_open(true);
    RESUME.with(|r| *r.borrow_mut() = Some(queue));
}

pub(super) fn resume_queue(app: &App) {
    let Some(queue) = RESUME.with(|r| r.borrow_mut().take()) else { return; };
    app.global::<BrowserState>().set_stems_queue_open(false);
    start(
        app,
        Plan {
            title: queue.title.clone(),
            tracks: queue.tracks(),
            estimate: 0,
            free: None,
        },
    );
}

pub(super) fn keep_queue(app: &App) {
    if let Some(mut queue) = RESUME.with(|r| r.borrow_mut().take()) {
        queue.paused = true;
        super::stems_queue::save(&queue);
    }
    let s = app.global::<BrowserState>();
    s.set_stems_queue_open(false);
    s.set_status("la operación de stems queda pausada".into());
}

pub(super) fn cancel_queue(app: &App) {
    RESUME.with(|r| *r.borrow_mut() = None);
    super::stems_queue::clear();
    let s = app.global::<BrowserState>();
    s.set_stems_queue_open(false);
    s.set_status("operación de stems cancelada".into());
}

/// Starts the batch for the folder or playlist of `entry`.
pub(super) fn prepare_entry(view: &mut View, app: &App, entry: &super::sidebar::Entry) {
    use super::sidebar::Entry;
    match entry {
        Entry::Pinned(i) => {
            let Some(source) = view.sources.get(*i) else {
                return;
            };
            let super::sources::Kind::Pinned { label, path } = &source.kind else {
                return;
            };
            let (title, path) = (label.clone(), path.clone());
            let weak = app.as_weak();
            app.global::<BrowserState>()
                .set_status(format!("{}: listing the folder…", path.display()).into());
            std::thread::spawn(move || {
                let tracks = openfader::library::folder::Folder::open_partial(&path)
                    .and_then(|f| f.tracks(None));
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(app) = weak.upgrade() else { return };
                    match tracks {
                        Ok(tracks) => prepare(&app, title, tracks),
                        Err(e) => app
                            .global::<BrowserState>()
                            .set_status(format!("cannot read the folder: {e}").into()),
                    }
                });
            });
        }
        Entry::User(id) => {
            let Some(node) = view.nodes.iter().find(|n| n.id == *id).cloned() else {
                return;
            };
            if node.kind != openfader::store::NodeKind::Playlist {
                return;
            }
            let keys = view.store.entries(*id).unwrap_or_default();
            let tracks = view.tracks_of(&keys, false).unwrap_or_default();
            prepare(app, node.name, tracks);
        }
        _ => {}
    }
}
