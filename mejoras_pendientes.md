# Mejoras y arreglos pendientes

Notas de trabajo (no forman parte del build). Orden aproximado de prioridad; se irán
haciendo de a uno, compilando en release al cerrar cada tanda.

> Recordatorio: la app **solo** debe ejecutarse en **release** (`start.sh` línea 14:
> `cargo run --release`). El build debug sirve para compilar/`cargo test`, no para correr la GUI.

> Última revisión: **26/10/2026**, contra `main` (`ddcc4a0`, 0.4.1). Las tandas de abajo están
> cerradas y `hallazgos.md` no tiene nada sin resolver. Sigue abierto **§16** (decisiones) y el
> **§17**, el plan de backport selectivo desde upstream (en validación, sin código cambiado).

## 1. Terminales que se abren solas — ARREGLADO
- **Síntoma:** cada vez que se carga un tema de la nube (deezer/youtube) o un vídeo se abre
  una ventana de terminal (a veces se cierra sola, a veces no).
- **Causa:** tres capas distintas de procesos de consola; en las tres el flag que se usaba
  (`CREATE_NO_WINDOW`) resulta insuficiente en este Windows (ver la clave más abajo):
  1. el proceso del plugin (Python);
  2. **los ayudantes que lanza el propio plugin**: `subprocess.run` de `plugins/deezer/main.py`
     (deemix, powershell) y de `plugins/yt-dlp/main.py` + `plugins-repo/yt-dlp/main.py` (yt-dlp);
  3. **los ayudantes de vídeo del núcleo**: `ffmpeg` (`src/video/stream.rs`, `src/video/audio.rs`)
     y `ffprobe` (`src/video/probe.rs`).
- **Arreglo aplicado:**
  - **Regla correcta (corregida en la tanda 3):** lo que decide no es Windows 11 ni Windows
    Terminal, sino **si el ayudante lanza a su vez otro programa de consola**.
    - Ayudante **hoja** (no lanza nada): `CREATE_NO_WINDOW | DETACHED_PROCESS` (0x08000008)
      vale; el hijo no tiene consola y no se ve nada.
    - Ayudante que **lanza un hijo de consola** (deemix es un `.cmd` que ejecuta `node`;
      yt-dlp lanza `ffmpeg`): con `DETACHED_PROCESS` el nieto **no hereda consola** y Windows
      le crea una **nueva, con ventana** — la terminal que aparecía en cada descarga
      (`node "...\deemix-src\packages\cli\dist\main.cjs"`). Con `CREATE_NO_WINDOW` a secas el
      ayudante tiene una consola **oculta** y el nieto la hereda: no se ve nada.
  - Uso actual:
    - `src/video/mod.rs`: helper `no_window(&mut Command)`, usado por `stream.rs`, `audio.rs`
      y `probe.rs` (`ffmpeg` / `ffprobe`, hojas) → 0x08000008;
    - `src/plugin/mod.rs` (`Plugin::call`) y `src/plugin/lua.rs` (`rk.run`) → 0x08000008;
    - `src/store/device_settings.rs` (`hostname`, hoja) → 0x08000008.
  - **Plugins Python** (`plugins/deezer/main.py`, `plugins/yt-dlp/main.py`): `NO_WINDOW` pasa a
    `CREATE_NO_WINDOW` a secas. Comprobado con el árbol de ventanas real: con las banderas
    viejas, un `check` del plugin abre una ventana de Windows Terminal con el `node` de deemix;
    con la nueva, **0 ventanas** y deemix responde igual (`deemix 0.2.0`).
  - `src/app/library/dialog.rs` (selector de archivos) y el ARL del plugin deezer:
    `CREATE_NO_WINDOW` a secas (igual que arriba, por la consola oculta).
- **Excepción (documentada):** donde el hijo es **PowerShell mostrando un diálogo** (el selector de
  archivos de la app y el diálogo del ARL del plugin deezer) hay que dejarle consola, porque
  PowerShell **no muestra sus diálogos sin consola** (`ShowDialog` vuelve al instante y no aparece
  nada; comprobado). Esos dos van con `CREATE_NO_WINDOW` a secas: el diálogo funciona, aunque
  Windows 11 puede enseñar la pestaña del terminal un momento. Queda como pendiente cambiarlos
  por un diálogo nativo (FFI) para quitarlo del todo.
- **Estado:** recompilado en release.

## 2. Panel Beat FX: "línea" que interrumpe la visualización — ARREGLADO
- **Síntoma:** al hacer clic en FX (panel beat FX) aparecía una línea azul (`Theme.accent`) a
  todo lo ancho en la fila de los slots, y en el tema VDJ la pantalla maestra la cortaba.
- **Causa 1 (huecos):** el `FxPanel` tenía `background: Theme.bg` (el color más oscuro) y sus
  bandas iban separadas por huecos de `spacing: 2px`. Esos huecos se veían como líneas.
  Arreglo: el panel pasa a `background: Theme.panel`, así los huecos se funden con las bandas.
- **Causa 2 (la banda azul):** Slint escribe los `float` con el **separador decimal del
  sistema** (coma en es-ES), y los `commands` de los `Path` se parsean como SVG, que solo
  admite el punto. El `Knob` (ui/gauges.slint) generaba `M 4,0502524 13,949747 A 7 7 ...`,
  el parser lo rechazaba (`Error while parsing path commands`) y femtovg dibujaba basura: una
  banda del color del trazo (el arco de valor, `Theme.accent`). Por eso solo salía en la vista
  BEAT FX (los slots tienen `Knob`), no en RELEASE/MERGE ni en el rack, y no salía con el
  renderer software. De paso, **ningún** arco ni aguja de ningún knob se dibujaba (tampoco el
  playhead del jog, que usaba la misma concatenación de `float`).
- **Arreglo aplicado:** `ui/gauges.slint` y `ui/deck-controls.slint` redondean las coordenadas
  (`round(...)`) antes de concatenarlas: los `commands` quedan con enteros, sin coma decimal.
  Los arcos y las agujas se dibujan ya, y la banda desaparece (0 errores de parseo en stderr).

## 3. Menú superior: la UI se congela al hacer clic (el audio sigue) — ARREGLADO
- **Síntoma:** al abrir/hacer clic en el menú de arriba, la parte visual se detenía
  (el tiempo quedaba pegado, las waveforms no avanzaban), pero la música seguía sonando.
- **Causa:** en Windows Slint entrega el `MenuBar` al sistema (muda), que abre un bucle
  modal de Win32 y bloquea el event loop de winit; los timers y repaints de Slint se paran.
  El audio corre en otro hilo, por eso no se corta.
- **Arreglo aplicado:** `src/main.rs` fija `SLINT_NO_MUDA=1` al arrancar (antes de cualquier
  llamada a Slint): Slint dibuja el menú dentro de la ventana y no hay bucle modal.

## 4. Añadir a la playlist temas de deezer/youtube (archivos de "nube") — ARREGLADO
- **Síntoma:** un tema/video de deezer o youtube no se podía añadir a la playlist; no
  aparecía en la lista.
- **Causa:** el Collection descartaba las pistas sin `file` (`src/store/collection.rs`), así
  que la clave de la playlist (`meta|título|artista`) no resolvía contra la biblioteca.
- **Arreglo aplicado:** la Colección guarda ahora pistas de plugin:
  - migración v16: columnas `plugin`, `source`, `remote_id` en `collection`;
  - `from_row` reconstruye `Track.remote` y deja `file: None`;
  - `put_rows`/`collection_put` aceptan pistas con `remote` (su `path` es una identidad
    sintética única);
  - `View::in_collection` da de alta la pista remota antes de añadirla a una playlist o
    paleta, de modo que se guarda como cualquier otra (`t<id>`).
- **Reproducción:** ya funcionaba: `src/app/deck/load.rs` descarga la pista remota al cargar
  el deck (y reutiliza la caché).

## 5. Pantallas de vídeo: una por deck y una central con la mezcla (tema "VDJ") — ARREGLADO
- **Deseo:** ver las transiciones visuales cuando la pista tiene vídeo, con un interruptor
  como los de FX/sampler, **sin** acortar los channel faders; y, según la referencia enviada
  (Virtual DJ), **una pantalla por deck** más **una central** con la mezcla en vivo.
- **Arreglo aplicado (3ª versión = tema "VDJ"):** el layout nuevo ya no va fijo en `app.slint`
  sino que lo pide el tema, con el token nuevo `Theme.stage-layout` (0 = escenario clásico,
  1 = escenario VDJ). Los temas clásicos quedan **igual** que antes.
  - `ui/theme.slint`: token `stage-layout` (grupo LAYOUT del editor de temas) y tamaños
    `h-video` (72 px, alto de la tira) y `w-deck-video` (224 px, ancho de la pantalla de deck).
    `h-deck-2` vuelve a 240 px de serie (el tema VDJ lo sube, ver ítem 9).
  - `ui/app.slint`: con `stage-layout == 1` el panel FX baja a la **tira** bajo los decks (los
    units de FX dejan libre el centro) y la **pantalla central** (16:9, `image-fit: contain`)
    se centra bajo la columna del mixer, compartiendo la tira; el browser cede el alto.
    En el escenario clásico el panel FX sigue arriba, entre la barra y los waveforms.
  - `ui/deck-panel.slint`: con `stage-layout == 1` y VIDEO PREVIEW activo, cada deck lleva su
    **pantalla** entre el bloque de loop y el jog, dibujada con el ancho que le ceden los pads
    de hot cue (que se estrechan); sin vídeo sale negra (marco de 1 px).
  - `src/app/video/tick.rs`: las filas por deck se publican **también** con el preview activo
    (antes sólo con la vista VIDEO abierta), porque alimentan las pantallas de los decks.
  - **Sin vídeo no se gasta nada:** con el preview apagado `tick.rs` sale antes de todo y la UI
    no dibuja ni la tira ni las pantallas; con el preview encendido y decks vacíos no se invoca
    `ffmpeg` (las pantallas salen negras).
  - Interruptores: chip **VIDEO** en la barra del mixer (junto a SPLIT) y menú **Ver > Video Preview**.
  - **Tema nuevo `VDJ`** (ver ítem 6) con este layout.

## 6. Tema tipo Virtual DJ (mezclas de vídeo) — ARREGLADO: tema "VDJ"
- **Deseo:** un tema para ver/mezclar vídeos, con la organización de Virtual DJ (decks con su
  pantalla, pantalla central con la mezcla, faders largos, FX debajo de los decks, browser abajo).
- **Estado:** hecho como **tema** (no como código fijo), para no cambiar los temas clásicos.
  - `themes-repo/vdj.json` (repositorio de temas) y copia instalada en
    `%APPDATA%\openfader\extension\skin\vdj.json` (es la carpeta que lee la app).
  - Colores grafito + cian + naranja (aire Virtual DJ), `stage-layout: 1`, `h-deck-2: 264`,
    `h-video: 72`, `w-deck-video: 224`, `pad-style: 0`.
  - Se elige como cualquier tema: PREFS > THEMES > **VDJ** (o la tecla de skin).
  - El **escenario clásico** (0) no cambia: el panel FX arriba y sin pantallas de deck.
  - Documentado en `docs/skins.md` (tabla de temas, tokens `stage-layout`, `h-video`,
    `w-deck-video` y grupo LAYOUT).

## 7. Recompilar release con los fixes (cierre de tanda) — HECHO
- Tanda 1 (18/10): ítems 2–5 y 8.
- Tanda 2 (19/10): ítem 1 completo (terminales), ítem 5 en su 2ª versión (pantalla del master
  en el mixer). Comprobar en la app: cargar un vídeo y un tema de la nube **sin** que aparezca
  ninguna terminal, y la pantalla del master junto a los canales (chip **VIDEO** / Ver).
- Capturas de contraste en `target/shots/video-preview-{off,on}.png` (no versionado).
- **Verificado en release (19/10):** cargando un vídeo, el `ffmpeg` del reproductor **no tiene
  `conhost` hijo** y no aparece ninguna ventana de terminal; la pantalla del master muestra el
  vídeo del deck mezclado por el crossfader.
- Tanda 3 (19/10): ítems 5 (3ª versión), 6, 9 y 10 — el escenario **VDJ** como **tema**.
  Comprobar en la app: PREFS > THEMES > **VDJ** → FX debajo de los decks, pantalla en cada deck
  y pantalla central bajo el mixer (ambas negras sin vídeo), channel faders largos, browser más
  corto; y que con los temas clásicos nada cambió. Captura en `target/shots/vdj-stage.png`.

## 9. Alargar los channel faders (opcional, a decidir) — ARREGLADO (en el tema VDJ)
- **Idea del usuario:** agrandar la zona de decks y quitarle alto al browser.
- **Estado:** el tema **VDJ** sube `Theme.h-deck-2` a **264 px** (de serie 240): los faders de
  canal pasan de ~57 a ~66-72 px de largo, los decks quedan más holgados y el browser cede el
  alto. Con los temas clásicos nada cambia.
- Se puede afinar en el propio tema (`themes-repo/vdj.json`, token `h-deck-2`) o desde el
  editor de temas (grupo SIZES) sin recompilar: guardar el JSON y el tema se recarga solo.

## 10. Tamaño de la pantalla del master vs. ancho de los pads — RESUELTO
- La pantalla central ya no le roba ancho a los canales: va en la **tira** bajo la columna del
  mixer y su ancho sale de su alto (`16/9`, `screen-w` en `ui/app.slint`); con el FX abierto la
  tira mide 110 px → pantalla ~195 px, y con el FX cerrado 72 px → ~128 px.
- El ancho que ceden los pads es el de las pantallas **de deck**: `Theme.w-deck-video` (224 px,
  `ui/theme.slint`), y sólo en los decks (cada pantalla se queda con `min(w-deck-video, hueco
  - 150px)`, así que en ventanas estrechas encoge sola).
- Ajuste sin recompilar: token `w-deck-video` (o `h-video`) en el JSON del tema.

## 8. Indicador de "ya descargado" en la búsqueda — ARREGLADO: ✓
- **Deseo:** al buscar un tema con varias versiones (Deezer/YouTube), marcar cuál(es) ya
  está(n) descargada(s) para no volver a bajarlas.
- **Arreglo aplicado:**
  - `src/plugin/source/fetch.rs`: `is_downloaded(&Remote)`, reexportado;
  - `src/app/browser/table.rs`: `Facts.downloaded` y columna `DOWNLOADED` (12 celdas);
  - `ui/browser-table.slint`: un **✓** verde a la izquierda del número de fila.
- **Alcance:** se calcula con el `Remote {plugin, source, id}` exacto (genérico para Deezer y
  YouTube). La acción "Descargar de una URL" guarda por nombre original, así que queda fuera
  de este indicador (match difuso).

## 11. Segunda tanda (19-20/10/2026): lo que se encontró usando la app — HECHO

Todo lo de esta tanda está detallado en [hallazgos_solucionados.md](hallazgos_solucionados.md)
(segunda tanda, con identificadores propios: `WIN-`, `PLUG-`, `DATA-`, `UI-`, `PACK-`, `DEEZ-` y `DOC-`).

| # | Qué se encontró | Estado |
|---|-----------------|--------|
| 11 | El lanzador `py` abría una **ventana de Windows Terminal** en cada llamada a un plugin | ARREGLADO — `WIN-01` (`9d908dc`) |
| 12 | En un salto de vídeo la imagen se quedaba **pegada en el frame del sitio anterior** (3 s) | ARREGLADO — `WIN-03` + `WIN-04` (`734797e`) |
| 13 | El yt-dlp no venía en el instalador y **no se podía actualizar** bajo Program Files | ARREGLADO — `PLUG-02` (`400ebb1`, `d6d1b9f`) |
| 14 | El motor de Deezer **no arrancaba** desde la instalación (programa nombrado sin extensión) | ARREGLADO — `PLUG-05` + `PACK-02` (`e0f280a`, `1d9f8c9`, `d6d1b9f`) |
| 15 | El motor de Deezer fallaba al descargar: *Invalid CSRF token*, *no dio un token*, token que rota | ARREGLADO — `DEEZ-01..03` (plugins `d1ba1de`) |
| 16 | Las preferencias que añadió el original llegaban **en inglés**, y el texto de las filas no se traducía | ARREGLADO — `UI-02` (`5289290`) |
| 17 | La ventana **no abría a pantalla completa** (el ítem del menú no hacía nada) | ARREGLADO — `UI-01` (`b28cf41`) |
| 18 | El README no contaba esta versión y solo estaba en inglés | ARREGLADO — `DOC-01` (`975576a`, con `README.es.md`) |
| 19 | **Instalador de Windows** y **release en GitHub** | HECHO — `PACK-01` (`525689b`), release `v0.2.2` |

**Queda a decisión del usuario** (nada abierto por avería): los tres puntos de entonces —acortar
el salto de vídeo, publicar la release de plugins y quitar la línea `$excluded` del empaquetado—
siguen igual y están reunidos en **§16**.

## 12. Tercera tanda (21/10/2026): el browser - HECHO

Lo cerrado, con su commit:

| ID | Lo que se hizo | Commit |
|----|----------------|--------|
| BR-01 | Las pistas de nube entran en la Colección antes de sonar: el historial, el color de "reproducida" y las listas las nombran igual y la misma pista no se loguea dos veces | f64fe8f |
| BR-02 | Sección "Downloaded" en el árbol: lo que los plugins de nube descargan, como fuente real (se carga, se importa, se busca) | bf8a746 |
| BR-03 | "Not in the Collection" por fuente y botón para añadirlo todo de golpe | bf8a746 |
| BR-04 | Filtro de medio (ALL / AUDIO / VIDEO) y columna "FORMATO" (nota musical o película) | 0f92826 |
| BR-05 | "Devices" lista todos los volúmenes (no solo los de biblioteca de DJ) y cada uno lista sus carpetas de primer nivel, que se abren como fuente | 06bf868 |
| BR-06 | Carpetas fijadas (botón FAV, sección "Favorites", pin por etiqueta de volumen + ruta relativa) y casillas para ocultar las secciones | f61921c |
| BR-07 | Búsqueda unificada (Colección + índice de las fijadas + fuentes leídas) con columna ORIGIN, sin disparar las búsquedas en línea | 6b26d39 |

### Fase 5 — cerrada en la sexta tanda (22/10/2026)

| ID | Lo que se hizo | Commit |
|----|----------------|--------|
| F5-1 | Filtro por origen en la barra: ALL / COLLECTION / YOUTUBE / DEEZER (`Origin` en `src/app/browser/filter.rs`, el botón cicla en `ui/browser.slint`) | f7ead3d |
| F5-2 | Preview de vídeo en la columna PREVIEW: un fotograma en vez de la onda (`src/app/browser/strip.rs`) | 85131bd |
| F5-3 | Aviso de duplicado al importar: *"this track is already in the Collection"* | f7ead3d |
| F5-4 | La estrella de una carpeta fijada se pulsa desde su fila: fija y quita sin seleccionarla antes | f7ead3d, 432a1c5 |

El detalle está en [hallazgos_solucionados.md](hallazgos_solucionados.md), sexta tanda.

**El origen "Devices" se añadió después** (`3fe48c5`): `Origin` tiene `All`/`Collection`/`YouTube`/
`Deezer`/`Devices`, así que una pista de un USB o del disco local que no esté en la Colección también
se puede aislar con el filtro.

## 13. Cuarta tanda (21/10/2026): los volúmenes - HECHO

Lo cerrado:

| ID | Lo que se hizo | Commit |
|----|----------------|--------|
| BR-08 | Un disco duro USB aparece en "Devices": Windows responde `DRIVE_FIXED` a un disco externo y el escaneo solo aceptaba `DRIVE_REMOVABLE` (pendrive o tarjeta) | 2a7192a |
| BR-09 | El disco local se puede recorrer: el volumen del sistema (C:) se lista en la sección "Devices" como `C: (OS)` y sus carpetas de primer nivel se abren como fuentes de solo lectura | 2a7192a |
| BR-10 | Listar un volumen ya no lo barre entero: solo se lee el primer nivel (antes se recorría el disco entero buscando audio) y ahora se ven todas sus carpetas, no solo las que tienen audio | 2a7192a |
| BR-11 | El instalador de Windows se compila otra vez (NSIS 3.11) y sube a la release v0.3.1, ya publicada | f1a0901 |
| BR-12 | Las carpetas fijadas se reindexan cuando el volumen se reconecta (ya no hay que fijarlas otra vez para que la búsqueda las encuentre) | a50a361 |

Lo de la fase 5 se cerró en la sexta tanda (arriba).

## 14. Quinta tanda (21/10/2026): el navegador - HECHO

Lo cerrado:

| ID | Lo que se hizo | Commit |
|----|----------------|--------|
| BR-13 | Columna BITRATE en la tabla de tracks (kbps, a la derecha de FORMATO) | 38f1bf4 |
| BR-14 | Secciones del árbol plegables (History, Tags, Devices, Downloaded, Música Local, Música en Línea) | 38f1bf4 |
| BR-15 | History: renombrar sesión manteniendo fecha y eliminar sesión con clic derecho | 38f1bf4 |
| BR-16 | Related tracks: auto-ancla al primer track visible si no hay selección | 38f1bf4 |

## 15. Séptima tanda (22/10/2026): SYNC y memoria - HECHO

Lo cerrado:

| ID | Lo que se hizo | Commit |
|----|----------------|--------|
| SYNC-01 | SYNC toma los candados de tempo/fase juntos y se mantiene dentro de 1 ms | 6f43923 |
| MEM-01 | Decodificación directa a `Track16` (decks, preview y análisis en bloque) y huella en streaming, sin copia f32 de la pista entera | 695bcd3 |
| MEM-02 | Análisis genérico sobre `audio::Frames` (grid, waveform, tonalidad, voz, caché) para analizar un `Track16` sin copiarlo | 695bcd3 |
| MEM-03 | `mallopt` en glibc, perfil `pi`, `Dockerfile.pi`, `stems-gpu` opcional (STEMS en CPU en ARM Linux) y `dist` sin símbolos | 695bcd3 |

Notas:

- El puerto se hizo sin la maquinaria de cancelación/bulk del original: se conserva el análisis local y el refactor mínimo sobre `Frames`/`Track16`.
- `6f43923` y `695bcd3` ya están en `origin/main` (salieron en 0.4.0), no solo en local.
- El preview de vídeo por fotograma (F5-2) **sí se implementó** (`85131bd`) y sigue en el código; la nota anterior lo daba por descartado y no era exacta. Se queda como está salvo que se decida otra cosa.
- La versión, los instaladores y la release se tocan solo cuando se pidan.

## 16. Lo que queda abierto (revisado el 26/10/2026)

Revisado contra el código de `main` (`7ee52c8`, versión 0.4.1). **`hallazgos.md` está cerrado
entero**: los 30 hallazgos figuran resueltos en [hallazgos_solucionados.md](hallazgos_solucionados.md).
Lo que sigue es decisión, no avería:

1. ~~**Origen "Devices" en el filtro del navegador**~~ — **hecho** (`3fe48c5`): el chip cicla
   ALL / COLLECTION / YOUTUBE / DEEZER / DEVICES y `Origin::Devices` deja las pistas cuyo archivo
   está en un volumen montado o en un disco local (también las que sí están en la Colección).
2. ~~**Salto de vídeo: de ~460 ms a ~150 ms**~~ — **hecho en dos partes**: el core muestra el
   keyframe anterior mientras llega el frame exacto (ffprobe lista los keyframes una vez;
   `feature/video-seek`), y el plugin yt-dlp re-codifica con un keyframe por segundo lo que entra
   en la Colección (`-g 30` + `-force_key_frames`, rama `feature/video-keyframes` del repo de
   plugins). El `fetch` de la nube se deja sin re-codificar para no retrasar la primera reproducción.
3. **Release de `MyOpenFadeR-Plugins`** — publicar los binarios nativos (el motor de Deezer, los
   importadores, los efectos) para que GET PLUGINS pueda instalarlos: los repositorios por defecto
   ya son de `ronellcl`, pero el de plugins aún no tiene releases y la descarga del binario busca
   el zip en `releases/latest`. El plugin yt-dlp es un `script`, así que se instala desde el zip del
   repositorio y no necesita asset propio; el instalador de Windows ya lleva su binario. No hace
   falta adjuntarlo a la release.
4. ~~**Empaquetado de Windows**~~ — **ya hecho** (`5289290`): no queda línea `$excluded`;
   `build-installer.ps1` compila el motor de Deezer y lo mete en `deezer\bin`. Solo falta compilar
   el instalador en Windows cuando se pida.
5. **Diálogos nativos en Windows** — el selector de archivos de la app y el diálogo del ARL del
   plugin deezer usan PowerShell con `CREATE_NO_WINDOW` porque sin consola no muestran nada; en
   Windows 11 puede asomar la pestaña del terminal un momento. Pasarlos a un diálogo nativo (FFI)
   lo quita del todo (ver §1).

**Deezer y yt-dlp: por qué fallaban** — eran dos averías distintas. `start.sh` compilaba los
plugins sin el `[patch]` al `sdk/` de este checkout (que sí escriben `install.sh` y la imagen de
la Pi), así que el motor de Deezer y los importadores no compilaban contra el tag viejo; ahora
`start.sh` escribe el patch y el motor compila. Y el binario de yt-dlp que el plugin se descarga
quedaba sin permiso de ejecución, con lo que toda llamada moría con *Permission denied*; el
plugin ahora le pone `+x` al bajarlo y al encontrarlo, lo que también arregla las instalaciones
que ya tenían el binario mal. Y el descompresor de la app tiraba el modo de los zips: un
`bin/deezer-engine` que la release trae en 755 quedaba en 644 y fallaba igual; ahora conserva el
modo y, si un programa de plugin ya instalado no lo tiene, se le devuelve el `+x` antes de
ejecutarlo. El catálogo, además, solo pedía el asset de release para las librerías: un plugin
`command` como Deezer se instalaba con el `plugin.json` pero sin `bin/deezer-engine`; ahora el
programa cuenta como archivo nativo y el asset se descarga.

**Repositorios por defecto repuntados** (`chore/repo-urls`): `src/catalog/list.rs`, `install.sh`,
`start.sh`, `Dockerfile.pi`, el About y la documentación usan ahora
`ronellcl/MyOpenFadeR-Plugins`, `ronellcl/MyOpenFadeR-Themes` y `ronellcl/MyOpenFadeR-Map`. Los
nombres reales de GitHub llevan mayúsculas y plural (`MyOpenFadeR-plugin` no existe). Los enlaces
de atribución al OpenFadeR original y el `[patch]` del SDK (`OpenRsG/OpenFadeR`, que es lo que fija
el `Cargo.toml` del repo de plugins) se quedan como están.

## 17. Plan de backport selectivo desde upstream (26/10/2026)

> **Estado:** planificado y en validación. **No se ha modificado código, no hay rama nueva y no
> hay commits de esta fase.**
>
> **Base local:** `main` = `ddcc4a0` (0.4.1; 15 commits por delante de `origin/main`).
> **Upstream de referencia:** `upstream/main` = `9ac712c` (0.2.5 más trabajo posterior sin release).
> **Decisión:** backport por fases; **nunca** un merge completo. El merge produce 306 archivos en
> conflicto y upstream no es un superconjunto: elimina funciones propias del fork.

### 17.1 Reglas de la operación

- Conservar, sin excepciones, las funciones propias: vídeo y seek, nube/yt-dlp/Deezer, catálogo con
  `SHA256SUMS`, seguridad de plugins, ffmpeg/ffprobe estáticos, instalador de Windows,
  Devices/pinned/downloaded, i18n español y escenario/temas VDJ.
- No copiar commits enteros cuando mezclan una corrección con un rediseño. Portar por bloques y
  comparar contenido, no hashes.
- Una rama por fase. Merge a `main` solo después de compilar en **release**, pasar tests y smoke
  test. Debug solo para compilar y ejecutar tests, nunca para la GUI.
- No tocar los commits ya fusionados `5e2f564` y `ddcc4a0`.
- No hacer push, tag, release ni cambio de versión sin aprobación explícita.
- Todo trabajo de Rust y Slint se apoya en los skills `rust-skills` y `slint`, y en subagentes de
  análisis/revisión. Los subagentes de planificación trabajan en modo read-only.
- Al cerrar cada fase, actualizar este apartado con commits, pruebas y estado; un solo commit
  grande al final de la fase, después de verificar.

### 17.2 Fases

| Fase | Objetivo | Estado | Cierre |
|---|---|---|---|
| 0 | Preparar rama y baseline | Pendiente | `main` limpio, build release y tests de partida verdes |
| 1 | Correcciones 0.2.5 de audio/UI | En validación | Tests + build release + smoke + revisión manual |
| 2 | Endurecer CI/empaquetado | Pendiente | Workflow valida el paquete antes de publicarlo |
| 3 | Cimientos store/device/plugin | Pendiente | Migración v17→nueva con backup/rollback probado |
| 4 | Browser/library/device features | Pendiente | Funciones nuevas sin perder fuentes ni i18n del fork |
| 5 | Stage/Studio/temas | Diferida | Solo con diseño y aprobación separados |

### 17.3 Fase 1 — correcciones 0.2.5

**Estado:** informes Rust y Slint recibidos el 26/10/2026. El de Rust fija C1–C7; el de Slint
confirma que `c293a3c` y `121d080` no tocan UI, que `10e977d` es C1 condicionado a reproducir el
stack, y que `1d6c7ba` tiene una mitad visual separable (4a) y otra de comportamiento (4b) que no
se aprueba en fase 1.

Objetivo: recuperar correcciones de comportamiento sin arrastrar el rediseño de browser, Stage,
Studio, temas ni la numeración nueva del store.

#### 17.3.1 Veredicto por commit

| Commit upstream | Veredicto | Detalle |
|---|---|---|
| `4ffd17a` | **Portar 3,5 de 5 partes** | Sí: SYNC audible, Beat FX CH/MST/TAP, stems conservan EQ y soft takeover. **No**: “Reload Tag conserva ediciones”, porque el fork no tiene overrides de texto. El informe Slint sí ve adaptable la variante “Reload Tag (discard edits)” con `collection_put` + `Store::action`, sin migración, como C8 opcional |
| `121d080` | **No cherry-pick; dividir** | El núcleo de motor/deck/SYNC va ahora como C2/C3. Browser, store, library, sampler/BEAT LAB, MIDI layout, automix y loudness quedan para fase 2/3. Confirmado: no toca UI |
| `c293a3c` | **No aplica** | El fork no tiene `src/audio/fx/prepare.rs`, ni `Exchange`/`Handoff`, ni el hilo `fx-prepare`; las etapas se construyen en el hilo de control. Portarlo exigiría importar el rediseño de FX de upstream. Confirmado: no toca UI |
| `1d6c7ba` | **Sí hot cues; 4a visual opcional; no 4b** | Sí: `seek_on_beat`, fase escrita en el callback, `Plan::Beat` y tests de spam. El informe Slint separa 4a (visual: `DeckState.slots`, `Theme.deck-number`, badges, con defaults neutros y sin cambiar comportamiento) de 4b (Rust de slots y DECK 3/4). 4a solo si se aprueba; **no** entran `deck/slots.rs`, `ui/studio/*`, `ui/touch*` ni la vista DECK 3/4, que choca con la arquitectura 4-deck/VDJ del fork |
| `10e977d` | **Portar si reproduce** | El fork tiene más overlays que upstream (`TrackEdit`, `PluginOverlays`, `DeviceOverlay`, `OfferBanner`, `ToastView`, `TipOverlay`), así que el riesgo de constructores anidados es mayor. Reproducir primero el stack con `ulimit -s 1024`; si no reproduce, no tocar `ui/app.slint` y no añadir el `ulimit` al smoke. Si reproduce, separar árboles y mantener el smoke con 1 MiB |

#### 17.3.2 Bloques de fase 1

| Bloque | Qué se hace | Archivos principales | Depende de | Riesgo |
|---|---|---|---|---|
| **C1** | Reproducir con 1 MiB de stack; solo si falla, separar vistas/barra/overlays con `if true:` (`10e977d`) | `ui/app.slint`, `.github/smoke.sh`, `docs/code-structure.md` | — | Bajo |
| **C5** | SYNC: solo el deck que suena lidera; un MASTER fijado se sigue; KEY SYNC no cambia | `src/app/deck/sync/master.rs`, `sync/mod.rs` | — | Bajo |
| **C4** | Beat FX con fuente CH/MST/TAP; stems conservan EQ y no cambian el strip a STEM ISO | `src/app/fx/`, `src/app/deck/stems/mod.rs`, `src/app/mixer/`, `src/app/prefs/`, `ui/fx.slint`, `ui/mixer.slint` | — | Medio |
| **C2** | Primitivas de transporte/reloj/loop de 0.2.5 (`latency_ms`, `steady`, `written_ms`, loops en 256 pasos, slip, catch-up consciente del loop) | `src/audio/player/{mod,clock,voice,tests}.rs`, `src/audio/transport.rs`, `src/app/deck/loops.rs` (solo cast a `f64`) | — | Medio |
| **C3** | Timing de deck, loops, quantize, cues, slicer, sequencer y SYNC sobre C2 | `src/app/deck/*`, `src/app/deck/sync/*`, `src/app/midi/actions/*`, `midi/input.rs` (release de `Kind::Deck`) | C2 | Alto |
| **C6** | Soft takeover MIDI para knobs/faders absolutos (`midi/pickup.rs`, pref, toast) | `src/app/midi/`, `src/app/prefs/controller/`, `src/app/toast.rs`, `src/app/mixer/mod.rs` | C4 (lectores `knob_at`/`iso_at`) | Alto, pero autocontenido |
| **C7** | Hot cues exactos al pulsar rápido (`seek_on_beat`) | `src/audio/player/{mod,voice,tests}.rs`, `audio/player/hotcue_tests.rs`, `audio/transport.rs`, `deck/quantize.rs` | C2 + C3 | Medio |
| **C8** | “Reload Tag (discard edits)” sin overrides de texto: refresca análisis/BPM/key desde el archivo y descarta el texto editado, dentro de `Store::action` | `src/app/library/track.rs`, `ui/app.slint`, `ui/browser-table.slint`, `es.po` | — | Bajo, opcional |

Orden práctico: **C1 → C5 → C4 → C2 → C3**. C6 y C7 son “fase 1 tardía”: se pueden cerrar en
otra rama o en un segundo commit después de que el núcleo esté verificado. C8 es opcional y solo
si se acepta la adaptación sin migración. El núcleo C1–C5 puede entregarse como un commit grande;
no mezclar con store, browser ni `ui/studio`.

#### 17.3.3 Qué no se toca en fase 1

- `src/store/mod.rs` (migraciones), `src/store/{track,overlay}.rs` (overrides de texto) y
  `src/app/browser/{sources,sidebar,list,table,wire,preview}.rs`.
- `src/audio/fx/prepare.rs` (no existe), el handoff/fading de FX y la migración v17 de gain.
- `src/app/deck/slots.rs`, `DeckState.slots`, `ui/studio/*`, `ui/touch*` y la vista DECK 3/4.
- Los `.po`/`.pot` masivos y `ui/touch-actions.slint` (no existe en el fork). Las cadenas nuevas
  van a `i18n::ES` o a literales ingleses según el caso.

#### 17.3.4 Pruebas y aceptación

- C1: build release y smoke Linux con `ulimit -s 1024`; abrir manualmente `TrackEdit`,
  `PluginOverlays`, `DeviceOverlay`, `Prefs` y `Toast` y comprobar z-order.
- C5: A suena a 128 y B está idle a 120; SYNC en A no cambia su tempo y A pasa a master; MASTER
  fijado en B se sigue; KEY SYNC sigue emparejando con B idle.
- C4: un unit en CH2 sigue al deck B; MST sigue al master; TAP manda; sin deck conocido conserva
  el último tempo y luego 120; un stem mutea sin cambiar el strip; el EQ sigue cortado en STEM
  ISO y al volver.
- C2: loop de 1 beat a 128 BPM sin deriva > 0,05 ms en 1000 pasadas; PLAY desde parado rampa y un
  hot cue no; latencia y `written_ms` correctos.
- C3: LOOP OUT con el playhead 20 ms pasado el out mantiene la fase; beat jump/loop move no
  saltan de beat; borrar todos los cues y recargar no los resucita; doble instantáneo en fase.
- C6: un knob espera hasta cruzar el valor en pantalla; toast de dirección; con el pref apagado
  vuelve el salto; botones, relativos y jog no cambian.
- C7: 500 pulsaciones con Q a distintas latencias/tempos/key lock no caen un beat fuera; los pads
  de loop caen dentro del loop en la fase escrita; el retorno de slip no resucita.
- Global: `cargo test --release` verde, build release y smoke; vídeo, nube, Devices y español sin
  regresiones; sin dependencias nuevas injustificadas.

Rollback: revertir la rama de fase 1. Al no haber migración de store, no hace falta restaurar la
biblioteca; aun así, hacer copia de `store.db` antes de las pruebas manuales. Regla de oro: si un
parche necesita `src/store/mod.rs`, el browser o `ui/studio`, se parte y se manda a fase 2/3.

#### 17.3.5 Contratos UI ↔ Rust del informe Slint

- `ui/fx.slint`: `FxUnit.master-tempo`; `FxState.tempo-source(int)`; el texto `src` muestra
  CH/MST/TAP. Sin `@tr` nuevo en `fx.slint`, `mixer.slint` ni `browser-table.slint`.
- `ui/mixer.slint`: `iso-part(int, int, float)`; los botones de parte mueven el nivel del stem sin
  cambiar el strip del canal a STEM ISO.
- Solo 4a: `DeckState.slots` con default `[0, 1, 2, 3]`, `Theme.deck-number` como propiedad `in`
  fuera de los grupos y `DeckDetail.badge-size` (14 px en EXPORT). No convertir `deck-number` en
  token de skin ni tocar `ui/build.rs`/`ui/lib.rs`.
- Si algún día entra 4b: `DeckState.count` sigue siendo `shown()`, no `active()`; al alcanzar C/D
  el layout de 2 decks no debe saltar a 4.
- C1: grupo A con `HelpOverlay`, `AboutOverlay`, `UpdateOverlay`, `WelcomeOverlay` y
  `StemsModelCard`; grupo B con `MidiOverlay`, `PrefsOverlay`, `TrackEdit`, `LibraryOverlays`,
  `PluginOverlays`, `DeviceOverlay`, `OfferBanner`, `ToastView` y `TipOverlay`. Dejar `DragGhost`
  y `FileDropOverlay` fuera de los grupos si el smoke aún falla.
- No importar los `.po`/`.pot` de upstream; añadir solo cadenas nuevas. No crear `ui/touch*` ni
  `ui/studio/*` ni tocar la geometría del escenario VDJ.
- Matriz visual mínima: 2H/2V/4H/4V, Classic/VDJ, vídeo on/off, tres skins y inglés/español;
  abrir Track, FX, mixer con STEM ISO, menú contextual, EXPORT de 2 players y split.

### 17.4 Fase 2 — CI y empaquetado

Adoptar de upstream:

- `timeout-minutes: 120` en los jobs de empaquetado (`a3b1e72`).
- Subir el artefacto **después** de que la app empaquetada pase el smoke test, con
  `if-no-files-found: error`.
- Smoke test del archivo desempaquetado en Linux, Windows y macOS, no solo del binario de
  `target/`.

Mantener del fork (no portar las eliminaciones de upstream):

- `tools/` estáticos, `.github/fetch_tools.py` y su uso en `release.yml`.
- El parche de `extension/plugin` al SDK de este checkout.
- Nombres `MyOpenFadeR-*`, app de macOS `MyOpenFadeR.app`, instalador NSIS de Windows.
- `src/main.rs` con `bundled_tools_first_in_path()`.

Opcional, solo si se aprueba:

- Job de Raspberry Pi con el `Dockerfile.pi` local y el perfil `pi` (`3020aec` en upstream). El
  Dockerfile ya existe en el fork; falta verificar el workflow y el archivo resultante.
- Medición de arranque con `SMOKE_TRACKS`/`big_library`. El fork **no** tiene
  `examples/big_library.rs`; habría que portarlo antes. No es necesario para cerrar fase 2.

Updater: **diferido**. El `src/update/` de upstream hardcodea `OpenRsG/OpenFadeR` y los nombres
`openfader-*`; copiarlo podría instalar upstream sobre el fork. Si se hace más adelante, adaptar
`REPO` a `ronellcl/MyOpenFadeR`, los sufijos a `MyOpenFadeR-*`, negarse a actualizar builds de
desarrollo/fuente, verificar SHA-256 y conservar backup/rollback. `5cd2834` mezcla updater, aviso
de análisis, búsquedas guardadas y backup; no portarlo entero.

### 17.5 Fase 3 — cimientos store/device/plugin

**Estado:** informe del subagente recibido el 26/10/2026. Confirma dos bloqueos reales y propone
las subfases F0–F5. No se ha editado código.

#### 17.5.1 Bloqueo 1: colisión de migraciones

Las dos ramas comparten `v1..v15` con la base de merge `66d2698`. A partir de ahí:

| Versión | Fork `ddcc4a0` | Upstream `9ac712c` |
|---|---|---|
| v16 | `collection.plugin/source/remote_id` (nube) | `track.title/artist/album/genre/label` |
| v17 | tabla `folder_index` | `UPDATE gain SET analysed_db = NULL` |
| v18 | — | `health_ignore`, `health_failure` |
| v19 | — | `stage`, `inbox_move`, `inbox_seen`, `collection.stage` |
| v20 | — | `artwork` |
| v21 | — | `device_backup.auto` |
| v22 | — | `saved_search` |
| v23 | — | `file_print`, `import_undo` |

Decisión propuesta:

- **No renumerar** las migraciones del fork. Se conservan `v1..v17` y se añaden las de upstream
  `v18..v23` **tal cual**, para que el número signifique lo mismo en ambos árboles a partir de v18.
- El store del fork que salta de v17 a v23 no ejecuta las migraciones v16/v17 de upstream. Por eso
  hace falta una pasada **idempotente** `reconcile()` al final de `migrate()` que solo cree objetos
  que falten con `CREATE ... IF NOT EXISTS`, `ALTER ... ADD COLUMN` guardado e índices
  `IF NOT EXISTS`; nunca `DROP`, nunca `UPDATE` de datos de usuario y nunca renumerar.
- El `UPDATE gain SET analysed_db = NULL` de upstream v17 **no se repite en cada apertura**: se
  marca una sola vez con `store_note('gain_reset:v17')` y se activa solo cuando llegue el analizador
  con límite por pico (F2.4). Antes de eso, resetear perdería análisis sin ganancia.
- El store fusionado en v23 **no es la unión de los dos v16**; el reconciliador cubre las columnas
  y tablas que upstream habría creado.

#### 17.5.2 Bloqueo 2: identidad del store

Hoy `PRAGMA user_version` no distingue un store v17 del fork de uno v17 de upstream. Tres rutas
comparan solo la versión: `Store::open`/`migrate`, `Store::restore`/`copy_tables_from` y el futuro
`roll_back_store`. Si se restaura un backup upstream v17–v23 en el fork, la versión pasa el
control, `replace_tables` vacía todas las tablas y solo rellena las que existen en el backup:
`folder_index` se pierde y las columnas cloud quedan a NULL, con un aviso de éxito. **Hay que
cerrar este agujero antes de añadir cualquier tabla nueva.**

Plan propuesto:

- Nuevo `src/store/identity.rs` con `Kind::{Empty, Fork, Foreign}`.
- `PRAGMA application_id` con `0x4D4F4644` (`MOFD`) en stores nuevos, y back-stamp en los stores
  del fork existentes.
- Sonda de firma para archivos sin marcar: fork ⇒ `collection.plugin` o `folder_index`; upstream ⇒
  `track.title`, `health_failure`, `stage` o `saved_search`.
- `check_openable()` antes de cualquier escritura; un store/backup `Foreign` se rechaza abriendo
  **read-only** y con un mensaje accionable. Un conversor de stores es un proyecto aparte.
- El zip de backup lleva un miembro `store.id`; un backup extranjero se rechaza antes de adjuntar
  la base.
- En `replace_tables`, conservar `LOCAL_TABLES = ["folder_index", "store_note"]`; si al backup le
  falta una tabla que tiene el store actual, **fallar cerrado** en vez de vaciarla. Después de
  restaurar: `reconcile()` + `PRAGMA foreign_key_check`.
- Documentar el mapa de versiones y la regla de solo añadir en `docs/store-schema.md`.

#### 17.5.3 Subfases F0–F5

| Subfase | Contenido | Cierre |
|---|---|---|
| F0 | Identidad, `reconcile()`, migraciones v18–v23, `backup_auto`, rollback y preservación de tablas locales | Store fork v17→v23 sin pérdida; segundo arranque sin cambios; backup upstream rechazado; restaurar/rollback conserva `folder_index` y columnas cloud |
| F1 | Primitivas sin UI: `paths::{free_name,move_file,move_files,move_back}`, journal de movimientos, `Store::system` para historial, health, artwork, saved_search, print, import_undo | Todo probado headless; sin editar `src/app/**` ni `ui/**` |
| F2 | Núcleo de library: inbox/stage, artwork, import report, loudness con límite por pico (activa el reset de gain), print/relocate | Funciona con el andamiaje de UI actual; las pistas cloud se saltan en print/health/artwork/inbox |
| F3 | Device + SDK: `commit.rs` (journal de escritura), `Provider::{check,fix,players}`, plan/check/transfer/undo/format; adaptar `device/mod.rs`, no reemplazarlo | `cargo check --all-targets` en Linux; revisión Windows/macOS; formateo solo tras dry-run y confirmación |
| F4 | Cableado de app: device, saved search, artwork, health, undo, sobre la UI y el i18n del fork | Nada de `tr!`/`.po` de upstream; cadenas nuevas en la tabla español/inglés del fork |
| F5 | Transfer/undo end-to-end: commit journal + backup + import undo; recuperación tras corte | Matar el proceso a mitad de escritura y recuperar biblioteca y dispositivo; poda de backups observada |

Orden de bloques/commits: (1) identidad + reconcile, (2) append v18–v23, (3) backup/rollback/
tablas locales. Estos tres van **antes** de cualquier otro trabajo de store; el bloque 1 va antes
de que aparezca cualquier tabla nueva. Después: paths/journal, health/artwork/saved_search/print/
undo, inbox/stage, artwork+plugin, SDK commit, device plan/check/transfer/undo, format gated, app
wiring, transfer+undo.

#### 17.5.4 No copiar en bloque

| Archivo del fork | Cambio upstream | Por qué rompe el fork |
|---|---|---|
| `src/device/{mod,linux,macos,windows}.rs` | tabla de particiones, FUSE, formateo | Elimina `top_folders`, `os_folder`, `Local`, `local_disks`, el escaneo de discos fijos y el filtro del disco del sistema |
| `src/app/browser/{sources,sidebar,list,table,wire,preview}.rs` | rework del árbol y la tabla | Elimina Favorites/pinned, Downloaded, OnlinePlugin, SystemFolder, LocalDisk y DeviceFolder |
| `src/store/{track,overlay}.rs` | modelo de overrides de texto | Quita `set_title`/`set_artist`/`set_collection_field` y rompe Edit Details |
| `src/plugin/{install,mod}.rs`, `src/catalog/*` | hook de device | Pierde SHA256SUMS, `folder_name -> Option<String>`, exec-bit, probes acotados, `concur::Gate` |
| `src/library/import.rs`, `src/app/library/undo.rs` | solapamiento | El fork tiene sus propias ediciones; solo cherry-pick de hunks |
| `src/app/i18n.rs`, `src/app/mod.rs`, `ui/app.slint`, `ui/browser.slint`, `ui/device*.slint` | `tr!`/`.po`, locale, theme profile, Stage/Studio | Quita español, skins/temas, track-edit y UI de vídeo |
| `src/store/backup.rs` | nueva capa de backup | Conservar `replace_tables` del fork y añadir alrededor, según 17.5.2 |

#### 17.5.5 Pruebas y aceptación

| Caso | Esperado |
|---|---|
| Store real del fork v17 → v23 | `folder_index` y columnas cloud intactas; `analysed_db` sin anular; `stage` NULL |
| Abrir dos veces | `user_version` igual, sin reescritura, una sola nota |
| Store upstream v17 o v23 | Rechazado read-only; hash y mtime intactos |
| Store fork v23 abierto por el binario `ddcc4a0` | “más nuevo que este build”, datos intactos |
| Backup fork → fork | Restaura; `folder_index` y cloud sobreviven |
| Backup upstream → fork | Rechazado antes de `ATTACH` |
| `backup_auto` → `roll_back_store` | Identidad y versión restauradas; reabre; poda por dispositivo |
| Migración interrumpida | Reabrir converge; ninguna migración a medias |
| Transfer con `fail_at` | Recupera dispositivo y biblioteca; backup previo presente |

Aceptación: `cargo test` verde; `cargo check --all-targets` verde en Linux y revisión de targets
Windows/macOS; las funciones del fork (pinned/local disks, cloud, español, skins, track-edit,
vídeo) siguen pasando sus tests; `docs/store-schema.md` existe.

#### 17.5.6 Riesgos específicos

| Riesgo | Mitigación |
|---|---|
| Restaurar store upstream borra datos del fork | Identidad + rechazo read-only + `LOCAL_TABLES` + fail closed |
| Reset de gain destruye análisis | Marcador, backup y solo tras F2.4 |
| Formateo deja un stick inservible | Dry-run, backup, confirmación explícita, verificación read-only y gate por SO |
| Cloud se cuela en print/health/artwork/transfer | Guard compartido `is_remote()` y test por función |
| ABI de plugins rota | Mantener `LIBRARY_ABI = 2`; peticiones desconocidas explícitas y matriz de pruebas |
| Reconciliador se convierte en migración oculta | Contrato en `docs/store-schema.md` y test de segunda apertura |

### 17.6 Fase 4 — browser, library y device features

Solo después de fase 3. Candidatos: multiselección y arrastre a USB, `Transfer to`, Auto Cue
(`af6b6e3`), búsquedas guardadas, library health/inbox/artwork, checker/formateo/preparación/
verificación/transferencia/undo de USB (`3800b64`, `1ed58d5`, `3cace47`, `66524c5`, `929d7f9`).

Guardrails: no reemplazar en bloque `src/app/browser/sources.rs`, `sidebar.rs` ni el árbol; el
fork debe seguir mostrando Downloaded/Pinned/Devices/LocalDisk/SystemFolder/OnlinePlugin/
DeviceFolder, el filtro por origen y el español. La UI se revisa con el skill `slint` y el
subagente de UI antes de tocar `ui/`.

### 17.7 Fase 5 — Stage/Studio/temas (diferida)

Upstream añade Stage/Studio y el motor de temas (`1727231`, `bc787f3`, `6c9a070`). Choca con el
escenario VDJ, `stage-layout`, `h-video`, `w-deck-video` y los temas propios. No se fusiona en
esta operación; requiere un diseño aparte y aprobación explícita.

### 17.8 Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Merge completo elimina funciones del fork | Backport manual por commits/bloques; nunca `merge upstream/main` |
| Migración de store pisa v17 o corrompe datos | Numeración propia, backup previo, tests de migración y rollback |
| Updater instala OpenFadeR upstream | No copiarlo; adaptar repo/artefactos o dejarlo fuera |
| Traducciones upstream pisan el español | No portar `.po` masivos; añadir solo claves nuevas |
| Rediseño de browser/Stage rompe VDJ o Devices | Fases 3–5 separadas; revisión Slint y pruebas visuales |
| Empaquetado pierde ffmpeg/plugins del fork | Conservar `fetch_tools.py`, `tools/`, parche de plugins e instalador |
| Regresión silenciosa en audio | Tests de motor + release build + smoke + prueba manual de SYNC/FX/decks |

### 17.9 Seguimiento

| Fecha | Paso | Estado |
|---|---|---|
| 26/10/2026 | Plan de backport documentado en este apartado | Hecho |
| 26/10/2026 | Informe store/device/plugin recibido; F0–F5 incorporados a §17.5 | Hecho |
| 26/10/2026 | Análisis Rust fase 1 y Slint UI | Hecho |
| 26/10/2026 | Informes reconciliados; §17.3.5 fija contratos UI/Rust y §17.3.1 ajusta el veredicto | Hecho |
| 26/10/2026 | F0a: identidad del store, rechazo read-only y `LOCAL_TABLES` | En curso |
| Pendiente | Aprobación del alcance de fase 1 y de F0 como precondición de fase 3 | Pendiente |
| Pendiente | Rama `backport/fase-1`, implementación y pruebas | Pendiente |
| Pendiente | Commit grande de fase 1 y merge a `main` | Pendiente |

**Hallazgo crítico en corrección (F0a):** hoy un backup de store de upstream v17–v23 puede
restaurarse en el fork y borrar `folder_index` y las columnas cloud sin avisar. No se debe añadir
ninguna tabla nueva antes de F0 (identidad + rechazo read-only + `LOCAL_TABLES`). F0a cubre solo
la identidad, el rechazo read-only, la marca `store.id` del zip y la preservación de tablas
locales; `reconcile()`, v18–v23, `backup_auto` y el rollback quedan en F0b/F0c.

Decisiones abiertas:

1. Confirmar si fase 1 incluye `10e977d` solo en caso de reproducir el problema de stack en el
   fork, y si `c293a3c` entra junto a las correcciones de FX.
2. Decidir si el endurecimiento de CI va en fase 1 o fase 2. Recomendación: fase 2.
3. Decidir más adelante si se quiere updater adaptado al fork; no se implementa sin aprobación.
4. Confirmar que no se tocará versión, tags, releases ni push durante las fases.
5. Aprobar F0 (store: identidad, reconcile, v18–v23, backup/rollback) como bloque separado y
   precondición de todas las funciones nuevas de biblioteca/dispositivo.
