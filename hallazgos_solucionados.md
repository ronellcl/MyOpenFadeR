# Hallazgos solucionados — registro de reparaciones

**Fecha:** 17/10/2026 · **Rama del core:** `fixes/hallazgos` (desde `main` @ `dbbe938`) · **Rama de mapeos:** `fixes/duplicated-jog-bend` en `maps-repo/` (desde su `main`) · **Rama de plugins:** `fixes/sdk-pin` en `plugins-repo/` (desde su `main`)
**Fuente:** cada apartado corresponde a un hallazgo de `hallazgos.md` (mismo ID). Se resolvió todo lo que no afecta el funcionamiento, según lo acordado.
**Verificación:** suite completa del core verde — `cargo test --lib` **580/580** y `cargo test --bin openfader` **470/470** (1 ignorado, preexistente).

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
| MAP-05 | Acciones sin uso en los mapeos | ✅ Resuelto (documentado) | `docs/mappings.md`: la tabla completa está y se usa también por MIDI LEARN | `af0e0aa` |
| MAP-06 | Documentación de mapeos de 24 líneas | ✅ Resuelto | Especificación completa + tablas generadas de acciones/luces con test + `audio-cards.json` | `af0e0aa` |
| X-01 | Descargas sin digest verificado | ✅ Resuelto | `SHA256SUMS` por release se verifica al bajar (opt-in; sin lista → aviso) | `5108039` |
| CORE-03 | Plugins sin aislamiento (modelo de confianza implícito) | ✅ Resuelto (documentación + aviso) | Sección «Trust» en `docs/plugins.md` + aviso stderr al instalar de terceros | `5108039` |
| X-02 | «GitHub como registro» sin modelo de confianza escrito | ✅ Resuelto (documentado) | Mismo apartado «Trust»: confianza por repositorio, los `OpenRsG/*` curados | `5108039` |
| PLUG-01 | Dependencia del SDK por ruta sin pin | ✅ Resuelto | `openfader-plugin` fijado al tag `v0.1.0` del core: el repo de plugins compila solo · override local documentado | plugins `47de0ff` + core `70b233b` |

Pendientes que requieren decisiones de producto o trabajo aparte, fuera de este documento: **THEME-05** (edición opt-in «guardar solo los tokens cambiados»: decisión de UI), **X-04 fase 2** (i18n: detección del locale del sistema + «override» de ajuste; el trabajo grande con compilación de la interfaz de usuario no ha empezado).

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
**Solución** (documental, como pide un hallazgo *informativo*): la tabla completa de `docs/mappings.md` muestra que las 23 acciones sin XML (`brake`, `grid-*`, `key-down/up`, `loop-1/2/8/16`, `jump-*`, `zoom-in/out`…) **siguen al alcance del usuario por MIDI LEARN** — el hallazgo sugería mappearlas en los controladores avanzados; eso es un cambio de datos del repo de mapas y queda como invitación abierta. El motor no cambió.

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

---

## Verificación

```powershell
# sh está en Git para Windows: los tests de plugin-fichero lo necesitan en el PATH.
$env:PATH = 'C:\Program Files\Git\usr\bin;' + $env:PATH
cargo test --lib 2>&1            # → test result: ok. 580 passed; 0 failed (+2 de catalog::check)
cargo test --bin openfader 2>&1  # → test result: ok. 470 passed; 0 failed; 1 ignored (+3: skins cotas/schema/contraste, store .part, docs/mappings.md)
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

**OpenFadeR-Map (`maps-repo`, rama `fixes/duplicated-jog-bend`)**
| Commit | Contenido |
|--------|-----------|
| `1b2fdba` | MAP-01 datos: 13 archivos, 66 líneas `jog-bend` duplicadas eliminadas |

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
