# Reproducción y DRM en CrunchyTUI

Documento de decisión y roadmap. Última actualización: 2026-08.

## Contexto

Desde 2024 Crunchyroll sirve sus streams **solo con DRM (Widevine)**.
`crunchy-cli` fue archivado por este motivo
([crunchy-labs/crunchy-cli#362](https://github.com/crunchy-labs/crunchy-cli/issues/362)).
`crunchyroll-rs` expone el stream, pero documenta que está cifrado y no lo
descifra. mpv y VLC **no tienen soporte para Widevine**, y
`libwidevinecdm.so` es un componente de navegador, no un plugin de mpv.

Consecuencia: no existe hoy una vía pública y mantenible para reproducir
Crunchyroll en mpv "pasándole las llaves". Cualquier implementación de ese
tipo implicaría extraer material de licencia y queda **fuera del alcance**
de este proyecto.

## Diseño actual (implementado)

```rust
enum PlaybackTarget {
    Mpv(PlayRequest),              // stream sin DRM → mpv directo
    Browser { url, title },        // stream con DRM → ventana app
}
```

Flujo al pulsar *Reproducir* en un episodio:

1. Se resuelve la versión de audio elegida (`EpisodeVersion`).
2. Se pide el `Stream` a la API.
3. Si `session.uses_stream_limits` o `drm.name` no está vacío →
   **ventana de navegador en modo aplicación** con la URL oficial
   `https://www.crunchyroll.com/watch/{id}/{slug}`.
4. Si no → `mpv <manifiesto> [--sub-file=…] [--hls-bitrate=…]`.

### Detección de navegador

Orden de preferencia (primer binario encontrado en `PATH`):

| Navegador | Modo ventana app |
|---|---|
| brave, brave-browser | `--app=URL` |
| google-chrome(-stable), chromium(-browser) | `--app=URL` |
| firefox | `--kiosk URL` (no tiene modo app) |

Si no hay ninguno, se muestra un popup de error.

### Modelo de credenciales

- La **TUI** guarda solo el *refresh token* de la API
  (`~/.config/crunchytui/config.toml`, permisos 0600). Nunca la contraseña.
- El **navegador** usa sus propias cookies. La sesión de la TUI **no se
  transfiere**: no copiamos tokens, cookies ni credenciales entre ambos.
- Si el navegador elegido ya tiene sesión de Crunchyroll, la reproducción
  es inmediata; si no, se inicia sesión una vez en esa ventana.

### Limitaciones conocidas del modo navegador

- Audio: se respeta la versión elegida (la URL apunta al episodio de esa
  versión de audio).
- Subtítulos y calidad: se eligen dentro del reproductor web; el popup de
  la TUI no puede imponerlos sobre DRM.
- Firefox se abre en kiosk (pantalla completa sin chrome).

## Roadmap: reproductor propio ("nativo")

Objetivo: una ventana propia de CrunchyTUI que reproduzca DRM sin abrir el
navegador del usuario, manteniendo las credenciales locales.

### Lo que NO se hará

- Decoder Widevine propio en Rust (no existe SDK público de Crunchyroll;
  requeriría material de licencia no disponible).
- Extracción de claves, descifrado, ni cualquier bypass de DRM.
- `wry`/WebKitGTK: no garantiza Widevine en Linux.

### La opción viable: CEF (Chromium Embedded Framework)

Una ventana de aplicación propia que embebe Chromium — el usuario ve una
ventana nativa de CrunchyTUI, no un navegador — y delega Widevine al CDM
oficial. Es lo único compatible con DRM sin romper el modelo de licencias.

### Fases

1. **PoC de viabilidad** (bloqueante, decide todo lo demás):
   binario mínimo `crunchytui-player` con CEF que abra una ventana sin
   chrome, cargue crunchyroll.com y reproduzca un episodio con Widevine
   en Linux/NixOS. Si CEF no obtiene Widevine en este entorno, el roadmap
   termina aquí y se queda el modo navegador.
2. **Perfil persistente propio** en `~/.local/share/crunchytui/player-profile`
   (login una sola vez, cookies locales, separadas del navegador del usuario).
3. **Integración**: `crunchytui-player --episode-url URL` lanzado desde la
   TUI; comunicación por Unix socket para abrir/pausar/cerrar/siguiente.
4. **Controles** (opcional, frágil): sincronizar audio/subtítulos/calidad
   con la página; posponer hasta que 1–3 sean estables.

### Criterio de salida

Si en la PoC (fase 1) Widevine no funciona con la build de CEF disponible
en NixOS, se documenta y no se invierte más: el modo navegador `--app`
pasa a ser la solución definitiva.
