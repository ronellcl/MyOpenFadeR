# Hallazgos solucionados — registro de reparaciones

**Fecha:** 17/10/2026 (primera tanda) y 20/10/2026 (segunda tanda, la de la interfaz, el vídeo, los plugins y el instalador) · **Rama del core:** `fixes/hallazgos` (desde `main` @ `dbbe938`), luego `main` con la fusión del original (`10e05f4`) y el trabajo de esta tanda sobre `main` · **Rama de mapeos:** `fixes/duplicated-jog-bend` en `maps-repo/` (desde su `main`) · **Rama de plugins:** `fixes/sdk-pin` y luego `main` en `plugins-repo/` (desde su `main`)
**Fuente:** la primera tanda, cada apartado corresponde a un hallazgo de `hallazgos.md` (mismo ID). La segunda tanda, lo que se encontró **usando** la aplicación (la app instalada, un vídeo de YouTube, una descarga de Deezer); sus identificadores son nuevos y se explican en su propia tabla. Se resolvió todo lo que no afecta el funcionamiento, según lo acordado.
**Verificación:** suite completa del core verde — `cargo test --lib` **580/580** y `cargo test --bin openfader` **473/473** (1 ignorado, preexistente). De la segunda tanda: `cargo test --lib` **571 ok / 11 fallos** (los 11 son los tests de plugin-fichero, que necesitan `sh` en el PATH, preexistente) y `cargo test --bin openfader` **480 ok / 1 ignorado**.

## Resumen

| ID | Hallazgo | Estado | Núcleo de la solución | Commit |
|----|----------|--------|------------------------|--------|
| CORE-01 | Path traversal en `folder_name` | ✅ Resuelto | Nombre de manifiesto validado como nombre de carpeta | `96ad82c` |
| CORE-02 | No se puede borrar la DLL cargada de un plugin viejo | ✅ Resuelto | Destino «apartado» con rename (no borrado) | `96ad82c` |
| CORE-04 | Timeouts que no cubren `rk.run` ni el join | ✅ Resuelto | `rk.run` bajo el plazo del script, hijo muerto al vencer | `96ad82c` |
| CORE-05 | Concurrencia sin límite | ✅ Resuelto | Compuertas con política *descartar el más nuevo* | `96ad82c` |
| CORE-06 | Probes síncronos y sin timeout | ✅ Resuelto | Probe en hilo propio + 10 s + caché por (ruta, mtime) | `2b327d7` |
| CORE-07 | Instalación no transaccional | ✅ Resuelto | Rename-away + put-back + purga de `.old-*` | `96ad82c` |
| CORE-08 | Zip sin topes | ✅ Resuelto | Tope de entradas, directorio central y bytes descomprimidos | `96ad82c` |
| CORE-09 | Manifiesto acepta claves desconocidas en silencio | ✅ Resuelto | Aviso a stderr, sin fallar | `96ad82c` |
| MAP-01 | Duplicados `jog`/`jog-bend` (scratch roto) | ✅ Resuelto | Corregidos los 13 XML + el runtime prefiere `jog` | core `cd51075` · mapas `1b2fdba` |
| MAP-02 | Un XML roto desactiva todos los mapeos | ✅ Resuelto | Cargar tolerante: el roto se salta y se informa | `cd51075` |
| MAP-03 | Parser silencioso | ✅ Resuelto | Aviso por elemento desconocido, `schemaVersion` y `deck` fuera de 0..3 | `cd51075` |
| MAP-04 | Test que no cubre el mapa runtime | ✅ Resuelto | Round-trip ahora rechaza claves `(status, midino)` repetidas | `cd51075` |
| MAP-07 | `audio-cards.json` fuera de GET MAPPINGS | ✅ Resuelto | El catálogo lo lee y se instala junto al mapeo (si el usuario no tiene el suyo) | `aa31d3a` |
| X-03 | `.part` sin limpieza en la ruta de error | ✅ Resuelto | write+rename envueltos; el borrador se elimina ante fallo | `aa31d3a` |
| THEME-01 | Temas publicados sin control de contraste | ✅ Resuelto | Test WCAG AA de los temas del checkout: par bajo → aviso (sin vetar temas artísticos) | `c12b87f` |
| THEME-02 | Longitudes sin cotas | ✅ Resuelto | `0..=512px` y `0..=5000ms`, con nota al recortar | `c12b87f` |
| THEME-03 | Formato de temas sin `schemaVersion` | ✅ Resuelto | `schemaVersion: 1` aditivo: el 1 es silencioso, otro valor avisa y se lee como 1 | `c12b87f` |
| THEME-04 | Guardado de temas no atómico | ✅ Resuelto | `.part` + rename con limpieza en fallo (patrón de los mapeos) | `c12b87f` |
| THEME-06 | Hints de layout sin persistencia (sorpresa) | ✅ Resuelto (documentado) | `docs/skins.md` lo explicita: la libertad del usuario manda, el tema no re-aplica | `c12b87f` |
| MAP-05 | Acciones sin uso en los mapeos | ✅ Resuelto (mapeadas + documentado) | 10 de las 23 acciones atadas a los pads libres de DDJ-1000/FLX10; las 13 restantes, vía MIDI LEARN | `docs/mappings.md`, `maps-repo` (`fixes/map05-actions`) |
| MAP-06 | Documentación de mapeos de 24 líneas | ✅ Resuelto | Especificación completa + tablas generadas de acciones/luces con test + `audio-cards.json` | `af0e0aa` |
| X-01 | Descargas sin digest verificado | ✅ Resuelto | `SHA256SUMS` por release se verifica al bajar (opt-in; sin lista → aviso) | `5108039` |
| CORE-03 | Plugins sin aislamiento (modelo de confianza implícito) | ✅ Resuelto (documentación + aviso) | Sección «Trust» en `docs/plugins.md` + aviso stderr al instalar de terceros | `5108039` |
| X-02 | «GitHub como registro» sin modelo de confianza escrito | ✅ Resuelto (documentado) | Mismo apartado «Trust»: confianza por repositorio, los `OpenRsG/*` curados | `5108039` |
| PLUG-01 | Dependencia del SDK por ruta sin pin | ✅ Resuelto | `openfader-plugin` fijado al tag `v0.1.0` del core: el repo de plugins compila solo · override local documentado | plugins `47de0ff` + core `70b233b` |
| X-04 | Sin i18n | ✅ Resuelto (piloto) | Ajuste LANGUAGE (sistema/inglés/español): menús `@tr` con catálogos bundled + tablas en Rust para las páginas de preferencias; español de fábrica | `4b185f3` |

Pendientes que requieren decisiones de producto o trabajo aparte, fuera de este documento: **THEME-05** (edición opt-in «guardar solo los tokens cambiados»: decisión de UI). X-04 quedó como piloto completo (menús + páginas de preferencias); traducir el resto (toasts, diálogos del navegador, texto de los paneles) sigue el mismo mecanismo sin decisiones nuevas.

---

## Detalle por hallazgo

### CORE-01 · path traversal en `folder_name` (crítico)
**Solución** (`src/plugin/install.rs`): `folder_name` dejó de copiar puntos y separadores: todo lo que no sea alfanumérico Unicode o `-_ ` va a `_`, se exige al menos una letra o dígito real (rechaza `..`, `.`, solo puntuación y cadenas vacías) y se rechazan las 22 palabras reservadas de dispositivos de Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`) que en `C:` crearían una carpeta ilegible. El resultado ya no puede salirse de la raíz de extensiones ni por truco del manifiesto.

**Pruebas:** `a_manifest_name_cannot_walk_out_of_the_extension_root` (un manifiesto `".."` ya no borra `extension/` ni nada fuera; `NUL`, `con`, `​...` se rechazan), `folder_names_sanitize_reject_walk_and_devices` (tabla de entradas válidas e inválidas).

### CORE-02 + CORE-07 · instalación segura frente a DLL cargada y fallos
**Solución** (`src/plugin/install.rs`): el destino se **aparta** con un rename a `.old-<pid>` y se devuelve con otro rename si nada de la actualización falla — el caso feliz es idéntico y un fallo (disco lleno, antivirus, DLL cargada) deja la copia vieja intacta y usable. Los restos `.old-<pid>` de sesiones anteriores se purgan al empezar la siguiente instalación. El staging (`.installing-<pid>`) se limpia igual que antes.

**Por qué rename y no delete:** en Windows, sobreescribir o borrar un fichero mapeado (DLL cargada) se rechaza, pero renombrar su carpeta suele permitirse — es el patrón que usan los instaladores para «renombrar y borrar luego».

**Pruebas:** `a_failed_update_leaves_the_old_copy_in_place` (el apartado y la vuelta funcionan donde el borrado se negaría; la versión previa sobrevive).

### CORE-04 · plazo de `rk.run` y del join
**Solución** (`src/plugin/lua.rs`): el VM hook ya recortaba scripts en bucle, pero un script bloqueado dentro de `rk.run` no dispara instrucciones: ahora `rk.run` lanza el hijo y lo **mata al vencer el plazo** del propio script (bucle `try_wait` + `kill`, con el stdout leído en un hilo para que un hijo que embuta la tubería no cuelgue al script). El mensaje de reproducción del fallo es `rk.run("algo") ran past the script timeout`.
En el lado del proceso (`Plugin::call`) el deadline ya existía (kill + wait) y los joins de escritura/lectura quedan acotados por el cierre de las tuberías: no requieren cambio.

### CORE-05 · sin límite de concurrencia
**Solución** (nuevo `src/plugin/concur.rs` + uso en `event.rs` y `source/mod.rs`): `Gate` es un semáforo contador **sin bloqueo** en `slot()`/`Slot` (RAII), con estáticos `EVENTS` y `SOURCES` de 16 huecos. Política *descartar el más nuevo*: un hook de evento que llegue con la compuerta llena se salta un disparo (con aviso a stderr) y una lectura de fuente se re-encola al siguiente rebuild — exactamente la conducta que ya tienen los plugins lentos. Los 16 huecos son generosos: ninguna sesión real los alcanza, y un plugin en bucle ya no puede fabricar cientos de hilos.

**Pruebas:** semáforo con conteo (bloqueo no bloqueante, devolución por drop).

### CORE-06 · probe de librerías nativas síncrono
**Solución** (`src/plugin/library/mod.rs`): el probe corre en un **hilo propio bajo reloj de 10 s** (`probe_bounded`); un worker colgado se abandona (una llamada ABI no se puede interrumpir) y la librería simplemente no participa en esa ronda, con aviso a stderr. Se añadió la **caché** que pedía el hallazgo: resultados por `(plugin, ruta, mtime)` en `PROBES` (`LazyLock`+`parking_lot::Mutex`), de modo que re-escanear la misma carpeta no vuelve a entrar en la librería; un cambio de mtime genera clave nueva y fuerza re-probe.

### CORE-08 · zip sin topes
**Solución** (`src/zip.rs`): tres topes + un contador de escritura:
- `MAX_ENTRIES = 10_000` (bomba de índice central rechazada con error claro).
- `MAX_CENTRAL = 64 MiB` y el offset del directorio central debe caer dentro del fichero: un zip que declara una tabla de 4 GiB (o fuera del archivo) se **rechaza sin asignar esa memoria**.
- `MAX_UNPACK = 1 GiB` por `unpack()`: la suma de tamaños declarados se comprueba **antes de escribir nada**, y `Cap<W>` (writer contador) corta la descompresión en el tope si el fichero miente sobre el tamaño — el CRC ya no es la única línea de defensa.
- `unpack_capped()` expone el tope para pruebas.
**Pruebas:** entrada > 10 001 rechazada; directorio central con tamaño mentiroso y con offset fuera de fichero; zip diseñado a mano que declara 100 bytes y contiene 64 KiB reales → se detiene en el `cap`.

### CORE-09 · manifiesto permisivo
**Solución** (`src/plugin/mod.rs`): `read_manifest` parsea también como JSON genérico y `unknown_keys` lista a **stderr** las claves no reconocidas (`plugin.json of <dir>: unknown keys ignored: sorce, …`) sin rechazar el plugin — un archivo viejo sigue cargando en la app nueva y un typo ya tiene diagnóstico. La lista de claves conocidas reproduce los 16 campos de `Manifest`.

### MAP-01 · duplicados `jog`/`jog-bend` (scratch roto en 13 controladoras)
**Datos (maps-repo, commit `1b2fdba`):** eliminadas las **66 líneas** repetidas de `jog-bend` en **13 archivos** (`OMNIS-DUO`, `XDJ-AN`, `XDJ-RX3`, `XDJ-XZ__1_`, `PIONEER/RX…` y sus clones `_MIDI`). Es el cambio que restaura el scratch: el binding del platter vuelve a ser `jog`.
**Runtime (core, commit `cd51075`):** `mapfile::one_per_key` colapsa los bindings en uno por `(status, midino)` en vez del «último gana» del `collect()`. Ante repeticiones se queda el primero — con una excepción deliberada: un `jog` posterior va sobre un `jog-bend` del mismo pad, porque el modo Bend no contesta nada mientras el platter esté tocado — y toda repetición entra en la línea de estado de MIDI, de modo que un archivo mal editado avisa en vez de fallar en silencio.
**Verificación:** el round-trip de los 176 XML del clon carga y reescribe exacto, y el chequeo anti-colisión pasa sin duplicados tras la corrección de datos.

### MAP-02 · un XML roto desactiva todos los mapeos
**Solución** (`src/app/midi/mapfile.rs` + `session.rs`): `pick` ya no propaga el primer error; acumula errores por archivo y lo salta. La respuesta es un `Pick { map, errors }` (antes `Result<Option<…>, String>`), y `session.rs` muestra los errores en la línea de estado de MIDI. Un archivo suelto sin `<name>` en `<config>/mappings` ya no desactiva las controladoras que sí tienen su archivo.

**Prueba:** `a_broken_file_skips_itself_and_the_others_load`.

### MAP-03 · parser silencioso
**Solución** (`src/app/midi/mapfile.rs`): tres diagnósticos nuevos dirigidos a `Mapping::bad` (→ línea de estado):
- elementos no reconocidos (`<controll> is not a mapping element; ignored`) — antes silencio absoluto;
- `schemaVersion` ≠ 1 (`schemaVersion "2" is not read here (this format speaks 1)`);
- `deck` fuera de 0..3 en controles y salidas (se salta la entrada con aviso: `action "play" has deck 5 (0..3 is A..D)`).
**Prueba:** `unknown_elements_and_schema_and_decks_are_reported` (tabla completa). El round-trip sobre los 176 archivos sigue exacto (ninguno dispara los nuevos avisos).

### MAP-04 · el test no cubría el mapa runtime
**Solución** (`src/app/midi/mapfile.rs`): `every_checked_out_mapping_reads_back_to_the_same_file` además comprueba que **ningún `(status, midino)` se repite** por archivo (el test de round-trip solo cubría el *archivo*), y mira `mappings/` y, si no existe, el clon vecino `../maps-repo` (con el clon de OpenFadeR-Map junto al core, como en este workspace).

### MAP-07 · `audio-cards.json` fuera de GET MAPPINGS
**Solución** (`src/catalog/mappings.rs`, `src/app/midi/get.rs`, `src/app/catalog/{mod,mapping}.rs`):
- el **scan** del catálogo reconoce `audio-cards.json` junto a los `*.xml`;
- `Entry` lleva `cards: Option<String>` y los dos flujos (GET MAPPINGS y la oferta para un controlador sin mapeo) la adjuntan desde su repositorio;
- `get::install_in` instala la tarjeta del repo **solo si el usuario no conserva ya una** (la primera carpeta con el fichero es la que la app lee; la del usuario gana), con `.part` + rename igual que los mapeos.

**Pruebas:** `installing_a_mapping_brings_the_audio_cards_along_once` (se instala con el mapeo y un segundo intento no pisa la del usuario).

### X-03 · `.part` sin limpieza en la ruta de error
**Solución** (`src/app/midi/get.rs`): `install_in` e `install_cards` envuelven write+rename y, si queda error, **borran el fichero borrador** antes de propagar. Un destino que no se puede pisar (un directorio donde va el mapeo) ya no deja `a.xml.part` huérfano ni toca lo que estaba.

**Prueba:** `a_failed_install_leaves_no_part_file_and_the_old_copy`.

### THEME-02 + THEME-03 · cotas de los valores y `schemaVersion`
**Solución** (`src/app/skin/file.rs`): las longitudes quedan en **0..=512 px** (`font-*`, `h-*`, `jog`…: el mayor por defecto es `h-deck-2` con 240 px, margen sobrado) y las duraciones en **0..=5000 ms** (`t-fast` 80, `t-fade` 180, `tooltip-delay` 600 por defecto). Un valor fuera de rango no se rechaza: se **recorta con nota** (`token "font-size": -5 is out of range 0..=512px, using 0px`), igual que un error pero sin perder el resto del tema. Los negativos de duración siguen fallando (como antes). El nuevo `schemaVersion` es **aditivo y silencioso para 1**; otro valor avisa (`schemaVersion "2" is not supported, reading as 1`) y se lee como 1. `serialize` lo escribe, así que los temas que guarde el editor van versionados; los 11 del repo cargan sin nota (no lo llevan) y el round-trip de la suite sigue exacto.

**Pruebas:** `lengths_and_durations_are_bounded` (los tres recortes + en-rango silencioso), `a_schema_version_of_1_is_silent_and_the_rest_notes`, round-trip `serialize_writes_what_parse_reads` intacto.

### THEME-04 · guardado no atómico
**Solución** (`src/app/skin/store.rs`): `write_into` escribe a `<slug>.json.part` y **renombra**, con limpieza del borrador en la ruta de error (el mismo patrón que los mapeos y las descargas). Un corte de luz o un disco lleno ya no puede truncar el JSON de un tema propio, y un destino bloqueado (donde hay un directorio por medio) no deja `.part` huérfano.

**Prueba:** `a_failed_save_leaves_no_part_file_and_what_was_there` (directorio donde va `blocked.json`: el apartado sobrevive, no queda borrador).

### THEME-01 · sin validación de contraste
**Solución** (`src/app/skin/file.rs`, módulo de pruebas): `the_checked_out_themes_have_readable_text_contrast` calcula el **ratio WCAG** de los cuatro pares de texto clave (`text`/`bg`, `text`/`panel`, `row-dim`/`bg` con umbral AA 4.5; `text-bright`/`accent` con umbral 3.0) sobre los temas del checkout (`extension/skin` o el clon vecino `../themes-repo`), componiendo el alfa sobre la superficie. Un par por debajo del umbral emite un **aviso por stderr con el tema y el par** — no falla el test, porque vetar temas artísticos no es el objetivo (así lo pide el hallazgo). El CI de OpenFadeR-Themes ya lo ejecuta: su línea `cargo test … the_checked_out_themes` casa con este test por prefijo, sin tocar el `ci.yml` del repo.
**Medición de partida** (los 11 temas, script auxiliar fuera del repo): `text`/`bg` mínimo 8.24 (Frappé), `text`/`panel` mínimo 6.75; los avisos salen solo en `text-bright`/`accent` de High Contrast (1.54), Ocean (2.35) y Frappé (2.99) — pares decorativos, no ilegibilidades del cuerpo de texto.

### THEME-06 · hints de layout sin persistencia (sorpresa documentada)
**Solución** (`docs/skins.md`): el párrafo de *Layout hints* explicita la decisión — los hints se aplican **una vez al elegir el tema**; si el usuario cambia la vista después, **su cambio es el que dura** y el tema no lo re-aplica en el arranque siguiente. Es lo que el hallazgo recomienda (re-aplicar en cada inicio anularía los cambios manuales del usuario y sí afectaría el funcionamiento): comportamiento intacto, expectativa escrita.

### THEME-05 · SAVE AS congela los 120 tokens (decisión de diseño)
**Estado:** el hallazgo lo califica de *Diseño*, y su solución recomendada es una opción nueva del editor («guardar solo los tokens cambiados», opt-in): eso es una decisión de UI/producto pendiente, no un simple parche. Lo hecho en su lugar: el **trade-off queda documentado** en `docs/skins.md` (SAVE AS lista cada token a propósito: un tema guardado deja de depender del base y no hereda mejoras futuras — los autoriales parciales sí). Nada en el código cambió.

### MAP-06 · documentación de mapeos insuficiente
**Solución** (`docs/mappings.md`, de 24 a ~290 líneas; + test):
- la especificación completa se promovió desde el comentario cabecera de `mapfile.rs` (ejemplo XML, `schemaVersion`, `deck`/`arg`/`blink`, `one per key` por MAP-01, los diagnósticos);
- **tabla de las 145 acciones generada desde `ACTIONS`** (nombre + forma del mensaje que quiere) y **tabla de las 35 luces** como las escribe un fichero (`Fx(0,2)` etc.), ambas cubiertas por `the_mappings_doc_lists_every_action_and_light`, que falla cuando el código crece y el documento no;
- `audio-cards.json` documentado (formato `{"headphones_3_4": ["fragmento", …]}`, primera carpeta con fichero gana, lo instala GET MAPPINGS, el del usuario vence).

### MAP-05 · acciones del core sin uso en mapeos
**Solución** (mapeado + documentación, como pide un hallazgo *informativo*): las 23 acciones sin XML se repartieron en dos frentes.

- **10 acciones atadas a los pads libres de DDJ-1000 y FLX10** (rama `fixes/map05-actions` del repo de mapas). Con los charts MIDI oficiales de Pioneer verifiqué que esas dos controladoras dejan **enteramente libres** las páginas de pad BEAT JUMP, BEAT LOOP y KEY SHIFT en los 4 decks (canales 8/10/12/14 = status `0x97/0x99/0x9B/0x9D`), que son de *state machine de hardware* — esas notas **solo** se envían cuando el usuario pone el pad en ese modo, así que el atado es puramente aditivo y no toca nada existente:
  - **BEAT JUMP** (`0x20-0x27`): `jump-back-4`, `jump-back-1`, `jump-fwd-1`, `jump-fwd-4` (atrás a la izquierda, adelante a la derecha, saltos mayores en los extremos);
  - **BEAT LOOP** (`0x60-0x67`): `loop-1`, `loop-2`, `loop-4`, `loop-8`, `loop-16` en fila ascendente (`loop-4` ya se usaba en otras controladoras, se incluye aquí para que la fila lea 1‑2‑4‑8‑16);
  - **KEY SHIFT** (`0x70-0x77`): `key-down` a la izquierda, `key-up` a la derecha.
  El **DDJ-REV7 no se tocó**: sus pads ya están atados (`hw-pad`/`hw-pad-delete`/`hw-pad-unload`). El test `every_checked_out_mapping_reads_back_to_the_same_file` (round-trip byte a byte + sin `(status, midino)` repetido) pasa con los XML ampliados.
- **las 13 acciones restantes** (`brake`, `fx-level`, `fx-toggle`, `grid-beat1/double/halve/edit/reset`, `help`, `pad-mode`, `split`, `zoom-in/out`) no tienen hardware dedicado claro en estas controladoras → documentadas como **«solo MIDI LEARN»**: la tabla de `docs/mappings.md` y su texto dejan claro que *toda* la tabla de acciones está al alcance de MIDI LEARN, no solo las que los XML de fábrica usan. El motor no cambió.
- *Caveat:* los 10 atados de pads son aditivos y semánticamente exactos (cada página de pad solo emite sus notas en su modo), pero **no se verificaron en hardware físico** — queda anotado por si conviene probarlos en un DDJ-1000/FLX10 real.

### X-01 · descargas sin checksum
**Solución** (`src/catalog/check.rs` nuevo + `mod.rs`/`plugins.rs`):
- un repositorio publica **`SHA256SUMS` por release** (una línea por asset, más el `<repo>-HEAD.zip` del archivo sin formato); es *opt-in* — su mantenimiento corre por release;
- cada descarga (`fetch` del archivo del repo y el asset preconstruido de librerías en `stage`) se verifica contra el `SHA256SUMS` de la última release **antes de usarse**: el hash concuerda → se usa; la lista no contiene la entrada, o el hash difiere → **se rechaza el fichero y se elimina** con un error específico (el mensaje dice qué hash se esperaba);
- sin lista (HTTP 404) → **aviso por stderr** y sigue instalando, para no cortar a los repos que aún no publican la lista;
- dispositivo `sha2` (puro Rust) como nueva dependencia;
- **partida de referencia publicada:** el CI del core debería regenerar el `SHA256SUMS` de los releases de OpenFadeR-Plugins al publicar zips (el flujo ya existe; el fichero se genera con `sha256sum <zips> > SHA256SUMS` y se adjunta al release) — pendiente de ejecutarse en su próximo release.

**Pruebas:** `sha256_is_the_one_every_answersheet_has` (vectores NIST), `the_list_entries_match_a_matching_file` (concordancia, fichro mal, entrada ausente, entrada corrupta, nombre con espacios).

### CORE-03 + X-02 · plugins sin aislamiento: modelo de confianza escrito y aviso al instalar
**Solución**:
- `docs/plugins.md` gana la sección **«Trust»**: qué puede un plugin (hilo de audio nativo, Lua en proceso, script Python normal — sin sandbox ni perfiles de permisos), qué protege el propio tiempo de ejecución (plazos y topes de concurrencia contra lo *lento*, no contra lo *hostil*), cómo es «GitHub como registro» (X-02: sin revisión central, la confianza es por repositorio, los `OpenRsG/*` por defecto son curados) y **qué comprueba el app por su cuenta** (SHA256SUMS de X-01, topes anti-bomba del zip, claves de manifiesto desconocidas con aviso);
- **aviso al instalar de terceros** (`catalog::plugins::install` → `warn_third_party`): al instalar un plugin desde un repositorio que no es de los `OpenRsG/*` de la primera ejecución, stderr lo dice con el nombre del repositorio. Es de la misma naturaleza de los avisos y diagnósticos del app (los skins y los mapeos ya trabajan con stderr); no toca el flujo ni la salida del instalador.
- el aislamiento real (scripts en procesos aparte con perfiles) queda declarado como **decisión de medio plazo** en el propio apartado, sin cambios de ejecución.

### PLUG-01 · build del workspace de plugins atado a `../../sdk`
**Solución** (`plugins-repo`, rama `fixes/sdk-pin`, commit `47de0ff`):
- `[workspace.dependencies]` del workspace ya no usa la ruta: `openfader-plugin = { git = "https://github.com/OpenRsG/OpenFadeR", tag = "v0.1.0", features = ["library"] }` — verificado con `cargo metadata`: resuelve al SDK del tag (`#248d269d`) y `rekordbox-xml` compila hasta tropezar con lo del entorno (perl de `openssl-sys`, preexistente);
- el repo entero **compila solo**: no hace falta un checkout del core al lado para ni siquiera `cargo metadata` (que antes fallaba directamente por la ruta ausente);
- el desarrollo sobre el `sdk/` propio sigue disponible por **`[patch]` en config local** (no comprometido), documentado en `README.md`, `CONTRIBUTING.md` y el propio `Cargo.toml`;
- el CI del repo de plugins mantiene los dos checkouts, y su comentario ya refleja que el SDK viene fijado;
- el core por su parte compila su binario con el SDK de aquí (`{ path = "sdk" }`, sin cambio) y `docs/plugins.md` cuenta el nuevo régimen (commit `70b233b`).

### X-04 · sin i18n (infraestructura de idiomas, piloto completo)
**Solución** (`4b185f3`, core `fixes/hallazgos`): la interfaz ya tiene **dos idiomas** (inglés, que son los literales del código, y **español de fábrica**) con la detección del locale del sistema por defecto y un ajuste para elegir el idioma:

- **ajuste nuevo** (`language = system|english|spanish`, PREFERENCES > VIEW > LANGUAGE, filas CHIPS): `system` (por defecto) lee el locale del usuario — `GetUserDefaultLocaleName` en Windows, `LC_ALL`/`LC_MESSAGES`/`LANG` en el resto; por defecto el castellano del sistema recibe los menús en español sin tocar nada;
- **cadenas de `.slint`** (los menús de `app.slint`, 71 literales): marcadas con `@tr(...)`; los catálogos **bundled** (`ui/translations/es/LC_MESSAGES/openfader-ui.po`) los empaqueta el build (`ui/build.rs`, `with_bundled_translations`) y `slint::select_bundled_translation` los cambia en caliente (`src/app/i18n.rs` sigue el ajuste en `prefs::wire` y con cada cambio; el nombre del dominio del catálogo es el del crate de la UI, `openfader-ui`);
- **cadenas de Rust** (las páginas de preferencias: etiquetas, cabeceras, chips y valores de dropdown, ~290 cadenas): `src/app/i18n.rs` trae la tabla `ES` y la aplica **centralmente** en los constructores de `Row` (`src/app/prefs/rows.rs`) — las 10 páginas de preferencias quedan traducidas sin retocar cada constructor; las claves, los hints y los valores que vienen de fuera (nombres de dispositivos, rutas, los manifiestos de los plugins, ya traducidos en la fase 1) pasan intactos);
- **lo que no está en la traducción queda en inglés** (mensaje truncado mejor que roto): son los *toasts* y textos de los paneles; añadir la palabra a la tabla o marcar su `@tr` es mecánico y no toca nada del mecanismo.

**Pruebas:** `the_menus_take_the_bundled_catalogs_after_the_window` (window headless + `select_bundled_translation("es")` → Ok: el dominio y el catálogo empaquetado concuerdan), `language_is_a_choice_setting_and_choose_applies_it` (parse/write del ajuste + clic path), `t_falls_back_to_english_and_translates_the_table_when_spanish_applies`.

---

## Segunda tanda (19-20/10/2026): la interfaz, el vídeo, los plugins y el instalador

**Rama del core:** `main` (sobre el 0.2.1 del original, con la fusión `10e05f4`) · **Rama de plugins:** `main` de `plugins-repo` (`d55a2a0`) · **Motor de Deezer:** `feature/deezer-v2` de `plugins-repo`

Esta tanda no viene de `hallazgos.md` (el audit del original): es lo que se encontró **usando** la aplicación —la app instalada, un vídeo de YouTube, una descarga de Deezer— y lo que se reparó. Los identificadores son nuevos: `WIN` (ventanas de consola, vídeo e instalación en Windows), `PLUG` (plugins), `DATA` (colección), `UI` (interfaz), `PACK` (empaquetado), `DEEZ` (motor nativo de Deezer) y `DOC` (documentación).

| ID | Hallazgo | Estado | Núcleo de la solución | Commit |
|----|----------|--------|------------------------|--------|
| WIN-01 | Cada llamada a un plugin abría una **ventana de Windows Terminal** con el lanzador `py` | ✅ Resuelto | El intérprete que viaja con el programa (`python\`) es el primero; `py` queda como último recurso | `9d908dc` |
| WIN-02 | Los ayudantes de consola (ffmpeg, ffprobe, selectores de fichero) podían heredar una consola | ✅ Resuelto | `CREATE_NO_WINDOW \| DETACHED_PROCESS` en todos ellos; el que lanza a otro de consola usa `DETACHED_PROCESS` para que el nieto no tenga ninguna | `570b72d` |
| WIN-03 | En un salto de vídeo **la imagen se quedaba pegada en el frame del sitio anterior** (3 s) | ✅ Resuelto | Mientras el salto está en curso la imagen se pone **negra**, y sale el ffmpeg ligero (80 ms): el frame nuevo está a ~460 ms | `734797e` |
| WIN-04 | El ffmpeg del sistema tarda **2 s en arrancar** y cada salto lo pagaba | ✅ Resuelto | El instalador lleva un build ligero y **mide su arranque y avisa** si el ffmpeg es el lento | `734797e`, `5289290` |
| PLUG-02 | El yt-dlp no venía en el instalador y su auto-actualización era **imposible bajo Program Files** (el binario es de solo lectura) | ✅ Resuelto | El programa viaja dentro y el plugin lo **siembra** en la carpeta de datos (escritura) la primera vez | `400ebb1`, `d6d1b9f` |
| PLUG-03 | La copia de un plugin en la carpeta de configuración **tapaba** a la del instalador (manda el `name` del manifiesto, no el nombre de la carpeta) | ✅ Resuelto | Aviso de copias ignoradas con la carpeta en uso, carpeta de origen a la vista y versiones comparadas numéricamente | `2eb82bf` |
| PLUG-04 | Un error inesperado en el plugin lo mataba **sin responder** (la app esperaba un reply que no llegaba) | ✅ Resuelto | Todo error se responde como `{"error": …}`; el rastro limpio vive en la respuesta | plugins `d1ba1de` |
| PLUG-05 | Un plugin cuyo programa se nombra **sin extensión** no se encontraba (la ruta de un hijo se resuelve contra la carpeta de quien llama) | ✅ Resuelto | `resolve()` prueba con `.exe` en Windows, y hay test | `e0f280a` |
| DATA-01 | Una pista de una fuente de plugin (YouTube, Deezer) **no podía vivir** en una lista ni en una paleta | ✅ Resuelto | Migración v16 (`plugin`, `source`, `remote_id`, ruta sintética); el audio se descarga al cargarla y el navegador marca con un ✓ las descargadas | `02864d8` |
| UI-01 | El menú «Ver > Pantalla completa» **no hacía nada** (ponía una propiedad que nadie leía) | ✅ Resuelto | Callback a Rust, y la ventana **abre a pantalla completa** por defecto (ajuste recordable) | `b28cf41` |
| UI-02 | Las preferencias que añadió el original (rejilla de beats en negrita, marcas de hot cue grandes) llegaban **en inglés**, y el texto de las filas no se traducía | ✅ Resuelto | Cinco entradas nuevas en la tabla `ES` y el `hint` de las filas pasa por el traductor | `5289290` |
| UI-03 | No existía el **escenario VDJ** (etapa de Virtual DJ) | ✅ Resuelto | `stage-layout: 1`: la franja de FX y la pantalla maestra bajo las mesas, una pantalla por mesa, faders de canal con su longitud | `7cdc0a4` |
| PACK-01 | **No había instalador** de Windows | ✅ Resuelto | NSIS con el programa, los plugins, los temas, los mapeos, ffmpeg, Python, el runtime de VC++ y un desinstalador que respeta tus datos | `525689b` |
| PACK-02 | El motor de Deezer no se empaquetaba, y la compilación **se quedaba colgada** al copiarlo (la app lo tenía abierto) | ✅ Resuelto | El motor se compila en el empaquetado y el exe recién hecho va directo al payload | `1d9f8c9`, `d6d1b9f` |
| PACK-03 | El manifiesto del yt-dlp 1.2.3 en español se perdió al guardarlo con BOM y acentos corruptos | ✅ Resuelto | Reescrito a mano en UTF-8 sin BOM, con la misma estructura y los mismos campos | plugins `d1ba1de` |
| DEEZ-01 | El motor de Deezer fallaba al descargar con **«Invalid CSRF token»** | ✅ Resuelto | La jarra de cookies: Deezer ata el token CSRF a la sesión (`sid`, `dzr_uniq_id`) | plugins `d1ba1de` |
| DEEZ-02 | «Deezer no dio un token para la pista» | ✅ Resuelto | La ficha de la pista viene en `results.DATA`, no en la raíz | plugins `d1ba1de` |
| DEEZ-03 | El token del gateway **rota** entre llamadas | ✅ Resuelto | Si el gateway lo rechaza, sesión nueva y un reintento (como hace deemix) | plugins `d1ba1de` |
| DOC-01 | El README no decía nada de esta versión y solo estaba en inglés | ✅ Resuelto | Sección «Qué añade esta versión» y `README.es.md` completo, con selector de idioma | `975576a` |

### WIN-01 · el lanzador `py` abría una ventana de Terminal en cada llamada a un plugin

**Síntoma:** al buscar en YouTube, al abrir el gestor de plugins o al pulsar cualquier acción, aparecía una ventana de consola con el título `C:\WINDOWS\py.exe` sobre la aplicación.

**Causa:** `python()` elegía el lanzador `py` de python.org siempre que estuviera en el PATH (lo está, en `C:\Windows`). El lanzador es un programa de consola que **arranca el intérprete real sin flags propios**: el hijo hereda la consola del lanzador, que a su vez no tenía (la app es una app de ventana) → Windows le asigna una nueva → ventana.

**Solución** (`src/plugin/mod.rs`): el intérprete que **viaja con el programa** (`python\python.exe`, el que instala el instalador) es el primero; luego el del PATH; el lanzador `py` queda como último recurso (solo cuando no hay ningún `python` en el PATH, que es el caso raro de una máquina con el Store stub). Verificado: con `py` arrancado sin consola, la ventana salía; con el intérprete propio, no aparece ninguna.

### WIN-03 + WIN-04 · el vídeo: el frame pegado y el ffmpeg que tarda dos segundos

**Síntoma:** al saltar en un vídeo, la imagen se quedaba **pegada en el último fotograma** del sitio anterior y tardaba tres segundos en volver al sitio bueno; a veces no se ponía en negro y a veces se quedaba allí.

**Causa:** dos cosas distintas. (1) `stream.rs` vaciaba la caché al reiniciar el decodificador pero **dejaba el frame viejo publicado**: la pantalla seguía con la imagen anterior hasta que llegaba la nueva. (2) Cada salto **arranca un ffmpeg nuevo**, y el build completo de gyan.dev (400 MB, con CUDA, Vulkan y libplacebo dentro) tarda **2.032 ms en arrancar**; medido con `-version`, contra los **80 ms** del build essentials (100 MB, los mismos decodificadores).

**Solución:** (1) mientras el destino no está en la caché, la imagen se pone **negra** y sale cuando llega el frame correcto; (2) el instalador empaqueta el build ligero y **mide su arranque al compilar**: si tarda más de 400 ms avisa con el enlace al build ligera.

**Medido en la app instalada** (con el watcher de ventanas capturando la zona del vídeo cada 60 ms): negro entre los 267 y los 367 ms, imagen correcta a los **464 ms**. Antes: ~3 s con el frame viejo pegado.

### PLUG-02 · el yt-dlp sin actualizar bajo Program Files

**Síntoma:** el programa de yt-dlp no venía en el instalador (lo bajaba el plugin al primer uso) y, cuando venía, `yt-dlp -U` **fallaba**: el binario estaba bajo `Program Files`, que es de solo lectura.

**Solución:** el instalador lo lleva dentro (`extension\plugin\yt-dlp\bin\yt-dlp.exe`) y el plugin **lo siembra** la primera vez que corre: si la copia del plugin no es escribible, se copia a `%LOCALAPPDATA%\openfader\plugins\yt-dlp\bin` y se usa esa, que sí se actualiza a diario. Verificado con el binario marcado como solo lectura: la copia aparece en la carpeta de datos y la acción «Actualizar yt-dlp ahora» responde *"yt-dlp: 2026.08.19; contra el canal stable: está al día"*.

### PLUG-05 · un plugin nombrado sin extensión no se encontraba

**Síntoma:** con el motor de Deezer dentro del instalador, el plugin no arrancaba: *«deezer: cannot start: El sistema no puede encontrar la ruta especificada»*.

**Causa:** el manifiesto nombra el programa `bin/deezer-engine` (sin extensión) y `resolve()` lo buscaba tal cual; al no existir, el proceso se lanzaba con ese nombre **relativo**, y la ruta de un hijo se resuelve contra **la carpeta de quien llama**, no contra la del hijo.

**Solución:** `resolve()` prueba también con `.exe` en Windows (con test), y el motor se compila con su extensión en el empaquetado.

### DEEZ-01..03 · el motor nativo de Deezer no llegaba a descargar

Tres fallos encadenados, encontrados al probar una descarga real con el ARL del usuario:

1. **«Invalid CSRF token»**: Deezer ata el token del gateway a la sesión, que viaja en cookies (`sid`, `dzr_uniq_id`, …). El cliente solo mandaba el ARL → la jarra de cookies de `http.rs` ahora guarda lo que el servidor pone y lo devuelve.
2. **«Deezer no dio un token para la pista»**: la ficha de la pista viene anidada en `results.DATA`; el motor la leía en la raíz.
3. **El token rota**: si el gateway lo rechaza, se abre sesión nueva y se reintenta una vez (lo que hace deemix).

**Verificado:** FLAC de 20,2 MB (904 kbps) y MP3 128 de 3,46 MB, y la app instalada responde *«deezer: ARL bueno (CL): FLAC y MP3 320»*.

### Lo que todo esto deja en pie

- Una máquina recién estrenada **no necesita nada más**: el instalador lleva la app, los plugins, los temas, los mapeos, un ffmpeg ligero, un Python, el programa de yt-dlp y el motor de Deezer.
- **Ni una ventana de consola** por delante de la aplicación, ni al buscar, ni al descargar, ni al elegir un fichero.
- Un salto de vídeo **responde en medio segundo** y con un negro limpio, no con un frame mentiroso.
- El **ARL** (la cookie de Deezer) vive solo en la carpeta de datos de la app: nunca se comite, nunca viaja en el instalador y nunca aparece en el historial de git (comprobado con el valor real sobre los cuatro repositorios, árboles e historiales).

---

## Verificación
## Verificación

```powershell
# sh está en Git para Windows: los tests de plugin-fichero lo necesitan en el PATH.
$env:PATH = 'C:\Program Files\Git\usr\bin;' + $env:PATH
cargo test --lib 2>&1            # → test result: ok. 580 passed; 0 failed (+2 de catalog::check)
cargo test --bin openfader 2>&1  # → test result: ok. 473 passed; 0 failed; 1 ignored (+6: skins cotas/schema/contraste, store .part, docs/mappings.md, los 3 de i18n/language)
cargo test --bin openfader -- midi:: 2>&1  # → 54/54 (el round-trip de los 176 XML de maps-repo incluido)
cargo test --bin openfader -- the_checked_out_themes 2>&1  # → 2/2 (integridad + contraste WCAG de los temas del checkout)
```

Notas de entorno (preexistentes, no del trabajo hecho):
- sin `sh` en el PATH, los 10 tests de plugins-fichero fallan con «cannot start "sh"» — es cómo son sus scripts de ejemplo.
- el test de integración `tests/folder` no compila aquí (`ort_sys` no disponible en rlib en este corazón de Windows); se probó con `--lib`/`--bin`, que sí compilan todo lo necesario de los módulos tocados.

## Commits

**Core (`fixes/hallazgos`)**
| Commit | Contenido |
|--------|-----------|
| `96ad82c` | CORE-01, CORE-02, CORE-04, CORE-05, CORE-07, CORE-08, CORE-09 (plugin/install + zip + lua + concur + evento/fuente + manifiesto) |
| `2b327d7` | CORE-06 (probe acotado + caché) |
| `cd51075` | MAP-01 (runtime), MAP-02, MAP-03, MAP-04 |
| `d66f8e1` | limpieza: `maps-repo` sale del índice del core (quedó como gitlink por error y se corrigió en el commit siguiente) |
| `aa31d3a` | MAP-07 + X-03 |
| `c12b87f` | THEME-02, THEME-03, THEME-04, THEME-01 (test WCAG), THEME-06 + THEME-05 documentados (`docs/skins.md`) |
| `af0e0aa` | MAP-06 + MAP-05 (`docs/mappings.md` completo + tablas con test) |
| `5108039` | X-01 (SHA256SUMS), CORE-03 + X-02 (Trust + aviso de terceros), nueva dependencia `sha2` |
| `70b233b` | repositorio de plugins: `docs/plugins.md` con el pin del SDK |
| `4b185f3` | X-04 (i18n: ajuste LANGUAGE, `@tr` en los menús + catálogos bundled, tabla `ES` para las páginas de preferencias) |
| `5d011c9` | THEME-05 (chip «ONLY CHANGES» en el editor de temas: guardar solo los tokens que difieren del base) |
| `c57f5a1` | MAP-05 docs (`docs/mappings.md`: páginas de pad de DDJ-1000/FLX10 atadas + resto vía MIDI LEARN) |

**Core, segunda tanda (`main`, sobre el 0.2.1 del original)**
| Commit | Contenido |
|--------|-----------|
| `570b72d` | WIN-02: los ayudantes de consola arrancan sin consola (`CREATE_NO_WINDOW \| DETACHED_PROCESS`) |
| `02864d8` | DATA-01: las pistas de una fuente de plugin viven en la Colección (migración v16) y el navegador marca las descargadas |
| `2eb82bf` | PLUG-03: el gestor enseña la carpeta de origen, avisa de las copias ignoradas y compara versiones por número |
| `7cdc0a4` | UI-03: el escenario VDJ (`stage-layout`), una pantalla por mesa y el vídeo en ellas |
| `525689b` | PACK-01: el instalador de Windows (NSIS) con todo dentro |
| `9d908dc` | WIN-01: los plugins usan el intérprete que viaja con el programa, no el lanzador `py` |
| `734797e` | WIN-03 + WIN-04: el salto de vídeo se pone en negro y el ffmpeg es el build ligero |
| `e0f280a` | PLUG-05: un programa nombrado sin extensión se encuentra igual (con test) |
| `1d9f8c9` | PACK-02: el motor de Deezer viaja en el instalador |
| `d6d1b9f` | PACK-02: el motor recién compilado va al payload (la copia en la app puede estar en uso) |
| `5289290` | UI-02: las preferencias nuevas del original en español + el `hint` de las filas se traduce; el README dice cómo poner un ffmpeg ligero |
| `b28cf41` | UI-01: la ventana abre a pantalla completa (ajuste recordable) y el menú Ver la enciende y la apaga |
| `975576a` | DOC-01: el README cuenta esta versión, en inglés y en español (`README.es.md`); versión 0.2.2 |

**OpenFadeR-Map (`maps-repo`)**
| Rama | Commit | Contenido |
|------|--------|-----------|
| `fixes/duplicated-jog-bend` | `1b2fdba` | MAP-01 datos: 13 archivos, 66 líneas `jog-bend` duplicadas eliminadas |
| `fixes/map05-actions` | `18cad36` | MAP-05 datos: DDJ-1000/FLX10 atán los pads BEAT JUMP/BEAT LOOP/KEY SHIFT a las 10 acciones sin uso |

**OpenFadeR-Plugins (`plugins-repo`, rama `fixes/sdk-pin`)**
| Commit | Contenido |
|--------|-----------|
| `47de0ff` | PLUG-01: `openfader-plugin` fijado al tag `v0.1.0` del core (compilación autónoma + parche local documentado) |

## Impacto en el usuario

- **Scratch restaurado** en las 13 controladoras XDJ/OMNIS afectadas.
- Un mapeo XML mal copiado **ya no deshabilita el resto**; los avisos llegan a la línea de estado de MIDI.
- Instalar/actualizar plugins **no puede** perder la versión anterior por un fallo a mitad (disco lleno, antivirus, DLL cargada).
- Un plugin hostil ya no puede: salir de su carpeta de instalación con el manifiesto, comerse memoria con un zip bomba, ni fabricar cientos de hilos disparando eventos; un plugin colgado ya no congela la UI ni el escaneo del navegador (el probe y `rk.run`, que antes sí podían congelarlos).
- Un `plugin.json` con un typo en una clave lo dice al arrancar (stderr) en vez de instalar «a medias».
- Un mapeo instalado desde un repositorio trae su `audio-cards.json` y el fichero del usuario se respeta.
- Un tema escrito a mano ya no puede dejar la UI inservible con `font-size: -5` o alturas de 10 000 px (se recortan con nota), y el formato trae `schemaVersion` para evolucionarlo sin ambigüedad; los que el editor guarda van firmados a traves de `.part`+rename (un corte de luz ya no trunca su JSON).
- Los temas del repositorio se miden por contraste WCAG en CI: un tema publicado con texto ilegible da la cara antes.
- Lo que descargan GET PLUGINS/THEMES/MAPPINGS se verifica contra el `SHA256SUMS` del repositorio cuando lo publica; un zip sustituido se rechaza con un error que dice qué hash se esperaba, y al instalar de un repositorio ajeno el console log desmiente el silencio.
- El modelo de confianza (qué puede un plugin, cómo se elige un repo, qué comprueba la app) está escrito para el usuario en `docs/plugins.md`; el build de los plugins ya no pide un checkout del core.
- **La interfaz habla español**: en un Windows/Linux con locale español, los menús de la barra y las 10 páginas de preferencias aparecen en español sin configurar nada; quien quiera inglés (o forzar el español) lo elige en PREFERENCES > VIEW > LANGUAGE. Cambiar el idioma es en caliente; nada sin traducir queda a medias (vuelve a su inglés original).

### De la segunda tanda

- **Ni una ventana de consola** delante de la aplicación: ni al buscar en YouTube, ni al abrir el gestor de plugins, ni al elegir un fichero.
- Un **salto de vídeo responde en medio segundo** y con un negro limpio, en vez de quedarse tres segundos con el frame del sitio anterior.
- Una máquina **recién estrenada no necesita nada más**: el instalador lleva la app, los plugins, los temas, los mapeos, un ffmpeg ligero, un Python, el programa de yt-dlp y el motor de Deezer.
- El yt-dlp **se actualiza solo** aunque la app viva en `Program Files` (usa la copia que guarda en tu carpeta de usuario).
- **Deezer descarga**: FLAC o MP3 320 con tu ARL, sin Node, sin deemix y sin Python por medio.
- Una pista de YouTube o Deezer **puede estar en una lista o en una paleta** como cualquier otra, y el navegador marca con un ✓ las que ya están descargadas.
- El **escenario VDJ**: la franja de FX y la pantalla maestra bajo las mesas, una pantalla por mesa y los faders de canal con su longitud.
- La ventana **abre a pantalla completa**, y si la devuelves a modo ventana, la próxima vez también.
- El **ARL** (la cookie de Deezer) no está en ningún repositorio: vive solo en la carpeta de datos de la app, y se comprobó con el valor real sobre los cuatro repositorios (árboles e historiales).
- El README cuenta **qué añade esta versión**, en inglés y en español.
### De la tercera tanda (el browser)

- **La sección "Downloaded"**: lo que los plugins de nube descargan se ve como una fuente más del árbol, con sus ficheros reales (se cargan, se importan y se buscan).
- **"Not in the Collection"**: cada fuente abierta dice qué ficheros suyos no tiene aún la Colección, y un botón los añade todos de golpe.
- **Un USB sin biblioteca de DJ** aparece en "Devices" y lista sus carpetas de primer nivel; cada carpeta se abre como fuente.
- **Las carpetas fijadas (FAV)**: el botón FAV fija la carpeta elegida, la sección "Favorites" las lista y un pin sobrevive a que el USB cambie de letra.
- **El filtro de medio (ALL / AUDIO / VIDEO)** y la columna **FORMATO** (nota musical o película) dicen de qué es cada pista.
- **Una sola búsqueda**: la caja del buscador mira la Colección, el índice de las carpetas fijadas y las fuentes leídas, y cada resultado dice de dónde salió (columna ORIGIN). Las búsquedas en línea de YouTube y Deezer no se disparan desde el filtro.
- **Las pistas de nube ya no se loguean dos veces**: entran en la Colección antes de sonar, así que el historial, el color de "reproducida" y las listas las nombran igual.
- **Las descargas de YouTube dicen el título, no el id**: el plugin graba las etiquetas en el fichero que descarga (título, artista, álbum) y la app las lee; para lo ya descargado, el plugin responde con el título de cada id (operación info).
- **El árbol pliega sus ramas**: un clic en una lista con listas debajo las pliega (y las despliega otra vez); el teclado solo mueve la selección. Las secciones (Favorites, Downloaded, Devices, Explorer) se muestran u ocultan desde el botón SECTIONS del browser, no solo desde Preferencias.

### De la cuarta tanda (los volúmenes)

- **Un disco duro USB ya aparece en "Devices"**: Windows responde `DRIVE_FIXED` a un disco externo (solo responde `DRIVE_REMOVABLE` a un pendrive o una tarjeta), así que el escaneo ahora acepta los dos tipos y deja fuera el volumen del sistema.
- **El disco local se puede recorrer**: el volumen donde está Windows (C:) se lista en la misma sección como **C: (OS)**, y sus carpetas de primer nivel se abren como fuentes de solo lectura, para navegar entre las carpetas del disco.
- **Listar un volumen no lo barre entero**: antes se recorría el árbol entero del disco buscando audio (1,8 TB = minutos); ahora solo se lee el primer nivel (instantáneo) y las carpetas se abren bajo demanda.
- **Se ven todas las carpetas del volumen**, no solo las que tienen audio: se puede navegar por el disco entero, no solo por su música.

### De la quinta tanda (21/10/2026): el navegador - HECHO

- **Columna BITRATE**: la tabla de tracks muestra el bitrate del archivo ("320 kbps", "256 kbps", etc.) a la derecha de FORMATO; queda vacío cuando el bitrate es desconocido (0).
- **Secciones plegables del árbol**: un clic en History, Tags, Devices, Downloaded, Música Local o Música en Línea pliega/despliega sus hijos (el ícono cambia entre ▸ y ▾); el teclado solo mueve la selección.
- **History con botón derecho**: renombrar sesión (mantiene la fecha como clave interna) y eliminar sesión.
- **Related tracks con auto-ancla**: si no hay track seleccionado, Related toma el primero de la lista visible en lugar de mostrar vacío.
- **Instalador v0.3.1**: NSIS 3.11 disponible, instalador compilado y subido a la release v0.3.1 en GitHub (ya no es borrador).

### De la sexta tanda (22/10/2026): el navegador y el reloj - HECHO

- **Filtro por origen**: ALL / COLLECTION / YOUTUBE / DEEZER en la barra del navegador; la Colección es lo que no viene de un plugin de nube.
- **La estrella de una carpeta fijada se pulsa**: un clic en su fila la fija o la quita, sin tener que seleccionarla antes.
- **Aviso de duplicado al importar**: si la pista ya está en la Colección, la carga lo dice en vez de añadirla otra vez.
- **Editar los detalles de una pista**: título, artista, álbum, género, sello y comentario desde el panel de la pista; se guarda en la Colección y se ve al momento en la lista y en el panel.
- **Los stems no se separan dos veces**: al recargar la misma pista (o un INSTANT DOUBLE) se reutilizan los stems que ya están en memoria si la caché encaja en frames y rejilla.
- **Memory cues como barras anchas con bandera M**: se distinguen de los hot cues de un vistazo y usan el color `Theme.memory-cue` del tema.
- **Salida de reloj MIDI**: PREFERENCIAS > CONTROLADOR > OTHERS > MIDI CLOCK OUT toma parte del nombre de un puerto de salida y manda 24 PPQN al tempo y beat de la mesa líder, START en su siguiente compás y STOP cuando no suena ninguna; si el aparato se desconecta, el hilo reabre el puerto solo.
- **El árbol habla un solo idioma**: las secciones y los mensajes del navegador pasan a inglés (Local Music, Devices, Online Music, Search, Downloads, Favorites, Downloaded), y las casillas de Preferencias llevan su traducción al español en la tabla de idioma; las carpetas del sistema se llaman Music / Videos / Desktop.
- **Tests**: los del navegador vuelven a pasar (árbol plegable, secciones Local Music / Devices / Online Music / Favorites y origen de la Colección); `cargo test --bin` queda en verde.

### De la séptima tanda (22/10/2026): SYNC y memoria - HECHO

- **SYNC engancha de una vez**: los candados de tempo y fase se toman juntos y la corrección se queda dentro de 1 ms, sin el tirón que se notaba al sincronizar dos mesas (`6f43923`).
- **Cada pista vive una sola vez, en 16 bits**: decks, preview del navegador y análisis en bloque decodifican directamente a `Track16` (10,6 MB por minuto de estéreo a 44,1 kHz) y ya no construyen una copia f32 de la pista entera; la huella se calcula en streaming mientras se decodifica (`695bcd3`).
- **El análisis lee a través de `audio::Frames`**: `analyze`, grid, waveform, tonalidad y voz aceptan tanto el PCM f32 como el `Track16` de un deck, sin copiarlo; una pista sin caché se analiza en el acto y el resultado se guarda igual que antes.
- **La memoria vuelve al sistema**: en glibc, `mallopt` fija el umbral de mmap y poda los arenas de los hilos, así que los buffers de una pista descargada no se quedan en el heap (medido por el original: 274 MB → 203 MB con dos pistas de 6 minutos).
- **STEMS en CPU y build para Raspberry Pi**: el feature `stems-gpu` (WebGPU) es opcional; `--no-default-features --features app` compila STEMS solo con CPU, y el perfil `pi` + `Dockerfile.pi` producen el archivo aarch64 para Pi 3/4/CM4 (Debian 13). El perfil `dist` ya recorta los símbolos.
- **Verificación**: release nuevo, «The War Song» cargada y reproducida (117,18 BPM, 9B, auto-gain, waveform, STEMS al 100 %), análisis fresco de esa pista sobre `Track16` (caché reescrita, sin errores) y las cinco vistas abiertas: SAMPLER, BEAT LAB, EDIT, VIDEO (BETA) y LIGHTING (Art-Net a 127.0.0.1 a 40 Hz).
- **Tests**: `cargo test --bin openfader` 491 ok (1 ignorado); `cargo test --lib` 577 ok y los 13 fallos preexistentes de `plugin::` (`sh` en el PATH).

