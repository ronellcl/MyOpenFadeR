# Plan de implementación del navegador

> Documento de planificación y seguimiento. La implementación está fusionada en `browser` (commit `726019b` + `6fffd7d`, ff-merge desde `78ebec0`).
> `main` = `74e790a`; release `v0.4.2` publicada. Sin push, tag ni release sin autorización.

## Estado de implementación

- [x] Selección múltiple de filas con Ctrl/Cmd y Shift.
- [x] Resaltado de las filas seleccionadas y contador de selección.
- [x] Borrado por lote de entradas de playlist, sin tocar los archivos del disco.
- [x] «Add to Tag List» y «Analyze track» aplicados al conjunto seleccionado.
- [x] Menú contextual del árbol con favorito, analizar y refrescar carpeta.
- [x] Favorito de subcarpetas anidadas.
- [x] Análisis de carpeta en segundo plano, saltando las pistas ya analizadas.
- [x] Versión del analizador separada del formato de caché, con sidecar `.ver`.
- [x] Reanálisis on demand al cargar en un deck una pista con análisis de versión anterior.
- [x] Popup único al iniciar cuando cambia la versión del analizador (una sola vez por versión; botones Reanalizar ahora / Más tarde; sin reanálisis masivo automático).
- [x] Aplicación diferida del análisis si el deck está reproduciendo (nunca cambiar el grid en caliente; pending aplicado al pausar o siguiente tick).
- [x] Batch add a playlist y palette con múltiples pistas seleccionadas (evita duplicados, conserva orden).
- [x] Pruebas completas, commit y fusión a `browser`.

> Rama de referencia: `browser` = `726019b` + `6fffd7d`; base previa `78ebec0`.
> `main` = `74e790a`, release `v0.4.2` publicada.
> Sin push, tag ni release sin autorización.

## Alcance

Este plan cubre seis funciones relacionadas con el navegador:

1. Selección múltiple en playlists y borrado de entradas.
2. Selección múltiple en carpetas y acciones por lote.
3. Acciones de carpeta desde el árbol izquierdo.
4. Análisis persistente de una carpeta completa.
5. Versión del analizador, aviso al iniciar y reanálisis on demand.
6. Aplicación segura del análisis sin romper un deck en reproducción.

Las funciones 1 y 2 comparten el mismo motor de selección, por lo que se implementan juntas.

## Restricciones permanentes

- Backport selectivo; nunca merge completo de upstream.
- Preservar video/seek, yt-dlp/Deezer, catálogo SHA, plugins, ffmpeg estático,
  instalador Windows, Devices/pinned/downloaded, i18n español y VDJ themes.
- No `cargo fmt --all`.
- No tocar `Folder::tracks`, el esquema del store, plugins, i18n, vídeo ni
  `tests/video.rs`.
- Mantener exactamente los 3 warnings preexistentes:
  - `PluginSearch`/`PluginDownloads` field 1 never read en
    `src/app/browser/mod.rs:180/182`.
  - `audio_wave` en `src/app/render/detail.rs:98`.
- Falla preexistente fuera de alcance:
  `tests/video.rs::frames_follow_the_playhead_through_seeks_loops_and_the_end`.
- Las cadenas nuevas siguen el mecanismo actual: `@tr` para menús traducibles;
  el resto mantiene el estilo literal del browser.

---

## Base común: selección múltiple de filas

No conviene implementar dos selecciones distintas para playlists y carpetas.
La tabla es la misma.

### Modelo

- Mantener `View.selected: Option<Track>` como pista principal para el inspector.
- Añadir `View.picked: BTreeSet<usize>` con las **filas visibles** seleccionadas.
- Añadir `View.anchor: Option<usize>` para selección por intervalo con Shift.
- Publicar la selección en `BrowserState.selected-rows`, o mediante una celda
  oculta `SELECTED` si Slint no permite consultar el array con comodidad.
- Actualizar solo las filas afectadas con `set_row_data`; no reconstruir 60.000
  filas al cambiar la selección.

### Interacción

- Clic normal: selecciona una fila, limpia el resto y actualiza el inspector.
- Ctrl/Cmd + clic: añade o quita una fila.
- Shift + clic: selecciona el intervalo entre `anchor` y la fila pulsada.
- Clic derecho sobre una fila ya seleccionada: conserva la selección.
- Clic derecho sobre una fila no seleccionada: selecciona solo esa fila y abre
  el menú.
- Escape: limpia la selección. Ctrl+A: selecciona todo lo visible, opcional.
- Al cambiar de lista, filtro, búsqueda u orden: limpiar selección y ancla para
  evitar índices obsoletos.

### Slint

- `PointerEvent` ya expone `modifiers.control` y `modifiers.shift`; no hace falta
  rastrear teclas globalmente.
- Modificar `ui/browser-table.slint` para que el clic izquierdo y derecho pasen
  los modificadores a un callback nuevo, conservando `track-selected` para el
  inspector.
- Añadir resaltado visual de fila seleccionada sin perder el resaltado de
  “cargada en deck”.

### Rust

- `View::pick(app, row, ctrl, shift)` centraliza la lógica.
- `View::selection_tracks()` devuelve las pistas seleccionadas, deduplicadas por
  archivo o clave.
- Las acciones por lote se ejecutan desde un único despachador; las de borrado
  van en orden descendente para no desplazar índices.

---

## 1. Selección múltiple en playlists y borrado de entradas

### Comportamiento

- El botón de suprimir y “Remove from list” actúan sobre todas las entradas
  seleccionadas.
- Nunca borran archivos del disco: solo quitan la entrada de la playlist,
  Tag List o Cue Bank.
- Si no hay selección múltiple, se mantiene el comportamiento actual sobre la
  fila activa.
- En una vista de carpeta, el botón no borra archivos: muestra un aviso o queda
  deshabilitado.
- El borrado es una sola operación undoable.

### Implementación

- `src/app/browser/mod.rs` y `wire.rs`: estado de selección y conexión de
  callbacks.
- `src/app/browser/lists.rs` o `playlists/lists.rs`: nuevo
  `remove_selection(app)`.
- `Home::User(id)`: mapear cada fila visible seleccionada a su índice original
  con `slots[shown[pos]]`; ordenar de mayor a menor.
- `Home::Tags`: eliminar las filas seleccionadas de `tags`.
- `Home::Bank(id)`: borrado múltiple equivalente.
- `src/store/node.rs`: añadir `remove_entries(id, &[usize])` como una sola
  transacción de journal. Sin cambio de esquema.
- Tras borrar: recargar, seleccionar el vecino más cercano, refrescar tabla,
  inspector y estado.
- Toast: “N entradas eliminadas de la playlist”.

### Pruebas

- Borrar 3 de 5 entradas, incluyendo claves duplicadas.
- Borrar con filtro activo: se borran las entradas correctas, no las posiciones
  visuales equivocadas.
- Deshacer restaura todas las entradas.
- En carpeta, la acción no toca archivos.
- Sin selección, se borra la fila activa como hoy.

---

## 2. Selección múltiple en carpetas y acciones por lote

### Comportamiento

- Ctrl/Cmd y Shift funcionan igual que en playlists.
- El clic derecho abre el menú sobre el conjunto seleccionado.
- Las carpetas son de solo lectura: no se ofrece “Remove from Collection” para
  archivos que no están en la colección; si una pista sí está en la colección,
  la acción se aplica a su entrada de colección, no al archivo.
- Acciones por lote previstas:
  - Añadir a playlist.
  - Añadir a Tag List.
  - Analizar las pistas seleccionadas.
  - Forzar reanálisis de las seleccionadas.
  - Analysis Lock / Unlock.
  - Reload Tag.
  - Mostrar en el gestor de archivos.
  - Relocate / Auto relocate, solo si la pista pertenece a la colección.
- “Cargar en deck” sigue siendo una acción de una sola pista: carga la pista
  principal, no 40 temas.

### Implementación

- El mismo motor de selección de la base común.
- `View::batch_action(app, action)` despacha por acción y por tipo de `Home`.
- Reutilizar `Ctx::analyze`, `Ctx::analyze_replacing_grid`,
  `Ctx::remove_tracks` y `track::act` de `src/app/library/`.
- Para análisis masivo, encolar en lotes y mostrar progreso; no bloquear el
  hilo UI.
- Deduplicar por ruta de archivo y omitir pistas sin `file`.

### Pruebas

- Seleccionar con Ctrl y Shift sobre una lista filtrada.
- El menú contextual conserva la selección si se abre sobre una fila
  seleccionada.
- Acción de analizar encola exactamente las pistas seleccionadas.
- “Remove from Collection” no aparece o no se aplica a archivos fuera de la
  colección.

---

## 3. Clic derecho sobre una carpeta del árbol izquierdo

### Comportamiento

- Clic derecho sobre `DeviceFolder` o `DeviceFolderList` abre un menú de carpeta.
- El menú incluye:
  - Abrir / mostrar en la tabla.
  - Marcar como favorita / quitar de favoritas.
  - Analizar carpeta.
  - Analizar carpeta forzando reanálisis.
  - Añadir pistas del nivel actual a una playlist.
  - Añadir todo el subárbol a una playlist.
  - Refrescar listado.
  - Mostrar en el gestor de archivos.
  - Plegar / desplegar.
- Favorita es independiente de analizar: una carpeta puede analizarse sin ser
  favorita.
- El favorito es la carpeta como entidad; no se marcan individualmente sus
  60.000 archivos.

### Implementación

- `ui/browser-tree.slint`: añadir rama de clic derecho y callback `row-menu(i)`.
- `BrowserState`: añadir estado y callback del menú de carpeta, con el `Entry`
  o un identificador estable.
- `ui/browser.slint` o `ui/browser-tree.slint`: `PopupWindow` con las acciones,
  reutilizando el patrón del menú de tabla.
- `src/app/browser/sources.rs`: helper `folder_path(entry)` para resolver:
  - `DeviceFolder(d,j)` → `sources[d].folders[j].path`.
  - `DeviceFolderList(d,j,k)` → `folder.path.join(playlist.id)`.
- `src/app/browser/pinned.rs`: extender `chosen_pin()` a `DeviceFolderList`.
- Favorito: escribir en `pinned_folders`, refrescar fuentes y lanzar la
  indexación en worker. Para carpetas enormes, indexación diferida o con
  progreso; nunca un walk de 60.000 archivos en el hilo UI.
- Refrescar: invalidar `levels` y `deep` de esa carpeta y volver a listar.
- Plegar/desplegar: reutilizar `folded_folders`.

### Pruebas

- Favorito de carpeta anidada aparece en Favorites con la ruta correcta.
- Quitar favorito lo elimina sin tocar archivos.
- Abrir carpeta desde el menú funciona con la misma navegación por niveles.
- Refrescar una carpeta no recorre el disco entero.
- El menú no bloquea la UI aunque la carpeta tenga miles de archivos.

---

## 4. Análisis persistente de una carpeta completa

### Objetivo

- Analizar una carpeta una vez y no repetir trabajo al volver a abrirla.
- Reanalizar solo si el usuario lo fuerza, o si un archivo cambió.
- Respetar Analysis Lock.

### Persistencia

- No tocar el esquema del store.
- Usar un sidecar versionado en la caché de análisis:
  - `analysis::cache::root()/folders/<hash>.bin` o `.json`.
  - Campos: ruta raíz, versión del analizador, fecha, y por archivo: ruta
    relativa, tamaño, mtime, perfil, estado (`fresh`, `missing`, `stale`,
    `locked`, `failed`).
- Escritura atómica `tmp + rename` desde un worker.
- El sidecar sirve para saber qué falta y para reanudar tras cancelar o cerrar.

### Flujo

1. El usuario elige “Analizar carpeta” o “Forzar reanálisis”.
2. Un worker enumera la carpeta con `walk_files` o `Folder::open_partial`;
   nunca en el hilo UI.
3. Por cada archivo:
   - Si está bloqueado: saltar y contar.
   - Si no ha cambiado y el análisis existe con el perfil actual: saltar.
   - Si cambió o falta: encolar.
   - Si existe pero es de una versión anterior del analizador: saltar por
     defecto; solo incluirlo con “Forzar” o con la opción explícita de
     reanalizar versiones anteriores.
4. Procesar en lotes pequeños, con progreso “escaneando x/y”, “analizando x/y”,
   “omitidos”, “fallidos” y cancelación.
5. Guardar el sidecar tras cada lote o al terminar.
6. Actualizar filas e inspector con los tags aprendidos, sin cambiar el grid de
   un deck en reproducción.

### Pruebas

- Segunda ejecución sobre carpeta sin cambios: no decodifica archivos ya
  analizados.
- Cambiar un archivo: solo ese se reencola.
- Forzar: se reencolan todos los no bloqueados.
- Analysis Lock: nunca se reanaliza.
- Cancelar: se detiene el encolado; los trabajos en curso terminan.
- Reabrir tras cancelar: el sidecar conserva lo completado.

---

## 5. Versión del analizador: popup y reanálisis on demand

### Separar dos versiones

- `VERSION`: versión del formato del archivo de caché.
- `ANALYZER_VERSION`: versión del algoritmo de análisis.

Hoy `VERSION=4` rechaza cualquier caché distinta, lo que provoca invalidación
masiva. El plan es:

- Pasar a un formato de caché v5 que acepte v4 como “legacy”.
- Los archivos v4 se cargan con `analyzer_version = 0`; no se borran ni se
  ignoran.
- Los nuevos análisis guardan `ANALYZER_VERSION`.
- `cache::load_for` sigue devolviendo el análisis antiguo para reproducir sin
  esperas.

### Popup al iniciar

- Guardar en `<cache>/analysis/seen-version` la última versión vista.
- Si `ANALYZER_VERSION` es mayor, mostrar una sola vez:
  - “Hay una nueva versión del analizador. ¿Quieres reanalizar los temas ya
    analizados?”
  - Botones: “Reanalizar ahora” y “Más tarde”.
- “Más tarde” registra la versión como vista para no repetir el aviso.
- “Reanalizar ahora” encola solo pistas con versión anterior, desde la colección
  y los sidecares de carpetas conocidas; nunca recorre discos no registrados.
- Respeta Analysis Lock.

### Reanálisis al cargar en deck

- Al cargar una pista, si su análisis tiene
  `analyzer_version < ANALYZER_VERSION`:
  - Usar el análisis existente de inmediato para que el deck no espere.
  - Encolar un reanálisis on demand en background.
  - Deduplicar por ruta para no encolarla dos veces.
- Si hay Analysis Lock, no encolar.
- Al terminar, actualizar caché, fila del browser e inspector.
- El resultado completo se aplica al deck solo de forma segura, según el punto 6.

### Pruebas

- Una caché v4 se carga y sigue funcionando.
- Una caché v5 conserva la versión.
- El popup aparece una vez por versión y “Más tarde” no lo repite.
- Una pista antigua cargada en deck se reanaliza una sola vez.
- No hay invalidación ni borrado masivo.
- Analysis Lock impide el reanálisis.

---

## 6. Aplicación segura del análisis sin romper el deck

### Regla principal

- Nunca modificar el grid, los beats o el análisis vivo de un deck mientras está
  reproduciendo, haciendo scratch o con un seek/loop en transición.

### Flujo

- Al terminar un reanálisis de la pista cargada:
  - Actualizar siempre caché, fila del browser e inspector.
  - Si el deck está parado o en pausa y sigue con el mismo archivo: aplicar de
    forma controlada.
  - Si está reproduciendo: guardar el resultado como `pending` y mostrar
    “Nuevo análisis listo; se aplicará al pausar o al volver a cargar”.
  - Aplicar al pausar, al llegar al final o en la siguiente carga.
- No sobrescribir:
  - Cues manuales.
  - Key editada por el usuario.
  - Tags o comentarios.
  - Grid editado manualmente.
- Si el grid es automático, recalcularlo solo después de aplicar y fuera de una
  transición de SYNC.
- Si Analysis Lock está activo, no hay reanálisis ni pending.

### Pruebas

- Reanalizar mientras reproduce: el grid no cambia.
- Pausar: se aplica el pendiente.
- Volver a cargar: se aplica el análisis nuevo.
- Grid manual: no se sobrescribe.
- Cues y loops se conservan.
- Analysis Lock: no se encola ni se aplica nada.

---

## 7. Popup de nueva versión del analizador al iniciar

### Objetivo

Avisar una sola vez al iniciar cuando existe una versión nueva del analizador, con opciones
"Reanalizar ahora" y "Más tarde", sin reanálisis masivo automático.

### Diseño

- Preferencia `seen-analyzer-version` en `Prefs::unknown` (par clave-valor, string). Se lee con
  `prefs::extra` y se escribe con `prefs::set_extra` / `prefs::update`. Sin tocar el esquema del store.
- En `app::run`, tras `prefs::wire(&app)`, programar un `slint::Timer` one-shot que, después de que el UI
  esté visible, compruebe si `seen < ANALYZER_VERSION`.
- Si es mayor: mostrar un `PopupWindow` modal una sola vez con:
  - "Hay una nueva versión del analizador (ANALYZER_VERSION). ¿Quieres reanalizar los temas ya analizados?"
  - Botón "Reanalizar ahora": marca `seen = ANALYZER_VERSION`, cierra el popup y encola solo pistas con
    versión anterior desde la colección (on demand, sin recorrer discos no registrados).
  - Botón "Más tarde": marca `seen = ANALYZER_VERSION` y cierra sin reanalizar.
- "Reanalizar ahora" respeta Analysis Lock y no toca pistas bloqueadas.
- El popup no bloquea el UI (se lanza por `slint::invoke_from_event_loop` tras el arranque).
- UI: añadir a `BrowserState` `analyzer-version-pending: bool`, `analyzer-version-message: string`
  y `callback analyzer-version-dismiss()`; `PopupWindow` en `browser.slint` controlado por esas propiedades.

### Riesgos controlados

- No aparecer más de una vez por versión (el pref registra la versión vista).
- No disparar reanálisis masivo automático (el reanálisis es on demand por deck load, ya implementado).
- No bloquear el hilo UI al iniciar.

### Pruebas

- Caché v4 se carga y sigue funcionando (versión leída como 0).
- Popup aparece una vez por versión y "Más tarde" no lo repite hasta cambiar de versión.
- "Reanalizar ahora" no recorre discos no registrados ni pistas con Analysis Lock.
- No hay invalidación ni borrado masivo de cachés existentes.

## 8. Aplicación diferida del análisis si el deck está reproduciendo

### Objetivo

Nunca cambiar el grid, beats o análisis vivo de un deck mientras está reproduciendo, haciendo scratch
o con un seek/loop en transición. El nuevo análisis se guarda como pendiente y se aplica al pausar,
al llegar al final o al recargar.

### Diseño

- Campo nuevo en `Loaded` (`src/app/deck/mod.rs`): `pending_analysis: Option<analysis::Analysis>`.
- En `poll_decode` (`src/app/deck/load.rs`), al llegar `analysed`:
  - Calcular `safe = !self.playing() && l.loop_.is_none() && !self.sync.scratching.get()`.
  - Si `safe`: aplicar beats/3band/waveform/cues como ahora.
  - Si no `safe`: guardar `an` en `l.pending_analysis`, NO tocar beats/3band/waveform, y añadir nota
    "grid: diferido (deck reproduciendo)".
- En `tick` (`src/app/deck/tick.rs`), tras `poll_decode()`: si `l.pending_analysis` es `Some` y ahora `safe`,
  aplicar y hacer `take()`; si aún no es safe, conservar para el siguiente tick.
- Detectar también el cambio `playing → false` en `tick` para aplicar pendiente inmediatamente.
- Respetar Analysis Lock: los jobs ya filtran pistas bloqueadas; el pending no se crea para pistas bloqueadas.
- No sobrescribir cues manuales, key editada, grid manual, tags ni comentarios: el código actual solo
  llena lo vacío.

### Riesgos controlados

- No tocar grid mientras reproduce/scratch/seeks/loop.
- Dedupe implícito: una sola `pending_analysis` por deck.
- Aplicar al pausar, al fin o al recargar.
- No sobrescribir ediciones manuales.

### Pruebas

- Reanalizar mientras reproduce: el grid no cambia.
- Pausar: se aplica el pendiente.
- Volver a cargar: se aplica el análisis nuevo.
- Grid manual: no se sobrescribe.
- Cues y loops se conservan.
- Analysis Lock: no se encola ni se aplica nada.

## 9. Batch add to playlist y palette

### Objetivo

Cuando hay varias pistas seleccionadas, añadir todas a una playlist o palette, no solo la pista activa.

### Diseño

- `src/app/browser/palette/mod.rs`: `on_palette_add_chosen` usa `v.picked_tracks()` y hace un bucle:
  para cada track, `q.add_track(&app, &mut v, pal, &t)`. `add_track` ya evita duplicados.
- `src/app/browser/playlists/manage.rs`: `open_add_to()` permite añadir cuando hay selección múltiple;
  `add_to(id)` añade `picked_tracks()` a la playlist elegida, saltando duplicados (si la key ya está en
  `entries`, avisa y salta).
- `View::picked_tracks()` ya existe y devuelve todas las picked (o `selected` si ninguna picked).
- Evitar duplicados y conservar el orden de las pistas seleccionadas.
- Soportar Tag List y user playlists.

### Riesgos controlados

- No crear entradas duplicadas en playlists ni Tag List.
- Conservar el orden de selección.
- No tocar archivos del disco.

### Pruebas

- Seleccionar 3 pistas y añadirlas a una playlist: aparecen las 3, sin duplicados.
- Añadir a palette: aparece en Tag List o palette correspondiente.
- Añadir pistas ya presentes: no se duplican.
- Sin selección múltiple: comportamiento actual sobre la pista activa.

---

## Orden de implementación

| Etapa | Rama propuesta | Contenido |
|---|---|---|
| 1 | `browser-selection` (fusionada en `browser`) | Base común, selección múltiple, borrado de playlists, acciones por lote en carpetas |
| 2 | `browser` (fusionada) | Menú contextual del árbol, favoritos de carpetas anidadas, refrescar, plegar |
| 3 | `browser-analysis-version` (esta fase) | Versión del analizador, caché v4/v5, popup al iniciar, reanálisis on demand |
| 4 | `browser-safe-apply` (esta fase) | Aplicación diferida si el deck reproduce; no cambiar grid en caliente |
| 5 | `browser` (esta fase) | Batch add a playlist y palette con múltiples pistas seleccionadas |
| 6 | `browser` (esta fase) | Pruebas completas, commit y fusión a `browser` |

Cada rama saldría de `browser`, se verificaría y se fusionaría por fast-forward. Sin push, tag ni release.

## Verificación común

- `cargo check --bin openfader` → 0 errores y exactamente los 3 warnings
  preexistentes.
- `cargo test --lib`.
- `cargo test --bin openfader`.
- `cargo test --release --bin openfader`.
- `cargo test --test folder --test folder_tags --test library_speed --test store`.
- `cargo build --release`.
- `git diff --check`.
- Smoke release aislado.
- No tocar `Folder::tracks`, el esquema del store, plugins, i18n, vídeo ni
  `tests/video.rs`.

## Riesgos que hay que controlar

1. **Rendimiento de selección con 60.000 filas**: actualizar solo filas
   afectadas, nunca reconstruir todo el modelo.
2. **Índices de playlist**: el borrado múltiple debe usar `slots` y orden
   descendente; si no, se borran entradas equivocadas.
3. **Favoritos de carpetas enormes**: la indexación no puede hacer 60.000
   lecturas de tags en el hilo UI.
4. **Análisis de carpeta**: sin lotes, cancelación y sidecar, se convierte en el
   mismo problema de una hora que ya detectamos.
5. **Compatibilidad de caché**: aceptar v4 como legacy es obligatorio para no
   invalidar análisis existentes.
6. **Deck en reproducción**: el resultado nuevo se guarda, pero no se aplica al
   grid hasta que sea seguro.
7. **Popup al iniciar**: no bloquear UI, no aparecer más de una vez por versión,
   no disparar reanálisis masivo automático, respetar Analysis Lock.
8. **Aplicación diferida**: no tocar grid mientras reproduce/scratch/seek/loop;
   dedupe por pista; aplicar al pausar/fin; no sobrescribir cues manuales,
   key editada, grid manual ni tags.
9. **Batch add playlist/palette**: evitar duplicados, conservar orden,
   soportar Tag List + user playlists + palettes, no tocar archivos.

## Siguiente paso propuesto

Implementar la **etapa 1** en `browser-selection`: selección múltiple, borrado
de entradas de playlist y acciones por lote en carpetas.
