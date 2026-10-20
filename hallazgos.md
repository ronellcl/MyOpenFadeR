# Hallazgos consolidados — OpenFadeR

**Fecha:** 2026-10-16
**Método:** análisis estático (sin compilar ni ejecutar) del core, `OpenFadeR-Plugins`, `OpenFadeR-Map` y `OpenFadeR-Themes`, contrastado con `docs/plugins.md`, `docs/mappings.md` y `docs/skins.md`.
**Versiones analizadas:** core `dbbe938` · plugins `f02eff7` · map `1024703` · themes `4c93b8c`.

## Resumen

| ID | Origen | Severidad | Hallazgo |
|---|---|---|---|
| CORE-01 | core (`src/plugin/install.rs`) | **Crítica** | Path traversal en instalación de plugins (`"name": ".."` borra `<config>/extension`) |
| CORE-02 | core (`src/plugin/`) | Alta | DLL de Windows bloqueada al actualizar/desinstalar un plugin nativo cargado |
| CORE-03 | core (arquitectura) | Media | Plugins sin aislamiento (acceso total al proceso y al sistema) |
| CORE-04 | core (`src/plugin/lua.rs`) | Media | Timeouts que no cubren `rk.run` ni el join del hilo |
| CORE-05 | core (`src/plugin/`) | Media | Concurrencia sin límite (un hilo por evento/fuente, sin tope) |
| CORE-06 | core (`src/plugin/library/mod.rs`) | Media | Probes de librerías síncronos y sin timeout |
| CORE-07 | core (`src/plugin/install.rs`) | Media | Actualización no transaccional: el destino se borra antes de que el rename tenga éxito |
| CORE-08 | core (`src/zip.rs`) | Media | Lectura de zip sin topes (bombas de descompresión) |
| CORE-09 | core (`src/plugin/mod.rs`) | Baja | `plugin.json` acepta claves desconocidas en silencio |
| PLUG-01 | OpenFadeR-Plugins (`Cargo.toml`) | Baja | Dependencia por ruta `../../sdk` sin versión: build no autónomo y sin pin |
| MAP-01 | OpenFadeR-Map (13 XML) | **Alta** | Duplicados `jog`/`jog-bend` sobre el mismo CC: el scratch no funciona en 13 controladoras |
| MAP-02 | core (`src/app/midi/mapfile.rs`) | Media | Un solo XML roto en cualquier carpeta de mapeos aborta la carga de todos |
| MAP-03 | core (`src/app/midi/mapfile.rs`) | Baja | Parser silencioso: etiquetas/atributos desconocidos y `schemaVersion` sin validar |
| MAP-04 | core + OpenFadeR-Map (CI) | Baja | El test de round-trip no cubre el mapa runtime (no detecta MAP-01) |
| MAP-05 | OpenFadeR-Map | Informativa | 23 acciones del core sin uso en ningún mapeo |
| MAP-06 | core (`docs/mappings.md`) | Baja | Documentación de mapeos de 24 líneas; `audio-cards.json` sin documentar |
| MAP-07 | core (`src/catalog/mappings.rs`) | Baja | GET MAPPINGS solo instala `.xml`; no distribuye `audio-cards.json` |
| THEME-01 | OpenFadeR-Themes + core | Media | Sin validación de contraste: un tema publicado puede quedar ilegible |
| THEME-02 | core (`src/app/skin/file.rs`) | Baja | Longitudes negativas aceptadas (sin cotas) |
| THEME-03 | Formato de temas | Baja | Sin `schemaVersion` |
| THEME-04 | core (`src/app/skin/store.rs`) | Baja | Guardado de temas no atómico |
| THEME-05 | core (`src/app/skin/`) | Diseño | SAVE AS serializa los 120 tokens: los temas no heredan mejoras futuras del base |
| THEME-06 | core (`src/app/prefs/`) | Baja | Los hints de layout solo se aplican al seleccionar, no al arrancar |
| X-01 | Transversal (4 repos) | Media | Sin checksums ni firmas en la distribución |
| X-02 | Transversal | Informativa | "GitHub como registro": sin revisión central ni semver obligatorio |
| X-03 | core (`src/app/midi/get.rs`) | Baja | `.part` residual si falla la instalación de un mapeo |

**Escala:** Crítica (pérdida de datos / explotable) · Alta (funcionalidad rota para usuarios reales) · Media (robustez / fiabilidad) · Baja (higiene, DX, docs) · Informativa (observación sin acción obligatoria).

---

## 1. Core (`OpenFadeR`)

### CORE-01 — Path traversal en la instalación de plugins
- **Severidad:** Crítica (único hallazgo bloqueante).
- **Descripción:** `folder_name` convierte el `name` de `plugin.json` en nombre de carpeta y acepta `.` y `..` (el punto está en la allowlist de caracteres). `install_into` resuelve `dest = root.join(folder_name(...))` y, si existe, ejecuta `remove_dir_all(&dest)` antes del rename. Un zip/hostil con `{"name": ".."}` hace que `dest` sea la propia raíz de extensiones y el instalador **borra `<config>/extension` entera** (plugins, temas y datos instalados) antes de ejecutar nada del plugin. Con más manipulación del manifiesto la escritura puede salir de la raíz.
- **Evidencia:** `src/plugin/install.rs:54-76` (`folder_name` en 66-76 acepta `.`; `remove_dir_all(&dest)` en 55-57) y `src/plugin/mod.rs:383` (el name se toma del manifiesto sin validar).
- **Posible solución:** rechazar `.` y `..`, prohibir nombres que empiecen por punto y limitar a letras/dígitos/espacio/`_`/`-`; además, comprobar que el destino canónico queda contenido en la raíz antes de tocar nada.
- **¿Afecta el funcionamiento?** No. Solo rechaza nombres patológicos; los 21 plugins existentes usan nombres válidos y el flujo normal de instalación no cambia.

### CORE-02 — DLL bloqueada al actualizar/desinstalar un plugin nativo cargado
- **Severidad:** Alta.
- **Descripción:** en Windows, una `.dll` cargada permanece mapeada y el SO impide borrarla o sobrescribirla. Al actualizar o desinstalar un plugin de librería nativa que la app ya cargó, el `remove_dir_all`/rename de `install.rs` falla y la instalación queda a medias (carpeta vieja inservible + nueva sin aplicar).
- **Evidencia:** `src/plugin/install.rs:55-58` (borrado y rename directos del destino) combinado con el loader de `src/plugin/library/mod.rs` (la librería permanece cargada hasta salir).
- **Posible solución:** patrón estándar de Windows: renombrar la carpeta antigua a `.pending-delete-<pid>` (el rename sí está permitido con la DLL mapeada), completar la instalación y purgar las carpetas pendientes en el siguiente arranque.
- **¿Afecta el funcionamiento?** No. Lo mejora: hoy la actualización falla o deja estado inconsistente; con el fix requiere el reinicio que ya exigía el ciclo de plugins nativos.

### CORE-03 — Plugins sin aislamiento
- **Severidad:** Media.
- **Descripción:** los plugins nativos corren en el proceso (ABI C) y los scripts (Lua/Python) tienen acceso a ficheros y red sin sandbox ni capa de permisos. Un plugin malicioso o con bugs puede leer/escribir cualquier cosa con los permisos del usuario o tirar el proceso de audio.
- **Evidencia:** arquitectura de `src/plugin/library/` (in-process), `sdk/src/library/` y el protocolo de scripts de `src/plugin/lua.rs` (sin restricciones de entorno documentadas); `docs/plugins.md` describe el modelo de confianza solo implícitamente.
- **Posible solución:** corto plazo, documentar el modelo de confianza y mostrar un aviso al instalar plugins de terceros; medio plazo, mover scripts a procesos separados con perfiles de permisos.
- **¿Afecta el funcionamiento?** Documentar/avisar, no. El aislamiento real sí: cambia rendimiento y la ABI de librerías (probablemente inviable para efectos de audio en tiempo real); es una decisión de diseño, no un parche.

### CORE-04 — Timeouts que no cubren `rk.run` ni el join
- **Severidad:** Media.
- **Descripción:** el deadline de los scripts cubre la llamada Lua principal, pero un script colgado dentro de un callback `rk.run`, o el join del hilo worker al finalizar, pueden bloquear más allá del timeout.
- **Evidencia:** `src/plugin/lua.rs:35-46` (plumbing del timeout) y el estudio del protocolo de scripts (el deadline no rodea `rk.run` ni el join del hilo lanzado en `event.rs:55`).
- **Posible solución:** aplicar el deadline también alrededor de la ejecución de callbacks `rk.*` y usar joins con espera acotada (o abandono del hilo con recreación del intérprete).
- **¿Afecta el funcionamiento?** No. Solo recorta ejecuciones ya consideradas colgadas; los scripts bien comportados no notan nada.

### CORE-05 — Concurrencia sin límite
- **Severidad:** Media.
- **Descripción:** cada evento de plugin y cada escaneo de fuente lanza su propio hilo, y cada llamada a proceso abre pares de hilos writer/reader, sin pool ni semáforo. Una ráfaga de eventos (o un plugin que dispara eventos en bucle) puede crear decenas de hilos/procesos simultáneos.
- **Evidencia:** `src/plugin/event.rs:55`, `src/plugin/mod.rs:495-497`, `src/plugin/source/mod.rs:135`; no hay `Semaphore`, pool ni cola en `src/plugin/`.
- **Posible solución:** limitar con un semáforo/cola de trabajo por tipo (eventos, fuentes) con un máximo configurable.
- **¿Afecta el funcionamiento?** Solo en cargas extremas: los eventos que llegan con el tope agotado se encolarían o saltarían. Con un límite generoso y una política de "descartar el más nuevo", el comportamiento visible no cambia.

### CORE-06 — Probes síncronos de librerías
- **Severidad:** Media.
- **Descripción:** el probe de una librería (cargar la `.so`/`.dll` y pedirle sus tablas) se ejecuta de forma síncrona en el hilo que lista/valida plugins. Una librería lenta o colgada en su `probe` bloquea la operación y, si es el hilo de UI, la interfaz.
- **Evidencia:** `src/plugin/library/mod.rs:88` (`fn probe`) y `:274` (llamada directa, sin hilo worker ni timeout en el módulo).
- **Posible solución:** ejecutar el probe en un worker con timeout y cachear el resultado por (ruta, mtime).
- **¿Afecta el funcionamiento?** No. Mejora la responsividad; el resultado es el mismo.

### CORE-07 — Actualización no transaccional
- **Severidad:** Media.
- **Descripción:** hay staging (`.installing-<pid>`), pero el destino se elimina **antes** del rename. Cualquier fallo entre el `remove_dir_all` y el `rename` exitoso (disco lleno, fichero bloqueado — véase CORE-02, antivirus) deja al usuario sin la versión anterior del plugin.
- **Evidencia:** `src/plugin/install.rs:52-62`: `read_manifest` sobre staging, `remove_dir_all(&dest)` en 55-57 y `rename` en 58; el `_ = remove_dir_all(&staging)` final no recupera el destino borrado.
- **Posible solución:** no borrar el destino: renombrar el destino viejo a `.old-<pid>`, hacer el rename de staging a destino y purgar `.old-*` al arrancar. La validación completa (manifiesto + ABI) debería ocurrir sobre staging antes de tocar nada.
- **¿Afecta el funcionamiento?** No. Mismo resultado en el caso feliz; añade recuperación en el caso de fallo.

### CORE-08 — Zip sin topes
- **Severidad:** Media.
- **Descripción:** el lector de zip usado por el instalador de plugins no tiene límites de entradas, tamaño descomprimido ni profundidad, así que un zip bomba o un zip con miles de entradas puede agotar disco/memoria durante la instalación.
- **Evidencia:** `src/zip.rs` sin constantes de límite (grep de `MAX|LIMIT|DEPTH` sin resultados), en contraste con `src/catalog/mappings.rs:17-27`, que sí usa `MAX_FILE` (4 MiB) y `DEPTH` para los mapeos.
- **Posible solución:** replicar el patrón del catálogo: tope de tamaño por entrada y total, número de entradas y profundidad, con error claro.
- **¿Afecta el funcionamiento?** No. Los zips de plugins reales están muy por debajo de cualquier tope razonable.

### CORE-09 — Manifiesto permisivo
- **Severidad:** Baja.
- **Descripción:** `plugin.json` acepta claves desconocidas sin aviso; una typo (`"efects"`) deja la sección sin aplicar y el plugin parece instalado "a medias" sin diagnóstico.
- **Evidencia:** parseo de `Manifest` en `src/plugin/mod.rs` (campos conocidos + resto ignorado). Contraste: el parser de temas sí acumula notas de claves desconocidas (`src/app/skin/file.rs:156`).
- **Posible solución:** acumular notas de claves desconocidas (warning en stderr y en el manager), no error duro.
- **¿Afecta el funcionamiento?** No (si es warning). Un rechazo estricto sí podría romper plugins existentes con claves a futuro; por eso se recomienda avisar y no fallar.

---

## 2. Plugins (`OpenFadeR-Plugins`)

### PLUG-01 — Build dependiente de `../../sdk` sin versión
- **Severidad:** Baja.
- **Descripción:** el workspace de plugins referencia el SDK por ruta relativa al checkout del core. Clonar solo `OpenFadeR-Plugins` no compila; y como no hay versión/pin, un cambio incompatible en `sdk/` rompe el build de todos los plugins de golpe y de forma silenciosa hasta que alguien compila.
- **Evidencia:** `plugins-repo/Cargo.toml:6` — `openfader-plugin = { path = "../../sdk", features = ["library"] }`; no hay `rev`/`tag`/versión publicada.
- **Posible solución:** publicar el crate `openfader-plugin` (crates.io o dependencia git con `tag`) y usar ese pin en el repo de plugins, manteniendo el override por `[patch]` para desarrollo local.
- **¿Afecta el funcionamiento?** No en runtime. Cambia el flujo de build/CI de los plugins (mejora reproducibilidad); el binario resultante es idéntico.

---

## 3. Mapeos (`OpenFadeR-Map` + parser del core)

### MAP-01 — Duplicados `jog`/`jog-bend`: scratch roto en 13 controladoras
- **Severidad:** Alta.
- **Descripción:** 13 archivos XML declaran dos `<control>` con el mismo `(status, midino)`: primero `jog` (scratch) y después `jog-bend`. El runtime colapsa los bindings en un `HashMap` donde **la última entrada gana**, así que el binding efectivo del platter es `jog-bend`, cuyo modo `Bend` ignora los ticks mientras hay touch activo. Resultado: tocar el platter inicia el estado de scratch pero girar no mueve el playhead — **el scratch no funciona** en esas controladoras (66 controles duplicados en total).
- **Evidencia:** datos: `OMNIS-DUO.xml:12-19`, `XDJ-RX3.xml:29-36` y otros 11 archivos (`Pioneer_DJ_XDJ-RR*`, `Pioneer_DJ_XDJ-RX2*`, `PIONEER_XDJ-AERO*`, `PIONEER_XDJ-R1`, `PIONEER_XDJ-RX*`, `XDJ-AN`, `XDJ-XZ__1_`). Runtime: `src/app/midi/session.rs:207-208` (`...collect()` a `HashMap`, último gana), `src/app/midi/input.rs:14-20` (`find` devuelve un solo binding), `src/app/midi/wheel.rs:18` (`Bend => jog.bend`), `src/app/midi/jog.rs:141-144` (`bend` no hace nada scratching).
- **Posible solución:** (a) eliminar las líneas `jog-bend` duplicadas en los 13 XML — el modelo `Jog::turn` ya alterna scratch/bend según el touch, así que `jog-bend` sobre el CC del platter es redundante; (b) añadir en el core un test que rechace claves `(status, midino)` duplicadas por archivo; (c) opcionalmente, que el binder avise y prefiera `jog` cuando haya colisión.
- **¿Afecta el funcionamiento?** Lo restaura y no tiene efectos colaterales: el binding que hoy gana (`jog-bend` sobre el platter) no produce scratch ni bend mientras se toca el platter, así que cualquier cambio solo puede mejorar. Los controladores con anillo separado ya mapean `jog-bend` en otro CC y no se ven afectados.

### MAP-02 — Un XML roto desactiva todos los mapeos
- **Severidad:** Media.
- **Descripción:** `pick` parsea todos los `.xml` de todas las carpetas de mapeos y propaga el primer error con `?`. Basta un archivo malformado (o un XML cualquiera sin `<name>`) soltado a mano en `<config>/mappings` para que **ninguna** controladora cargue mapeo, aunque sus archivos estén bien.
- **Evidencia:** `src/app/midi/mapfile.rs:270-297` (bucle con `parse(...)?`); `files()` en `:250-268` incluye todo `.xml` de la carpeta.
- **Posible solución:** acumular errores por archivo, saltar el roto con un mensaje en el estado de MIDI y continuar; parsear primero los candidatos cuyo texto contenga `<OpenFadeRMapping`.
- **¿Afecta el funcionamiento?** No. Los archivos válidos se siguen cargando igual; solo deja de penalizarlos un archivo ajeno.

### MAP-03 — Parser silencioso
- **Severidad:** Baja.
- **Descripción:** el parser ignora etiquetas y atributos desconocidos sin avisar (un `<controll>` produce un archivo sin bindings y sin diagnóstico), no valida `schemaVersion` (un `"2"` futuro se parsearía como v1) y acepta `deck` fuera de 0..3 (se ignora en runtime sin aviso).
- **Evidencia:** `src/app/midi/mapfile.rs:135-186` (brazos `_ => {}`; `deck` sin rango) y ausencia de lectura de `schemaVersion`.
- **Posible solución:** avisar de elementos/atributos desconocidos, validar `deck` 0..3 en parse y rechazar (o avisar) `schemaVersion` ≠ 1. Las luces y acciones desconocidas ya van a `m.bad`.
- **¿Afecta el funcionamiento?** No. Solo convierte fallos silenciosos en diagnósticos; los 176 archivos actuales no tienen elementos desconocidos ni decks fuera de rango.

### MAP-04 — Test de mapeos que no cubre el mapa runtime
- **Severidad:** Baja.
- **Descripción:** el CI del repo de mapas ejecuta el test del core de round-trip (parsear y reescribir el XML), que valida el *archivo* pero no el *mapa runtime* que colapsa duplicados. Por eso MAP-01 pasó el CI.
- **Evidencia:** `src/app/midi/mapfile.rs:317-330` (`every_checked_out_mapping_reads_back_to_the_same_file`): los duplicados se preservan en `binds` y en el `write`, así que el test pasa.
- **Posible solución:** añadir un test a nivel de `Midi::load_mapping`/`file_binds` que verifique que no hay colisiones de clave, o mover la comprobación de duplicados a `parse`.
- **¿Afecta el funcionamiento?** No; es solo test.

### MAP-05 — Acciones del core sin uso en mapeos
- **Severidad:** Informativa.
- **Descripción:** 23 de las 145 acciones de la tabla `ACTIONS` no aparecen en ningún XML: `brake`, `fx-level`, `fx-toggle`, `grid-beat1`, `grid-double`, `grid-edit`, `grid-halve`, `grid-reset`, `help`, `jump-back-1`, `jump-back-4`, `jump-fwd-1`, `jump-fwd-4`, `key-down`, `key-up`, `loop-1`, `loop-16`, `loop-2`, `loop-8`, `pad-mode`, `split`, `zoom-in`, `zoom-out`. Varias son muy útiles en hardware (beat jumps, key shift, loops por beats).
- **Evidencia:** extracción de `learn.rs`/`mapped.rs` (145 acciones) frente a los valores `action="..."` de los 176 XML (122 usadas).
- **Posible solución:** revisar si esos controles pueden mapearse en los controladores avanzados (DDJ-1000/FLX10, REV7) y documentar la lista como "solo MIDI LEARN".
- **¿Afecta el funcionamiento?** No; añadiría mapeos, no cambiaría el motor.

### MAP-06 — Documentación de mapeos insuficiente
- **Severidad:** Baja.
- **Descripción:** `docs/mappings.md` tiene 24 líneas y delega la especificación al comentario cabecera de `mapfile.rs`; no hay tablas de acciones ni de luces, ni mención a `audio-cards.json` (que solo documenta el README del repo de mapas).
- **Evidencia:** `docs/mappings.md` (24 líneas) frente a `docs/skins.md` (340 líneas con tablas completas).
- **Posible solución:** promover el comentario de `mapfile.rs` a `docs/mappings.md`, generar las tablas de acciones/luces desde el código (con el mismo patrón de test que usa `docs/skins.md` en `src/app/skin/mod.rs:463-469`) y documentar `audio-cards.json`.
- **¿Afecta el funcionamiento?** No.

### MAP-07 — `audio-cards.json` fuera de GET MAPPINGS
- **Severidad:** Baja.
- **Descripción:** el catálogo de mapeos solo escanea e instala `.xml`; `audio-cards.json` (tarjetas con auriculares en los canales 3/4) no se distribuye por GET MAPPINGS. Funciona porque el release empaqueta el repo completo en `mappings/`, pero quien instale mapeos desde un repo propio no obtiene el fichero.
- **Evidencia:** `src/catalog/mappings.rs:20-27` (filtro `ends_with(".xml")`), `src/audio/bus.rs:24-33` (lee `audio-cards.json` de la primera carpeta de mapeos que lo tenga).
- **Posible solución:** que el escaneo del catálogo reconozca e instale también `audio-cards.json`, o documentar que debe copiarse a mano.
- **¿Afecta el funcionamiento?** No.

---

## 4. Temas (`OpenFadeR-Themes` + core)

### THEME-01 — Sin validación de contraste
- **Severidad:** Media.
- **Descripción:** nada impide publicar un tema con `text` y `bg` casi iguales: la UI quedaría ilegible. El editor lo muestra en vivo, pero el CI del repo de temas (que sí valida tokens y layout) no mide contraste.
- **Evidencia:** tests de `src/app/skin/file.rs:408-460` (validez, no legibilidad); los 11 temas del repo pasan porque fueron hechos a mano, no por validación.
- **Posible solución:** test que calcule ratio de contraste (WCAG AA) de los pares clave (`text`/`bg`, `text`/`panel`, `text-bright`/`accent`, `row-dim`/`bg`) y avise (sin fallar, para no vetar temas artísticos).
- **¿Afecta el funcionamiento?** No; es test/aviso. Los 11 temas actuales tienen contraste correcto.

### THEME-02 — Longitudes sin cotas
- **Severidad:** Baja.
- **Descripción:** `Ty::Length` acepta números negativos y valores enormes; un tema a mano con `font-size: -5` o `h-deck-2: 10000px` puede dejar la UI inservible hasta borrarse el archivo.
- **Evidencia:** `src/app/skin/file.rs:188-190` (`number(j, "px")` sin signo ni rango) y `:220-226`; las duraciones, en cambio, ya rechazan negativos.
- **Posible solución:** cotas razonables (`0..=512px` para tamaños/fuentes, `0..=5000ms` para duraciones) con nota cuando se recorte.
- **¿Afecta el funcionamiento?** Solo rechazaría valores que hoy rompen la UI; ninguno de los 11 temas ni de los tokens por defecto se ve afectado.

### THEME-03 — Sin `schemaVersion`
- **Severidad:** Baja.
- **Descripción:** el formato de tema no lleva versión; la evolución futura depende de ignorar claves desconocidas con notas. Los mapeos sí tienen `schemaVersion="1"` (aunque el parser no lo comprueba, véase MAP-03).
- **Evidencia:** `src/app/skin/file.rs:124-166` (claves de primer nivel fijas, sin versión).
- **Posible solución:** añadir `"schemaVersion": 1` (opcional, ignorado por ahora) para poder introducir cambios futuros sin ambigüedad.
- **¿Afecta el funcionamiento?** No; es aditivo y con valor por defecto.

### THEME-04 — Guardado no atómico
- **Severidad:** Baja.
- **Descripción:** guardar un tema (SAVE AS, DUPLICATE, instalación) escribe directamente con `std::fs::write`; un corte de luz o disco lleno a mitad de escritura truncaría el JSON del usuario.
- **Evidencia:** `src/app/skin/store.rs:100-101` (`std::fs::write(&path, ...)`), en contraste con la instalación de mapeos, que usa `.part` + rename (`src/app/midi/get.rs`).
- **Posible solución:** escribir a `<slug>.json.part` y renombrar.
- **¿Afecta el funcionamiento?** No.

### THEME-05 — SAVE AS congela los 120 tokens (trade-off)
- **Severidad:** Diseño (decisión consciente).
- **Descripción:** los temas guardados desde el editor serializan todos los tokens para no depender de cambios futuros del base (reproducibilidad). El coste: si la paleta integrada mejora, los temas guardados no la heredan nunca.
- **Evidencia:** `src/app/skin/store.rs:96-101` + `src/app/skin/draft.rs:3-4` (el draft guarda todos los tokens); documentado en `docs/skins.md`.
- **Posible solución:** ofrecer en el editor "guardar solo los tokens cambiados" además de "guardar completo", con el base como fallback (el parser ya lo soporta: los temas `ocean` y `high-contrast` son parciales).
- **¿Afecta el funcionamiento?** Sería opt-in y por tanto sin impacto en los temas existentes; cambiar el comportamiento por defecto sí alteraría la reproducción exacta de temas guardados, por eso se propone como opción.

### THEME-06 — Hints de layout sin persistencia
- **Severidad:** Baja.
- **Descripción:** los 11 hints de `layout` (p. ej. `deck_layout: "vertical"`) se aplican una vez al seleccionar el tema, no al arrancar; si el usuario cambia la vista después, el "look" del tema no se restaura en el siguiente inicio. Está documentado, pero puede sorprender.
- **Evidencia:** `docs/skins.md:103-108` y `src/app/prefs/` (`apply_layout` solo en la selección).
- **Posible solución:** mantener el comportamiento (la libertad del usuario manda) y hacerlo explícito en la ficha del tema en la galería; la alternativa (re-aplicar en cada arranque) anularía los cambios manuales del usuario.
- **¿Afecta el funcionamiento?** La solución recomendada (documentar), no. Re-aplicar en arranque sí cambiaría el comportamiento actual.

---

## 5. Transversal

### X-01 — Sin checksums ni firmas
- **Severidad:** Media.
- **Descripción:** los cuatro repos se distribuyen por descarga de zip de GitHub sin digest verificado en el cliente; los plugins nativos ejecutan código con permisos plenos del usuario (véase CORE-03). Un compromiso de GitHub/zip interpuesto serving sería invisible.
- **Evidencia:** `src/catalog/` (descarga de zips sin verificación de hash) y `src/plugin/install.rs` (instala lo que llegue con un `plugin.json` válido).
- **Posible solución:** publicar `SHA256SUMS` por release y verificarlo en la instalación; a medio plazo, firmas de paquetes.
- **¿Afecta el funcionamiento?** No; solo rechaza descargas alteradas.

### X-02 — Distribución "GitHub como registro"
- **Severidad:** Informativa.
- **Descripción:** no hay índice central con revisión: cualquiera puede añadir su repo a `repositories.json` e instalar de ahí. Es coherente con la filosofía del proyecto, pero concentra la confianza en la lista de repos por defecto (`OpenRsG/*`).
- **Evidencia:** `~/.config/openfader/repositories.json` + catálogo (`src/catalog/`); la lista inicial trae los repos de OpenRsG.
- **Posible solución:** nada técnico urgente; documentar el modelo de confianza junto a CORE-03 y X-01.
- **¿Afecta el funcionamiento?** No.

### X-03 — `.part` residual en instalaciones de mapeos
- **Severidad:** Baja.
- **Descripción:** la instalación de un mapeo escribe a `.part` y renombra, pero si el rename falla el `.part` queda huérfano en `<config>/mappings` (no se carga — no acaba en `.xml` — pero ensucia).
- **Evidencia:** `src/app/midi/get.rs:78-94` (ruta `.part` + rename sin limpieza en error).
- **Posible solución:** borrar el `.part` en el camino de error (o al descubrir archivos).
- **¿Afecta el funcionamiento?** No.

### X-04 — Sin i18n: la interfaz es inglés incrustado (propuesta de traducción al español)
- **Severidad:** Informativa (mejora propuesta).
- **Origen:** core (`ui/*.slint`, textos de Rust) + plugins (manifiestos y toasts propios).
- **Descripción:** no existe ningún archivo de idiomas ni infraestructura i18n: los ~65 `.slint` llevan ~1.000 líneas con `title:/text:/tooltip:` y ~1.000 literales en inglés incrustados, además de cadenas en Rust (toasts, errores). Traducir "todo el sistema" no es un toggle: es adoptar un mecanismo de traducción.
- **Evidencia:** sin `.po/.pot/.ftl/locales` en el core; `slint = "1.17"` en `Cargo.toml` (soporta gettext: `@tr()` en `.slint`, `.po/.mo`, `slint::init_translations!` en el arranque).
- **Fase 1 (sin recompilar el core, ya realizable):** traducir los plugins propios (`plugins/deezer`, `plugins/yt-dlp`): labels/ajustes de `plugin.json`, toasts y mensajes de `main.py` — el host los muestra tal cual, en cada llamada.
- **Fase 2 (PR al core, propuesta):** adoptar el i18n de Slint, **multilenguaje con detección del sistema**: (a) marcar los literales de UI con `@tr(...)` empezando por los menús de `app.slint` y las páginas de preferencias (~200 cadenas de piloto); (b) crear un catálogo por idioma — `ui/locales/<lang>.po` (compilado a `.mo`), empezando por `es` y dejando el formato listo para añadir más idiomas después (el inglés queda como mensaje base dentro del propio código, sin `.po`); (c) al arrancar, elegir idioma: **por defecto el locale del sistema operativo** (`slint::init_translations!` con el locale detectado: `es-ES`→`es`, etc.), con override manual en la setting nueva **Language** (`System` | `English` | `Spanish` | ...); (d) extender el piloto al resto de la UI (~1.000 cadenas). El compilador de Slint valida los literales marcados en tiempo de build, así que los textos faltantes no rompen la app: quedan en inglés. Añadir un idioma nuevo después no toca código: solo un `.po` más.
- **Posible solución:** ver Fase 2. Herramienta de extracción: `slint-ls`/extractor propio o xgettext sobre los `.slint`.
- **¿Afecta el funcionamiento?** El marcado con `@tr` no cambia el comportamiento (solo cómo se muestran los textos); requiere recompilar el core (Rust + Slint). Los plugins siguen mostrando lo que su `plugin.json`/`main.py` digan, sin tocar el host — y los textos no traducidos quedan en inglés, nunca rompen.

---

## 6. Verificado sin hallazgos

Puntos que se auditaron y están correctos:

- **Mapeos:** 176 XML parsean; 0 acciones o luces desconocidas; 0 puertos o nombres duplicados; 0 `deck` fuera de rango; 0 `hires` con LSB fuera de rango; `folder_name`/install de mapeos valida el nombre como `.xml` plano. Estadística: 33.139 controles (1.960 a 14 bits), 23.704 salidas (212 con `blink`), 27 `stateRequest`, 28 `ringInterval`.
- **Temas:** 11 archivos JSON válidos, nombres únicos, 0 tokens desconocidos (120 en `ui/theme.slint`: Surfaces 29, Text 7, Accents 11, Waveform 6, Decks/meters 23, Typography 12, Sizes 29, Motion 3), `base` y `layout` correctos; `slug()` es seguro frente a path traversal; los built-ins no se pueden sobrescribir; `docs/skins.md` sincronizada por test (`src/app/skin/mod.rs:463-469`).
- **Plugins:** los 21 manifiestos del repo usan nombres válidos (no activan CORE-01); panics contenidos en la frontera C ABI (`panic = "unwind"` obligatorio en `plugins-repo/Cargo.toml:19-20`).

## 7. Prioridad de arreglo sugerida

1. **CORE-01** (crítico, fix pequeño y localizado).
2. **MAP-01** (17 líneas de XML + un test; restaura el scratch de 13 controladoras).
3. **CORE-07 + CORE-02** juntos (mismo archivo, mismo patrón: no destruir el destino antes de tiempo; purge diferida en Windows).
4. **MAP-02** (tolerancia a archivos rotos).
5. CORE-08, CORE-06, CORE-04, X-01 (robustez, mayormente testearlos y acotar).
6. Bajas y documentación (CORE-09, MAP-03/04/06/07, THEME-01..06, PLUG-01) en limpio.
