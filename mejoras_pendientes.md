# Mejoras y arreglos pendientes

Notas de trabajo (no forman parte del build). Orden aproximado de prioridad; se irán
haciendo de a uno, compilando en release al cerrar cada tanda.

> Recordatorio: la app **solo** debe ejecutarse en **release** (`start.sh` línea 14:
> `cargo run --release`). El build debug sirve para compilar/`cargo test`, no para correr la GUI.

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

**Queda a decisión del usuario** (nada abierto por avería):

- Acortar más el salto de vídeo: el motor nativo de Deezer ya está en `main` de `plugins-repo` y
  viaja en el instalador; si el motor se considera terminado, la línea `$excluded` del script de
  empaquetado puede desaparecer (ahora no excluye nada).
- El salto de vídeo se puede bajar de ~460 ms a ~150 ms con dos cambios posibles, ya medidos:
  re-codificar el vídeo descargado con keyframes cada segundo (`-g 30`, a costa de una descarga más
  lenta) o hacer el salto en dos fases (el keyframe al instante, el frame exacto después).
- Publicar el yt-dlp 1.2.4 y el motor de Deezer en el repositorio `MyOpenFadeR-Plugins` (el
  `main` ya los tiene; falta decide si se abre una *release* con el binario de yt-dlp adjunto).
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

### Lo que queda (de la fase 5 del plan)

- Filtro por origen en la barra (All / Colección / YouTube / Deezer / Dispositivos).
- Preview de vídeo en la columna PREVIEW (un fotograma en vez de la onda).
- Aviso de duplicado al importar ("esta pista ya está en la Colección").
- La estrella de la fila fijable con el clic (hoy se fija con el botón FAV y la estrella es solo la marca).
- Reindexar el índice de las fijadas cuando el volumen se reconecta (hoy se indexa al fijar).

## 13. Cuarta tanda (21/10/2026): los volúmenes - HECHO

Lo cerrado:

| ID | Lo que se hizo |
|----|----------------|
| BR-08 | Un disco duro USB aparece en "Devices": Windows responde `DRIVE_FIXED` a un disco externo y el escaneo solo aceptaba `DRIVE_REMOVABLE` (pendrive o tarjeta) |
| BR-09 | El disco local se puede recorrer: el volumen del sistema (C:) se lista en la sección "Devices" como `C: (OS)` y sus carpetas de primer nivel se abren como fuentes de solo lectura |
| BR-10 | Listar un volumen ya no lo barre entero: solo se lee el primer nivel (antes se recorría el disco entero buscando audio) y ahora se ven todas sus carpetas, no solo las que tienen audio |

Sigue pendiente lo de la fase 5 (arriba) y el instalador de Windows (NSIS no está disponible en esta máquina: la
release v0.2.3 queda como borrador).