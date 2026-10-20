//! The UI language (Preferences > View > LANGUAGE): English by default, Spanish shipped with the program
//! (X-04). System = the operating system's locale decides.
//!
//! Two kinds of strings:
//! * the fixed ones in Rust (the recipes of the preferences pages: row labels, headers, chips and dropdown
//!   values) go through [`t`], applied centrally in the [`super::prefs::rows::Row`] builders;
//! * the ones in `.slint` (so far the menu bar of ui/app.slint) are marked with `@tr` there and look their
//!   translation up in the bundled catalogs (ui/translations), switched at run time with
//!   `slint::select_bundled_translation`.
//!
//! [`sync`] applies the LANGUAGE setting: `prefs::wire` calls it at start-up and registers it as a change
//! listener. UI thread only (the translation state is thread-local, like the rest of src/app). Strings that
//! are not in the table come back unchanged: a missing translation is the English text, never a broken UI.

use super::prefs::view::Language;
use std::cell::Cell;

thread_local! {
    /// The last language [`sync`]/[`apply`] handed to Slint; `None` until the first call (the start-up
    /// comparison). Unrelated settings changes do not re-style every window again.
    static APPLIED: Cell<Option<bool>> = const { Cell::new(None) };
    /// Spanish on ([`t`] looks the string up in [`ES`]); English (the literals) off.
    static SPANISH: Cell<bool> = const { Cell::new(false) };
}

/// Translates a fixed UI string of the preferences pages (`t` is what [`super::prefs::rows::Row`] puts
/// behind every label). Linear table: the pages are rebuilt one click at a time, not per frame.
pub fn t<'a>(s: &'a str) -> &'a str {
    if !SPANISH.with(Cell::get) {
        return s;
    }
    ES.iter()
        .find(|(k, _)| *k == s)
        .map_or(s, |(_, v)| v)
}

/// Applies the LANGUAGE setting: the Rust side now takes its strings from [`ES`] and the `.slint` side
/// selects the bundled translation. Call from `prefs::wire` (after the window exists) and on every change.
pub fn sync(p: &super::prefs::Prefs) {
    let spanish = match p.view.language {
        Language::System => system_uses_spanish(),
        Language::English => false,
        Language::Spanish => true,
    };
    apply(spanish);
}

fn apply(spanish: bool) {
    if APPLIED.with(|a| a.get()) == Some(spanish) {
        return;
    }
    APPLIED.with(|a| a.set(Some(spanish)));
    SPANISH.with(|s| s.set(spanish));
    // The preference rows rebuild (`prefs::refresh`); the menus take the bundled catalog.
    let lang = if spanish { "es" } else { "en" };
    if let Err(e) = slint::select_bundled_translation(lang) {
        // "en" always exists (the literals themselves); a miss here means the bundled catalog was
        // never compiled in: warn rather than translate silently.
        eprintln!("warning: the UI language catalogs are not available ({e:?})");
        APPLIED.with(|a| a.set(None));
    }
}

/// Does the operating system use a Spanish locale? Windows reads the user's locale (Windows 10+ style
/// "es-ES"); elsewhere the usual `LC_*` / `LANG` environment variables decide.
///
/// The language part of the code (before `-`, `.` or `_`; case-insensitive) decides: `es`, `es-ES`,
/// `es_MX.utf8`, all Spanish.
fn system_uses_spanish() -> bool {
    fn is_spanish(code: &str) -> bool {
        code.chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .eq("es".chars())
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
        let mut buf = [0 as u16; 85]; // LOCALE_NAME_MAX_LENGTH
        // SAFETY: `buf` holds the locale name and its NUL terminator; the call writes at most 85 WORD.
        let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
        n > 0 && is_spanish(&String::from_utf16_lossy(&buf[..((n.min(85) as usize) - 1).max(0)]))
    }
    #[cfg(not(windows))]
    {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .any(|l| is_spanish(&l))
    }
}

/// The Spanish translations of the fixed strings the preferences rows show (the labels, headers, chips
/// and dropdown values; keys, hints and values that come from places outside the program — device names,
/// folder paths, the plugins' own manifests — are not here and pass through untouched). Product names the
/// DJ world keeps in English (STEMS, BEAT LAB, HOT CUE, SYNC RATE, ...) are not translated either; the
/// table's key is the exact English literal, in file order of no particular rhyme.
const ES: &[(&str, &str)] = &[
    // ------------------------------------------------------------------ the LANGUAGE row itself
    ("LANGUAGE", "IDIOMA"),
    ("SYSTEM", "SISTEMA"),
    ("ENGLISH", "INGLÉS"),
    // ------------------------------------------------------------------ VIEW (src/app/prefs/view.rs)
    ("DISPLAY TYPE", "TIPO DE PANTALLA"),
    ("Tooltips", "Tooltips"),
    ("BROWSE", "NAVEGADOR"),
    ("FONT SIZE", "TAMAÑO DE FUENTE"),
    ("SMALL", "PEQUEÑO"),
    ("NORMAL", "NORMAL"),
    ("LARGE", "GRANDE"),
    ("MEDIUM", "MEDIO"),
    ("Bold characters", "Letras en negrita"),
    ("LINE SPACE", "ESPACIADO"),
    ("Show the selected track to the center", "Llevar la pista seleccionada al centro"),
    ("Coloring of played tracks", "Colorear las pistas reproducidas"),
    ("Reset the colour of played tracks when quitting", "Volver al color original de las pistas reproducidas al salir"),
    ("PLAYED TRACKS", "PISTAS REPRODUCIDAS"),
    ("RESET NOW", "REINICIAR AHORA"),
    ("KEY DISPLAY", "VISOR DE TONALIDAD"),
    ("ALPHANUMERIC", "ALFANUMÉRICO"),
    ("CLASSIC", "CLÁSICO"),
    ("DATABASE", "BASE DE DATOS"),
    ("WAVEFORM", "FORMA DE ONDA"),
    ("DRAWING RATE", "FRECUENCIA DE DIBUJO"),
    ("OVERVIEW / PREVIEW", "OVERVIEW / PREVIEW"),
    ("BEAT COUNT", "CONTEO DE BEATS"),
    ("POSITION", "POSICIÓN"),
    ("Click on the waveform for PLAY and CUE", "Hacer clic en la onda para PLAY y CUE"),
    ("JOG DISPLAY", "VISOR DE JOG"),
    ("HOT CUE COUNTDOWN", "CUENTA ATRÁS DE HOT CUES"),
    ("TRAFFIC LIGHT", "TRAFFIC LIGHT"),
    ("LAYOUT: MEDIA BROWSER", "DISEÑO: NAVEGADOR DE BIBLIOTECA"),
    ("Related Tracks", "Pistas relacionadas"),
    ("Cue Bank", "Banco de cues"),
    ("Track Suggestion", "Sugerencias de pistas"),
    ("Explorer", "Explorador"),
    ("LAYOUT: BROWSER PANEL", "DISEÑO: PANEL DEL NAVEGADOR"),
    ("CUE Analysis Playlist", "Lista CUE Analysis"),
    ("Playlist Palette", "Paleta de listas"),
    ("Display Cue Markers on Preview", "Mostrar los marcadores de CUE en la previsualización"),
    ("Display All Tracks in the Playlists", "Mostrar todas las pistas en las listas"),
    ("LAYOUT: DECK", "DISEÑO: DECKS"),
    ("Display SYNC RATE", "Mostrar SYNC RATE"),
    ("Phrase (Enlarged Waveform)", "Frases (Onda ampliada)"),
    ("Phrase (Full Waveform)", "Frases (Onda completa)"),
    ("Always show the types of phrases", "Mostrar siempre los tipos de frases"),
    ("Vocal (Enlarged Waveform)", "Vocales (Onda ampliada)"),
    ("Vocal (Full Waveform)", "Vocales (Onda completa)"),
    ("EFFECT PANEL", "PANEL DE EFECTOS"),
    ("ENLARGED WAVEFORMS", "ONDAS AMPLIADAS"),
    ("PAD MODE", "MODO PAD"),
    ("COLOR", "COLOR"),
    ("SKIN", "APARIENCIA"),
    ("WAVEFORM COLOR", "COLOR DE ONDA"),
    ("3BAND", "3BANDAS"),
    ("BLUE", "AZUL"),
    ("GREEN", "VERDE"),
    ("ORANGE", "NARANJA"),
    ("RED", "ROJO"),
    ("PURPLE", "MORADO"),
    ("HOT CUE COLOR", "COLOR DE HOT CUES"),
    ("PAD FX COLOR", "COLOR PAD FX"),
    ("SAMPLER COLOR", "COLOR DEL SAMPLER"),
    ("SYNC WITH FREQUENCY", "SINCRONIZAR CON LA FRECUENCIA"),
    ("DECK LAYOUT", "DISEÑO DE DECKS"),
    ("HORIZONTAL", "HORIZONTAL"),
    ("VERTICAL", "VERTICAL"),
    ("enlarged waveforms", "ondas ampliadas"),
    ("Mixer", "Mezclador"),
    ("EXPORT LAYOUT", "DISEÑO DE EXPORT"),
    ("DECKS", "DECKS"),
    ("2 DECKS", "2 DECKS"),
    ("4 DECKS", "4 DECKS"),
    // ------------------------------------------------------------------ AUDIO (prefs/audio)
    ("CONFIGURATION", "CONFIGURACIÓN"),
    ("MASTER OUTPUT", "SALIDA MASTER"),
    ("BOOTH OUTPUT", "SALIDA BOOTH"),
    ("HEADPHONES OUTPUT", "SALIDA DE AURICULARES"),
    ("CONTROL DEVICE", "DISPOSITIVO DE CONTROL"),
    ("CONTROL DEVICE INFORMATION", "INFORMACIÓN DEL DISPOSITIVO DE CONTROL"),
    ("OUTPUT IN USE", "SALIDA EN USO"),
    ("MIXER MODE", "MODO DE MEZCLADOR"),
    ("EXTERNAL MIXER MODE", "MODO MEZCLADOR EXTERNO"),
    ("INTERNAL", "INTERNO"),
    ("EXTERNAL", "EXTERNO"),
    ("OUTPUT CHANNELS · INTERNAL", "CANALES DE SALIDA · INTERNO"),
    ("OUTPUT CHANNELS · EXTERNAL", "CANALES DE SALIDA · EXTERNO"),
    ("INPUT CHANNELS", "CANALES DE ENTRADA"),
    ("INPUT / OUTPUT", "ENTRADA / SALIDA"),
    ("INPUT DEVICE", "DISPOSITIVO DE ENTRADA"),
    ("no such input device: nothing to record from", "no existe ese dispositivo de entrada: no hay nada que grabar"),
    ("SAMPLE RATE", "FRECUENCIA DE MUESTREO"),
    ("BUFFER SIZE", "TAMAÑO DEL BÚFER"),
    ("METRONOME", "METRÓNOMO"),
    ("TONE", "TONE"),
    ("VOLUME", "VOLUMEN"),
    ("MICROPHONE", "MICRÓFONO"),
    ("MICROPHONE INPUT", "ENTRADA DE MICRÓFONO"),
    ("Enable microphone input", "Activar la entrada de micrófono"),
    ("MIC LEVEL", "NIVEL DE MIC"),
    ("TALKOVER LEVEL", "NIVEL DE TALKOVER"),
    ("TALKOVER MODE", "MODO TALKOVER"),
    ("MIC CHANNELS", "CANALES DE MIC"),
    ("BOOTH LEVEL", "NIVEL BOOTH"),
    ("PROBLEM", "PROBLEMA"),
    ("NOTE", "NOTA"),
    ("REC SOURCE", "FUENTE REC"),
    ("needs a microphone input: OpenFadeR has none", "necesita una entrada de micrófono: OpenFadeR no tiene ninguna"),
    // ------------------------------------------------------------------ ANALYSIS (prefs/analysis.rs)
    ("TRACK ANALYSIS", "ANÁLISIS DE PISTAS"),
    ("OPENFADER ANALYSIS", "ANÁLISIS OPENFADER"),
    ("TRACK ANALYSIS MODE", "MODO DE ANÁLISIS DE PISTAS"),
    ("TRACK ANALYSIS SETTING", "AJUSTES DE ANÁLISIS DE PISTAS"),
    ("BPM / Grid", "BPM / Rejilla"),
    ("Use high precision BeatGrid analysis", "Usar análisis BeatGrid de alta precisión"),
    ("ANALYSIS PROCESS", "PROCESO DE ANÁLISIS"),
    ("QUALITY", "CALIDAD"),
    ("SPEED", "VELOCIDAD"),
    ("Apply multi-thread to the analysis process", "Usar múltiples hilos en el proceso de análisis"),
    ("CUE ANALYSIS", "ANÁLISIS DE CUES"),
    ("BPM RANGE", "RANGO DE BPM"),
    ("N BARS", "N COMPASES"),
    ("every {n} bars", "cada {n} compases"),
    ("AUTO", "AUTO"),
    ("AUTO ANALYSIS", "ANÁLISIS AUTOMÁTICO"),
    ("Auto Analysis", "Análisis automático"),
    ("ANALYZE NOW", "ANALIZAR AHORA"),
    ("ANALYZE LIBRARY", "ANALIZAR BIBLIOTECA"),
    ("Write the value to the ID3 tag", "Escribir el valor en la etiqueta ID3"),
    ("KEY ANALYSIS", "ANÁLISIS DE TONALIDAD"),
    ("KEY", "TONALIDAD"),
    ("Personal Cue Tendency", "Tendencia personal de cues"),
    ("Prohibit overwriting CUEs", "Prohibir sobrescribir los CUEs"),
    ("Set to 1.1 Bars", "Poner en 1.1 compases"),
    ("on phrases", "sobre las frases"),
    ("Phrase", "Frase"),
    ("Vocal", "Vocal"),
    ("HOT CUES", "HOT CUES"),
    ("MEMORY CUES", "MEMORY CUES"),
    ("THE CONNECTED LIBRARY", "LA BIBLIOTECA CONECTADA"),
    // ------------------------------------------------------------------ CONTROLLER (prefs/controller)
    ("DECK", "DECK"),
    ("TEMPO RANGE", "RANGO DE TEMPO"),
    ("JOG ROTATION SPEED", "VELOCIDAD DE GIRO DEL JOG"),
    ("Load Setting: playback starts at the Memory/Hot Cue nearest to the beginning", "Load Setting: la reproducción empieza en el Memory/Hot Cue más cercano al comienzo"),
    ("KEY LOCK", "KEY LOCK"),
    ("Auto Cue: the cue point is the first sound above the level", "Auto Cue: el punto de cue es el primer sonido por encima del nivel"),
    ("AUTO CUE LEVEL", "NIVEL DEL AUTO CUE"),
    ("After spinning the jog on CUE, return to the cue point and pause", "Tras girar el jog en CUE, vuelve al punto de cue y pausa"),
    ("Eject/Load Lock: no LOAD while a track is playing", "Bloqueo de EJECT/LOAD: no se carga ninguna pista mientras suena otra"),
    ("EJECT/LOAD LOCK", "BLOQUEO DE EJECT/LOAD"),
    ("Memory Cue Call Lock: no memory cue call while a track is playing", "Bloqueo de Memory Cue: no se llaman memory cues mientras suena una pista"),
    ("Needle Lock: no needle search while a track is playing", "Bloqueo de aguja: no se busca con la aguja mientras suena una pista"),
    ("HOT CUE: during pause, GATE playback is applied", "HOT CUE: en pausa se aplica la reproducción GATE"),
    ("Continuous Playing: load the next track of the list when one ends", "Reproducción continua: carga la siguiente pista de la lista cuando una termina"),
    ("Prefetch: download the next track of the list in the background", "Precarga: descarga en segundo plano la siguiente pista de la lista"),
    ("Allow BEAT/BPM SYNC with double/half BPM", "Permitir BEAT/BPM SYNC con el BPM doble/mitad"),
    ("Automatically changes the BEAT/BPM SYNC setting", "Cambia el ajuste BEAT/BPM SYNC automáticamente"),
    ("SYNC TYPE", "TIPO DE SYNC"),
    ("MIX POINT LINK", "MIX POINT LINK"),
    ("Enable MIX POINT LINK Function", "Activar la función MIX POINT LINK"),
    ("Automatically limits the zoom out range", "Limita automáticamente el rango de alejamiento del zoom"),
    ("Automatically switches between MIX IN/MIX OUT", "Alterna automáticamente entre MIX IN/MIX OUT"),
    ("Display grids", "Mostrar rejillas"),
    ("Display time", "Mostrar el tiempo"),
    ("Display only when waveform is enlarged", "Mostrar solo cuando la onda esté ampliada"),
    ("DEFAULT MIX POINT SETTING", "AJUSTE PREDETERMINADO DEL PUNTO DE MEZCLA"),
    ("MIX IN", "MIX IN"),
    ("MIX OUT", "MIX OUT"),
    ("EFFECT", "EFECTO"),
    ("1 (MASTER)", "1 (MASTER)"),
    ("2 (EACH UNIT)", "2 (CADA UNIDAD)"),
    ("RELEASE FX UNITS", "UNIDADES RELEASE FX"),
    ("Enable MERGE FX", "Activar MERGE FX"),
    ("Stop playing the operating deck after MERGE FX finishes", "Detiene el deck en uso cuando termina MERGE FX"),
    ("Apply RELEASE FX on CFX: SOUND COLOR FX goes off while RELEASE is held", "Aplicar RELEASE FX en CFX: SOUND COLOR FX se apaga mientras se mantiene RELEASE"),
    ("MIXER", "MEZCLADOR"),
    ("CONTROLLER AUDIO", "AUDIO DEL CONTROLADOR"),
    ("Auto Gain: even out the loudness of loaded tracks", "Auto Gain: iguala el volumen de las pistas cargadas"),
    ("OUTPUT LEVEL", "NIVEL DE SALIDA"),
    ("EQ", "EQ"),
    ("MICROPHONE / TALK OVER", "MICRÓFONO / TALK OVER"),
    ("OTHERS", "OTROS"),
    ("IMMEDIATE", "INMEDIATO"),
    ("NEXT BEAT", "SIGUIENTE BEAT"),
    ("OTHERS (needs Q on)", "OTROS (necesita Q activado)"),
    ("Q switches all decks at once", "Q cambia todos los decks a la vez"),
    ("QUANTIZE: HOT CUE", "QUANTIZE: HOT CUE"),
    ("QUANTIZE: LOOP/SAMPLER(LOOP)", "QUANTIZE: LOOP/SAMPLER(LOOP)"),
    ("QUANTIZE: SEQUENCER", "QUANTIZE: SEQUENCER"),
    ("QUANTIZE: REVERSE", "QUANTIZE: REVERSE"),
    // ------------------------------------------------------------------ EXTENSIONS (prefs/extensions.rs)
    ("STEMS", "STEMS"),
    ("ENABLE THE STEMS FUNCTION", "ACTIVAR LA FUNCIÓN STEMS"),
    ("STEMS MODE", "MODO STEMS"),
    ("3 STEMS", "3 STEMS"),
    ("4 STEMS", "4 STEMS"),
    ("ACTIVE STEM SETTING", "STEM ACTIVO"),
    ("STEM WAVEFORM DISPLAY", "VISOR DE ONDA STEMS"),
    ("STEMS ANALYSIS PROCESS", "PROCESO DE ANÁLISIS STEMS"),
    ("STEM INSTANT DOUBLES", "STEM INSTANT DOUBLES"),
    ("STEMS LAYOUT", "DISEÑO STEMS"),
    ("Custom layout: STEM ISO knobs", "Diseño personalizado: mandos STEM ISO"),
    ("Custom layout: STEM FX", "Diseño personalizado: STEM FX"),
    ("Custom layout: ACTIVE STEM buttons", "Diseño personalizado: botones ACTIVE STEM"),
    ("VIDEO (BETA)", "vídeo (BETA)"),
    ("Enable Video Function", "Activar la función de vídeo"),
    ("LIGHTING (BETA)", "iluminación (BETA)"),
    ("Enable Lighting Function", "Activar la función de iluminación"),
    ("Enable the BEAT LAB Function", "Activar la función BEAT LAB"),
    ("DRUM CAPTURE", "DRUM CAPTURE"),
    ("Display track titles on DRUM SWAP SLOT", "Mostrar los títulos en los slots de DRUM SWAP"),
    ("DRUM SWAP", "DRUM SWAP"),
    ("FACTORY SAMPLES PACK (sample pack)", "PACK DE SAMPLES DE FÁBRICA"),
    ("LOCATION OF FILES", "UBICACIÓN DE LOS ARCHIVOS"),
    ("empty = ~/Music/OpenFadeR/Drums", "vacío = ~/Music/OpenFadeR/Drums"),
    ("empty = ~/Music/OpenFadeR/Sampler", "vacío = ~/Music/OpenFadeR/Sampler"),
    // ------------------------------------------------------------------ DJ SYSTEM (prefs/dj_system)
    ("DEVICE SETTINGS", "AJUSTES DEL DISPOSITIVO"),
    ("No device connected", "No hay ningún dispositivo conectado"),
    ("settings of one device are edited here once it is connected", "los ajustes de un dispositivo se editan aquí cuando está conectado"),
    ("CONNECTED DEVICES", "DISPOSITIVOS CONECTADOS"),
    ("DEVICE LIBRARY BACKUP", "COPIA DE LA BIBLIOTECA DEL DISPOSITIVO"),
    ("Automatically backup device library upon removal", "Copia de seguridad automática de la biblioteca del dispositivo al desconectarla"),
    ("Backup now and the list of backups are on each device's panel", "Copiar ahora y la lista de copias están en el panel de cada dispositivo"),
    ("Backups", "Copias de seguridad"),
    ("HOW", "CÓMO"),
    ("TWO-WAY", "BIDIRECCIONAL"),
    ("THIS COMPUTER", "ESTE EQUIPO"),
    ("Keep cues, grid and phrases the same on the collection and the device copy of a track", "Mantener cues, rejilla y frases iguales en la colección y en la copia del dispositivo"),
    ("Plugging a device in fills the copy that has none; loading a track makes the copy you loaded win", "Conectar el dispositivo rellena la copia que no tiene; cargar una pista hace ganar a la copia cargada"),
    ("HISTORY", "HISTORIAL"),
    ("Delete from the device after importing the play history", "Borrar del dispositivo tras importar el historial de reproducción"),
    ("Delete playlist tracks from device if playlist is deleted", "Borrar del dispositivo las pistas de una lista al eliminar esta"),
    ("Computer Name", "Nombre del equipo"),
    ("DELETE TRACKS", "BORRAR PISTAS"),
    ("CUE SYNC", "SINC. DE CUES"),
    // ------------------------------------------------------------------ LIBRARIES (prefs/libraries.rs)
    ("SOURCE", "FUENTE"),
    ("MODE", "MODO"),
    ("ON CONFLICT", "EN CASO DE CONFLICTO"),
    ("No library is connected.", "No hay ninguna biblioteca conectada."),
    ("install or enable one in EXTENSIONS", "instala o activa una en EXTENSIONES"),
    ("DETECTED", "DETECTADO"),
    ("CONNECT", "CONECTAR"),
    ("CONNECTED", "CONECTADO"),
    ("DISCONNECT", "DESCONECTAR"),
    ("NEWEST WINS", "GANA EL MÁS RECIENTE"),
    ("OURS WINS", "GANA EL NUESTRO"),
    ("SOURCE WINS", "GANA LA FUENTE"),
    ("TRACKS", "PISTAS"),
    ("SYNC NOW", "SINCRONIZAR AHORA"),
    ("FOLDER…", "CARPETA…"),
    ("FILE…", "ARCHIVO…"),
    ("ADD", "AÑADIR"),
    ("DELETE", "BORRAR"),
    // ------------------------------------------------------------------ ADVANCED (prefs/advanced.rs)
    ("AUDIO INPUT", "ENTRADA DE AUDIO"),
    ("RECORDINGS", "GRABACIONES"),
    ("RECORDING STARTS WHEN", "LA GRABACIÓN EMPIEZA CUANDO"),
    ("RECORDING ENDS WHEN", "LA GRABACIÓN TERMINA CUANDO"),
    ("REC BUTTON", "BOTÓN REC"),
    ("Automatically import a recorded file", "Importar un archivo grabado automáticamente"),
    ("Automatically display the tag information of a recorded file", "Muestra la información de etiquetas de un archivo grabado automáticamente"),
    ("Automatically optimize the audio level of a recorded file", "Optimiza el nivel de audio de un archivo grabado automáticamente"),
    ("Automatically separate files when silence continues", "Separa los archivos cuando continúa el silencio automáticamente"),
    ("SILENCE LENGTH", "LONGITUD DEL SILENCIO"),
    ("SILENCE LEVEL THRESHOLD", "UMBRAL DE SILENCIO"),
    ("EXPORT LOOP AS WAV", "EXPORTAR LOOP COMO WAV"),
    ("STATUS", "ESTADO"),
    // ------------------------------------------------------------------ MIX POINTS (prefs/controller/mixpoint.rs)
    // ------------------------------------------------------------------ SAMPLER (prefs/controller/sampler.rs)
    ("SAMPLER", "SAMPLER"),
    ("Roll", "Roll"),
    ("Capture the loop in an empty slot by pressing its pad during loop playback", "Captura el loop en un slot vacío pulsando su pad mientras el loop suena"),
    ("Do not overwrite a loaded slot", "No sobrescribir un slot cargado"),
    ("Sequence Load: a track loaded into a slot loops over its full length and starts", "Sequence Load: una pista cargada en un slot hace loop de su longitud completa y empieza"),
    ("hits are recorded on the grid of OTHERS > QUANTIZE > SEQUENCER (beat length of QUANTIZE)", "los hits se graban en la rejilla de OTROS > QUANTIZE > SEQUENCER (longitud del beat de QUANTIZE)"),
    // ------------------------------------------------------------------ catalogue pages (src/app/catalog)
    ("AUTO BEAT LOOP", "LOOP DE BEATS AUTOMÁTICO"),
    ("Add a comment to CUE", "Añadir un comentario al CUE"),
    ("Auto Panel Mode: the SAMPLER button of a controller opens the sampler", "Auto Panel Mode: el botón SAMPLER de un controlador abre el sampler"),
    ("BEAT GRID AND CUES FROM", "REJILLA DE BEATS Y CUES DE"),
    ("CAPTURE FOLDER", "CARPETA DE CAPTURA"),
    ("COPY", "COPIAR"),
    ("CROSSFADER CURVE", "CURVA DEL CROSSFADER"),
    ("Copy from Preferences", "Copiar desde Preferencias"),
    ("Delete file", "Borrar archivo"),
    ("Display the CUE Analysis playlist", "Mostrar la lista CUE Analysis"),
    ("Import the play history automatically", "Importar el historial de reproducción automáticamente"),
    ("LENGTH", "LONGITUD"),
    ("MUTE/SOLO ON LOAD", "MUTE/SOLO AL CARGAR"),
    ("No device plugin", "Sin plugin de dispositivo"),
    ("EXTRACT", "EXTRAER"),
    ("Repeat CUE settings per N bars from 1.1 Bars", "Repetir los ajustes de CUE cada N compases desde 1.1"),
    ("Reset to defaults", "Restablecer los valores"),
    ("SET CUE DURING ANALYSIS", "PONER CUES DURANTE EL ANÁLISIS"),
    ("START-UP (OpenFadeR)", "INICIO (OpenFadeR)"),
    ("20 S SILENCE", "20 S DE SILENCIO"),
    ("VINYL SPEED: RELEASE/START", "VELOCIDAD VINYL: RELEASE/START"),
    ("VINYL SPEED: TOUCH/BRAKE", "VELOCIDAD VINYL: TOUCH/BRAKE"),
    // The preferences the upstream added in 66d2698 (the bold beat grid and the large hot cue markers): they came
    // in English, and their options are the names of the enum, in upper case.
    ("BEAT GRID", "REJILLA DE BEATS"),
    ("HOT CUE MARKERS", "MARCAS DE HOT CUES"),
    ("BOLD", "NEGRITA"),
    (
        "Bold: thicker lines, wider downbeats",
        "Negrita: líneas más gruesas y beats fuertes más anchos",
    ),
    (
        "Large: double-size flags and letters",
        "Grande: banderas y letras al doble de tamaño",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_falls_back_to_english_and_translates_the_table_when_spanish_applies() {
        assert_eq!(t("LANGUAGE"), "LANGUAGE");
        assert_eq!(t("not in the table"), "not in the table");
        SPANISH.with(|s| s.set(true));
        assert_eq!(t("LANGUAGE"), "IDIOMA");
        assert_eq!(t("not in the table"), "not in the table");
        SPANISH.with(|s| s.set(false));
        assert_eq!(t("LANGUAGE"), "LANGUAGE");
    }

    /// The window exists and the catalogs ship with it: after the window a language selection succeeds
    /// (a miss would mean the bundled catalog was not compiled in — e.g. a domain or path mix-up).
    #[test]
    fn the_menus_take_the_bundled_catalogs_after_the_window() {
        use crate::app::{App, headless};
        headless::init();
        let _app = App::new().unwrap();
        apply(true);
        assert_eq!(APPLIED.with(|a| a.get()), Some(true));
        apply(false);
        assert_eq!(APPLIED.with(|a| a.get()), Some(false));
    }
}
