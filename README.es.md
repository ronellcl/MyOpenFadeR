<p align="center">
  <img src="assets/openfader.png" alt="Logotipo de OpenFadeR" width="160">
</p>

<h1 align="center">OpenFadeR</h1>

<p align="center">
  <img src="docs/media/hero.png" alt="OpenFadeR con dos mesas mezclando, sincronizadas, con ondas, mezclador y navegador de pistas" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img alt="Licencia: AGPL-3.0" src="https://img.shields.io/github/license/OpenRsG/OpenFadeR"></a>
  <a href="https://github.com/OpenRsG/OpenFadeR/stargazers"><img alt="Estrellas" src="https://img.shields.io/github/stars/OpenRsG/OpenFadeR?style=flat"></a>
</p>

<p align="center">
  <a href="README.md"><b>English</b></a> · <a href="README.es.md"><b>Español</b></a>
</p>

OpenFadeR es una aplicación de DJ libre para dos o cuatro mesas, escrita en Rust con [Slint](https://slint.dev). Analiza tus pistas, separa stems, mezcla con efectos y un sampler, y lee controladores MIDI de un conjunto creciente de archivos de mapeo. Los temas, los plugins y los mapeos de controlador se instalan desde repositorios de GitHub con un clic.

<p align="center">
  <img src="docs/media/demo.webp" alt="Cargando dos pistas, sincronizándolas, moviendo el crossfader y cambiando de tema" width="100%">
</p>

<p align="center"><a href="docs/media/demo.mp4">El vídeo en MP4</a></p>

Todas las capturas de esta página salen de la aplicación en marcha, con una biblioteca de pistas sintéticas generadas para estas imágenes. No hay música con derechos en ellas.

## Qué añade esta versión

La 0.4.0 reúne lo que esta rama ha añadido desde la 0.3.1:

- **SYNC engancha de una vez.** Los candados de tempo y fase se toman juntos y la corrección se queda dentro de 1 ms, así que sincronizar dos mesas ya no da el tirón que se notaba.
- **Cada pista vive una sola vez, en 16 bits.** Las mesas, la vista previa del navegador y el análisis en bloque decodifican directamente a muestras de 16 bits (10,6 MB por minuto de estéreo a 44,1 kHz) en vez de guardar una copia f32 de la pista entera, y la huella se calcula mientras se decodifica.
- **El análisis lee a través de `audio::Frames`.** La rejilla, la onda, la tonalidad y la voz aceptan la pista de 16 bits de una mesa sin copiarla; una pista sin caché se analiza en el acto y se guarda como antes.
- **La memoria vuelve al sistema.** En glibc el asignador devuelve los búferes de una pista descargada en vez de dejarlos en el heap (274 MB a 203 MB con dos pistas de 6 minutos, en la medición del original).
- **Build para Raspberry Pi.** `Dockerfile.pi` compila un archivo de 64 bits para Pi 3, 4 y CM4 (Debian 13), con STEMS en la CPU. Ver [docs/raspberry-pi.md](docs/raspberry-pi.md).
- **MIDI clock de salida.** PREFERENCIAS > CONTROLADOR > OTHERS > MIDI CLOCK OUT envía un reloj de 24 PPQN al tempo y al beat de la mesa líder, con START en su siguiente compás y STOP cuando ya no suena nada.
- **Memory cues.** Los puntos de memoria se dibujan como barras anchas con una M, junto a los hot cues.
- **Editar los detalles de una pista.** Un diálogo edita título, artista, álbum, género, comentario, valoración y color sin salir del navegador.
- **Los stems se reutilizan.** Una pista que ya se separó conserva sus stems al volver a cargarla, y un nuevo camino de enganche arregla la primera carga.
- **El navegador filtra por origen.** El filtro por origen, las carpetas fijadas y el aviso de duplicados van atados al árbol plegable; las etiquetas del árbol hablan un solo idioma (inglés, o español en las preferencias) y la columna PREVIEW muestra un fotograma fijo en vez de la onda.

<details>
<summary>Versiones anteriores (0.3.1 y anteriores)</summary>

Esta versión lleva los cambios de esta rama sobre el 0.2.1 original: la interfaz
en español, un instalador de Windows que no necesita nada más en la máquina,
ninguna ventana de consola, un motor de vídeo que sigue el ritmo, YouTube con
imagen, el motor nativo de Deezer, pistas de la nube en la Colección, el
escenario VDJ y una ventana que abre a pantalla completa.

- **La interfaz en español.** Todos los menús, las páginas de preferencias y las etiquetas de los plugins están traducidos, y PREFERENCIAS > VISTA > IDIOMA elige Sistema, English o Español. Las preferencias que el original añadió últimos (la rejilla de beats en negrita, las marcas de hot cue grandes) también están traducidas.
- **Un instalador de Windows que lo lleva todo.** `OpenFadeR-<versión>-windows-x86_64-setup.exe` instala el programa para todos los usuarios (pide administrador) con los plugins, los temas, los mapeos de controlador, un ffmpeg ligero, un Python para los plugins de Python y el programa yt-dlp: una máquina recién estrenada no necesita nada más. Ver [packaging/windows](packaging/windows).
- **Ninguna ventana de consola.** Los plugins usan el intérprete que viaja con el programa, no el lanzador `py`, y todos los ayudantes (ffmpeg, ffprobe, los selectores de fichero) arrancan sin consola: ya no aparece una ventana de Windows Terminal sobre la aplicación.
- **Vídeo que sigue el ritmo.** Un salto se pone en negro y vuelve con el frame correcto en vez de quedarse congelado en el último, y el ffmpeg que arranca para él es el build ligero (80 ms frente a dos segundos): un salto tarda medio segundo en vez de tres.
- **YouTube con imagen.** El plugin de yt-dlp descarga H.264 + AAC (los formatos que muestra cualquier reproductor), la imagen llega a la pantalla de cada mesa y a la pantalla maestra, y el plugin se mantiene al día desde la copia que guarda en tu carpeta de usuario.
- **Deezer, nativo.** El plugin de deezer es un programa en Rust: busca en el catálogo y descarga las pistas (FLAC o MP3 320) con tu propio ARL, sin Node, sin deemix y sin Python por medio. Viaja con el instalador.
- **Pistas de la nube en la Colección.** Una pista de una fuente de plugin (YouTube, Deezer) puede vivir en una lista o en una paleta como cualquier otra: el audio se descarga cuando la carga una mesa, y el navegador marca con un ✓ las que ya están descargadas.
- **El escenario VDJ.** Un tema puede pedir el diseño de Virtual DJ: el panel de FX y la pantalla maestra comparten una franja bajo las mesas, cada mesa tiene su propia pantalla y los faders de canal mantienen su longitud.
- **La ventana abre a pantalla completa.** PREFERENCIAS > VISTA > "Abrir en pantalla completa" (activo por defecto). El ítem "Pantalla completa" del menú Ver la enciende y la apaga, y la elección se recuerda.
- **Un browser que enseña lo que hay.** El árbol lista todos los volúmenes montados (un USB sin biblioteca de DJ también, y un disco duro USB: Windows lo llama disco fijo) con sus carpetas de primer nivel, cada una abierta como fuente, y una sección "Downloaded" con lo que descargaron los plugins de nube: ficheros de verdad, que se cargan, se importan y se buscan.
- **También el disco local.** El volumen donde está Windows (C:) se lista con los demás, en solo lectura, para recorrer sus carpetas como las de un USB.
- **Las carpetas que fijas.** El botón FAV fija la carpeta que tengas elegida; las carpetas fijadas te esperan en la sección "Favorites", y el pin sobrevive a que el USB cambie de letra.
- **Una sola búsqueda para todo lo local.** La caja del buscador mira la Colección, el índice de las carpetas fijadas y las fuentes ya leídas, y cada resultado dice de dónde salió. Las búsquedas en línea de YouTube y Deezer se quedan en su propia caja: el filtro no pregunta a ningún plugin.
- **Audio o vídeo, dicho claro.** Un chip en la barra de filtros deja pasar todo, solo el audio o solo el vídeo, y la tabla de pistas tiene la columna FORMATO con una nota para la música y una película para el vídeo.
- **Bitrate.** La tabla de pistas gana la columna BITRATE a la derecha de FORMATO, mostrando "320 kbps", "256 kbps", … (vacío cuando es desconocido).
- **Secciones plegables.** Un clic en Historial, Etiquetas, Discos, Downloaded, Música Local o Música en Línea pliega/despliega sus hijos (el ícono cambia entre ▸ y ▾); el teclado solo mueve la selección.
- **Historial con clic derecho.** Clic derecho en una fila de sesión para renombrarla (manteniendo la fecha como clave interna) o eliminarla.
- **Related tracks con ancla automática.** Si no hay pista elegida al activar Related, toma la primera visible en lugar de mostrar vacío.

</details>

## Descarga

Coge el archivo de tu sistema en la [página de versiones](https://github.com/ronellcl/MyOpenFadeR/releases): Linux (x86_64), Windows (x86_64) o macOS (Apple silicon). Cada uno trae la aplicación con sus temas, mapeos de controlador y plugins. Descomprímelo y ejecuta `openfader` (en macOS: botón derecho sobre `OpenFadeR.app` > Abrir la primera vez, la app no está firmada). Esto es una versión preliminar: cuenta lo que no funcione.

En Windows hay además un instalador, `OpenFadeR-<versión>-windows-x86_64-setup.exe`: instala el programa para todos los usuarios (pide administrador) con los plugins, los temas, los mapeos de controlador, ffmpeg y un Python para los plugins de Python, añade accesos en el menú Inicio y en el escritorio y aparece en Agregar o quitar programas. Desinstalarlo deja tus ajustes, tu biblioteca y los complementos descargados en tu carpeta de usuario salvo que digas lo contrario. Ni el instalador ni la app están firmados, así que Windows SmartScreen pregunta la primera vez. Ver [packaging/windows](packaging/windows) para construirlo.

## Compilar desde el código

Necesitas `git` y un [toolchain de Rust](https://rustup.rs).

```sh
git clone https://github.com/OpenRsG/OpenFadeR.git
cd OpenFadeR
./start.sh ~/Music
```

La primera vez clona los plugins, los temas y los mapeos junto al código, compila todo y conecta la carpeta que le diste a la Colección. Deberías ver tus pistas en el navegador, con el BPM, la tonalidad y las vistas previas de onda rellenándose según acaba el análisis. Haz doble clic en una pista para cargarla en la mesa 1, pulsa `Espacio` para reproducir.

`start.sh` sin ruta abre la Colección como la dejaste; cada biblioteca y carpeta conectada se sincroniza al arrancar y cada vez que cambia, y `$XDG_MUSIC_DIR` (si no, `~/Music/OpenFadeR`) está siempre conectada. Para compilar solo el núcleo: `cargo build --release`.

En Windows el vídeo de una mesa lo decodifica un proceso hijo de `ffmpeg`, y un salto lo vuelve a arrancar: el build completo de [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) (400 MB) tarda dos segundos en arrancar y cada salto lo espera, mientras que el build essentials (100 MB) tarda 80 ms. Ponlo en `target\release\tools\` y el programa lo encuentra primero (esa carpeta va primera en su PATH):

```powershell
Expand-Archive ffmpeg-release-essentials.zip -DestinationPath C:\Tools\ffmpeg
New-Item -ItemType Directory -Force target\release\tools
Copy-Item C:\Tools\ffmpeg\ffmpeg-*-essentials\bin\ffmpeg.exe, C:\Tools\ffmpeg\ffmpeg-*-essentials\bin\ffprobe.exe target\release\tools
```

Para instalarlo en tu usuario en Linux (el comando `openfader` y su entrada en el menú de aplicaciones, con los plugins, los temas y los mapeos dentro): `./install.sh`. También deja los repositorios de plugins, temas y mapeos por defecto de PREFERENCIAS (GET PLUGINS / GET THEMES / GET MAPPINGS), que la aplicación usa de todas formas hasta que edites la lista. Vuelve a ejecutarlo para actualizar; `./install.sh --uninstall` lo quita.

## Un recorrido

### Análisis listo cuando cargas

BPM, rejilla de beats, downbeat, tonalidad, onda de tres bandas, frases (intro, subida, estribillo, bajada) y cues automáticas se calculan en segundo plano y se cachean. Las pictures de arriba los muestran: barras de frase de color sobre las ondas ampliadas, hot cues de la A a la H bajo cada mesa, y un color de tonalidad en la lista de pistas. Las mesas sincronizan tempo y beats, el key sync sigue la tonalidad y el bloqueo de tonalidad mantiene el pitch mientras subes el tempo.

Para ondas más claras, abre PREFERENCIAS > VISTA > FORMA DE ONDA. Pon REJILLA DE BEATS en NEGRITA para líneas de contorno sobre el audio, con los beats fuertes más anchos, que no dependen solo del color. Pon MARCAS DE HOT CUES en GRANDE para banderas y letras al doble de tamaño en las mesas y en las vistas previas del navegador (cuando los marcadores de cue de la vista previa están activados). Los dos ajustes se aplican al momento y se guardan; NORMAL deja los tamaños de siempre.

### Dos mesas o cuatro

<p align="center"><img src="docs/media/four-decks.png" alt="Cuatro mesas sonando en sincronía con cuatro bandas de onda apiladas" width="100%"></p>

Pulsa `D` para cambiar de diseño. Las cuatro mesas tienen sus propios pads, loops, slip, sync y botones de stems. El diseño de las mesas (horizontal o vertical) es un ajuste de VISTA, y un tema puede pedirlo.

### Efectos

<p align="center"><img src="docs/media/fx.png" alt="Unidades de Beat FX con eco, reverb y flanger sobre las mesas, y la fila de Color FX" width="100%"></p>

Beat FX tiene varias unidades con tres huecos cada una, con el tempo tomado de la mesa o marcado a mano. COLOR FX (filtro, space, dub echo, sweep, noise, crush, cross-pass) está en cada canal y MERGE FX trabaja sobre toda la mezcla. Más efectos llegan con los plugins de efectos.

### Sampler

<p align="center"><img src="docs/media/sampler.png" alt="Franja del sampler con dieciséis huecos, bancos y un secuenciador" width="100%"></p>

Dieciséis huecos en dos bancos, cargados desde una mesa o desde un fichero. Los huecos suenan como un golpe o como una puerta, pueden seguir el tempo, y un secuenciador graba un patrón y lo repite.

### Stems

<p align="center"><img src="docs/media/stems.png" alt="Botones de stem en dos mesas, con la voz y la batería silenciadas en la mesa A" width="100%"></p>

Activa STEMS en PREFERENCIAS > EXTENSIONES y cada pista cargada se separa en voz, instrumento, bajo y batería (hay también un modo de 3 stems) con un modelo ONNX que corre en CPU o en la GPU. Al activar STEMS se descarga ese modelo una vez (316 MB), con una tarjeta de progreso abajo a la derecha. El botón de stem de una mesa se rellena mientras su stem suena a fondo; un clic lo silencia, otro lo devuelve. Los pads y un controlador pueden silenciar o aislar, y los knobs del mezclador pueden actuar sobre los stems. La onda sigue lo que oyes.

### BEAT LAB

<p align="center"><img src="docs/media/beatlab.png" alt="Franja de BEAT LAB con bucles de percusión capturados de dos mesas" width="100%"></p>

BEAT LAB captura unos cuantos beats del stem de percusión de una mesa en un hueco. Rellena los huecos de una mesa, cambia entre ellos, y pon la batería de una pista bajo la melodía de otra. Necesita STEMS y se activa en la misma página de preferencias.

### Un navegador que sugiere

<p align="center"><img src="docs/media/suggest.png" alt="El navegador de pistas mostrando sugerencias para la pista cargada, ordenadas por coincidencias" width="100%"></p>

El navegador lee carpetas (las subcarpetas se convierten en listas) o plugins de biblioteca, y tiene búsqueda, columnas ordenables, un filtro de pistas (rango de BPM, coincidencia de tonalidad, valoración), pistas relacionadas, una Tag List, listas inteligentes, historial, un buscador de duplicados y un gestor de ficheros que faltan. La pestaña SUGGEST ordena el resto de tu colección contra la pista cargada y enseña una puntuación para cada una, con filtros de BPM y tonalidad. Tus propias ediciones (cues, loops, rejilla, valoración, comentario, color) viven en `store.db`; una fuente de biblioteca nunca se escribe.

### Modo EDIT

<p align="center"><img src="docs/media/edit.png" alt="Modo EDIT con una selección de 16 compases ajustada al beat sobre la onda de la pista entera" width="100%"></p>

EDIT muestra la pista entera con su rejilla de beats. Selecciona con ajuste a beat o a compás, y copia, corta, pega, inserta, borra, silencia, duplica o añade silencio, deshace y rehace, y renderiza el resultado a un fichero WAV o lo carga en una mesa.

### Modo LIGHTING (beta)

<p align="center"><img src="docs/media/lighting.png" alt="Modo LIGHTING con cabezas de iluminación encendidas por la frase de la pista que suena" width="100%"></p>

LIGHTING parchea cabezas de iluminación (pares RGB, RGBW, cabezas móviles, estroboscopios) y las maneja desde la frase que suena, así que un estribillo se ve distinto de un puente. Elige una paleta y una escena, o deja que AUTO elija. El DMX sale por Art-Net o sACN, o a ninguna parte: la vista de escenario funciona sin ningún equipo. Está desactivado por defecto: actívalo en PREFERENCIAS > EXTENSIONES para añadir LIGHTING al menú de modos.

### Exportar y vídeo

El modo EXPORT enseña los cues y los memory points de las dos mesas junto a un panel de DISPOSITIVOS. Los palos USB y las tarjetas SD los escriben plugins de dispositivo, con varios formatos uno al lado del otro en el mismo palo compartiendo una sola copia del audio, así que el mismo palo suena en reproductores y aplicaciones distintas. Una pista que está a la vez en la colección y en un palo conserva los mismos cues, rejilla y frases en las dos copias: al enchufar el palo se rellena la copia que no los tenía, y cargar una pista hace que gane la copia que cargaste.

<p align="center"><img src="docs/media/export.png" alt="Modo EXPORT con la edición de cues de dos mesas y un panel de dispositivos" width="100%"></p>

El modo VIDEO (beta) reproduce vídeos en una ventana de salida y hace el crossfade entre las imágenes de las dos mesas. Está desactivado por defecto: como LIGHTING, aparece en el menú de modos cuando lo activas en PREFERENCIAS > EXTENSIONES.

### Tu biblioteca es tuya

La Colección es la base de datos propia de OpenFadeR. Las bibliotecas de otras aplicaciones se conectan a ella con plugins de biblioteca (PREFERENCIAS > BIBLIOTECAS) y se sincronizan; un formato que guarda menos que la Colección (menos hot cues, sin loops, una paleta de color más pequeña) se escribe desde una copia ajustada, y la línea de estado dice qué no cupo. Archivo > Exportar Colección escribe además un fichero JSON abierto, una tabla CSV y una lista M3U8, descritos con las tablas de la base de datos en [docs/library-format.md](docs/library-format.md).

## Temas

<p align="center"><img src="docs/media/themes-grid.jpg" alt="La misma vista en Catppuccin Mocha, Latte, Rek, So Pro, Dracula, Nord, Tokyo Night y Gruvbox Dark" width="100%"></p>

La aplicación trae Catppuccin Mocha (el de serie) y Latte. El repositorio [OpenFadeR-Themes](https://github.com/OpenRsG/OpenFadeR-Themes) añade once más, así que hay trece en total. Un tema puede cambiar colores, tipografías, tamaños e incluso el diseño, y se aplica al momento. PREFERENCIAS > TEMAS tiene un editor para cada token con vista previa en vivo, GUARDAR COMO, e importación.

<p align="center"><img src="docs/media/themes-gallery.png" alt="PREFERENCIAS, página TEMAS con la galería y GET THEMES" width="100%"></p>

## Plugins, temas y mapeos

Tres repositorios acompañantes alimentan la aplicación, y PREFERENCIAS los lista desde la primera vez:

| Repositorio | Qué guarda | Dónde en la aplicación |
| --- | --- | --- |
| [OpenFadeR-Plugins](https://github.com/OpenRsG/OpenFadeR-Plugins) | plugins de biblioteca, dispositivo, efecto, acción y fuente (17 al escribir esto) | PREFERENCIAS > EXTENSIONES > GET PLUGINS |
| [OpenFadeR-Themes](https://github.com/OpenRsG/OpenFadeR-Themes) | temas | PREFERENCIAS > TEMAS > GET THEMES |
| [OpenFadeR-Map](https://github.com/OpenRsG/OpenFadeR-Map) | mapeos de controladores MIDI (más de 170 archivos) | PREFERENCIAS > CONTROLADOR > GET MAPPINGS |

INSTALL instala uno. Añade cualquier repositorio con su ruta de GitHub (`usuario/nombre`) y la aplicación también encuentra ahí sus plugins, temas o mapeos, así que compartir un tema es publicar un repositorio de GitHub.

Los plugins arrancan desactivados. En el primer arranque una ventana de bienvenida ofrece los sugeridos (los plugins que leen y escriben las bibliotecas, dispositivos y efectos de otros programas de DJ) y STEMS; ACEPTAR los enciende, OMITIR los deja apagados. Ayuda > Bienvenida la vuelve a abrir.

En cada arranque la aplicación le pide a GitHub la versión más nueva; cuando es más nueva que la que está corriendo, una ventana ofrece abrir su página de descarga. Sin conexión no se muestra nada.

Enchufa un controlador que aún no tenga mapeo y la aplicación ofrece el archivo que le corresponde del repositorio de mapeos: un clic en INSTALL y el controlador funciona. Los archivos de mapeo cubren la capa por defecto de cada controlador; el aprendizaje MIDI y los LED de respuesta vienen de serie, y la detección de conexión y desconexión recoge controladores mientras la aplicación está en marcha. El formato está descrito en [docs/mappings.md](docs/mappings.md).

Para aparatos externos, PREFERENCIAS > CONTROLADOR > OTHERS > MIDI CLOCK OUT toma parte del nombre de un puerto de salida y envía un reloj MIDI de 24 PPQN al tempo y al beat de la mesa líder: START en su siguiente compás y STOP cuando ya no suena ninguna, para que una caja de ritmos o un secuenciador siga la mezcla (y vuelva a engancharse tras desconectar y conectar el aparato).

## Atajos de teclado

Una tabla mueve las acciones y la ayuda de `?` (Shift = mesa B, Ctrl = mesa C, Ctrl+Shift = mesa D):

| Tecla | Acción | Tecla | Acción |
| --- | --- | --- | --- |
| Espacio | reproducir/pausar | K | key sync |
| Z / X | CUE A / CUE B | `-` / `=` / 0 | bajar / subir / reset de tonalidad |
| 1–8 | pads (por modo de pad) | M | siguiente modo de pad |
| L | loop de 4 beats | V / E / B | slip / reverse / brake |
| I / O / R | loop dentro / fuera / reloop | T | añadir a la Tag List |
| `[` / `]` | salto de beat −4 / +4 | ← / → | fracción de beat del FX |
| Intro | LOAD | F | FX on/off |
| S | SYNC | Ctrl+F | foco en la búsqueda |
| Q | MASTER | W / Shift+W | VISTA / Área |
| Arriba / Abajo | selección del navegador | ? | ayuda |
| G / Y / N | fila de rejilla / tap / beat 1 aquí | `,` / `.` | desplazamiento de rejilla ∓1 ms |
| P | franja del sampler | A / Shift+A | AUTOMIX on/off / saltar |
| D | diseño de 2 / 4 mesas | Ctrl+Z / Ctrl+Shift+Z (Ctrl+Y) | deshacer / rehacer una edición de biblioteca |

La tabla se edita en PREFERENCIAS > TECLADO.

## Dónde se encuentra cada cosa

Cada tipo se busca en una lista de carpetas por variable de entorno (separada por `:`), una carpeta del directorio de configuración y la carpeta que viene junto al ejecutable (o en una carpeta por encima).

| Tipo | Variable de entorno | Carpeta de configuración | Carpeta incluida |
| --- | --- | --- | --- |
| Plugins | `OPENFADER_PLUGINS` | `~/.config/openfader/extension/plugin/` | `extension/plugin/` |
| Temas | `OPENFADER_SKINS` | `~/.config/openfader/extension/skin/` | `extension/skin/` |
| Mapeos de controlador | `OPENFADER_MAPPINGS` | `~/.config/openfader/mappings/` | `mappings/` |

`start.sh` clona los tres repositorios en esas carpetas (git las ignora aquí) y los plugins nativos se compilan con `extension/plugin/build.sh` contra el `sdk/` de esta copia. Los ajustes viven en `$XDG_CONFIG_HOME/openfader/` (por defecto `~/.config/openfader/`), la Colección (`store.db`) y nuestro análisis en `$XDG_DATA_HOME/openfader/` (por defecto `~/.local/share/openfader/`).

### Linux, Windows y macOS

La documentación escribe las carpetas de Linux; las demás siguen la convención de su sistema (`$XDG_*_HOME` gana en todos los sistemas cuando está definido):

| | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Ajustes, tus plugins, temas y mapeos (`~/.config/openfader/` en esta documentación) | `~/.config/openfader/` | `~/Library/Application Support/openfader/` | `%APPDATA%\openfader\` |
| Almacén, análisis, plugins descargados | `~/.local/share/openfader/` | `~/Library/Application Support/openfader/` | `%LOCALAPPDATA%\openfader\` |
| Cachés (stems, modelos) | `~/.cache/openfader/` | `~/Library/Caches/openfader/` | `%LOCALAPPDATA%\openfader\cache\` |

Los archivos de las versiones traen las carpetas incluidas junto al ejecutable (`OpenFadeR.app/Contents/Resources/` en macOS). Los dispositivos extraíbles se encuentran como palos USB montados por debajo de `/run/media` o `/media` (Linux), letras de unidad extraíbles (Windows) y volúmenes por debajo de `/Volumes` (macOS); la expulsión usa udisks, la petición de expulsión del propio volumen o `diskutil`. Solo Linux tiene los deslizadores de volumen por tarjeta en la página de audio (el mezclador ALSA, el servidor de sonido) y monta por su cuenta un palo enchufado (udisks): en Windows y macOS el volumen es el del sistema y el SO monta el palo. Los selectores de ficheros son zenity o kdialog en Linux, los diálogos de PowerShell en Windows y `osascript` en macOS.

El comportamiento del audio, del MIDI y de las carpetas se ajusta con `OPENFADER_OUTPUT_DEVICE`, `OPENFADER_HP_CHANNELS`, `OPENFADER_CUE_DEVICE`, `OPENFADER_MIDI_PORT`, `OPENFADER_MIDI_LOG`, `OPENFADER_JOG_TICKS`, `OPENFADER_TEMPO_INVERT`, `OPENFADER_REC_DIR`, `OPENFADER_EXPORT_DIR`, `OPENFADER_PATH_MAP` (remapeo de rutas de audio, `desde=>hasta;…`) y `OPENFADER_DEVICE_ROOTS` (carpetas extra tratadas como dispositivos).

## Documentación

- [docs/plugins.md](docs/plugins.md): formato de plugin y la ABI de biblioteca / dispositivo, con el SDK en [`sdk/`](sdk/)
- [docs/skins.md](docs/skins.md): ficheros de tema, cada token, pistas de diseño
- [docs/mappings.md](docs/mappings.md): archivos de mapeo de controlador
- [docs/library-format.md](docs/library-format.md): la exportación abierta en JSON / CSV y las tablas de la base de datos
- [hallazgos_solucionados.md](hallazgos_solucionados.md): lo que se encontró y lo que se reparó, con el commit de cada arreglo

El código sigue [docs/code-structure.md](docs/code-structure.md): una carpeta por sección (`library/`, `analysis/`, `audio/`, `plugin/`, `app/`, …), un `mod.rs` base y una hija por variante. Solo `app/` y la crate `ui/` usan Slint.

## Cómo contribuir

Los informes de errores, los archivos de mapeo, los temas y los plugins son bienvenidos. Lee antes [CONTRIBUTING.md](CONTRIBUTING.md); el [código de conducta](CODE_OF_CONDUCT.md) se aplica, y los avisos de seguridad pasan por [SECURITY.md](SECURITY.md). `cargo test --release` ejecuta las pruebas, y las de los repositorios acompañantes también cuando están descargados.

## Licencia

AGPL-3.0, ver [LICENSE](LICENSE). Hecho por [OpenRsG](https://github.com/OpenRsG).
