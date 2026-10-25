//! Reproducción externa: mpv para streams sin DRM y una ventana de
//! navegador en modo aplicación para contenido con DRM (Widevine).

use std::process::{Command, Stdio};

use crate::errors::AppError;

/// Todo lo necesario para reproducir un episodio en mpv.
pub struct PlayRequest {
    /// URL del manifiesto del stream (HLS/DASH).
    pub url: String,
    /// Título mostrado en la ventana de mpv.
    pub title: String,
    /// URL del archivo de subtítulos (ass/vtt), si se eligió alguno.
    pub sub_url: Option<String>,
    /// "auto", "1080", "720" o "480".
    pub quality: String,
}

/// Destino de reproducción resuelto para un episodio.
pub enum PlaybackTarget {
    /// Stream directo, reproducible con mpv.
    Mpv(PlayRequest),
    /// Contenido con DRM: se abre la página oficial del episodio en una
    /// ventana de navegador en modo aplicación (sin pestañas ni barra de
    /// direcciones). La sesión la gestiona el navegador, nunca la TUI.
    Browser {
        /// URL oficial del episodio (https://www.crunchyroll.com/watch/…).
        url: String,
        /// Título del episodio, solo para mostrarlo en el popup.
        title: String,
    },
}

/// Argumentos extra de mpv según la calidad elegida.
// ponytail: --hls-bitrate solo aplica a HLS; en DASH mpv lo ignora y adapta
// solo. Los bitrates son aproximados por resolución.
fn quality_args(quality: &str) -> Vec<String> {
    match quality {
        "1080" => vec!["--hls-bitrate=max".into()],
        "720" => vec!["--hls-bitrate=4500000".into()],
        "480" => vec!["--hls-bitrate=2500000".into()],
        _ => vec![],
    }
}

/// Lanza mpv desacoplado del TUI (mpv abre su propia ventana).
pub fn launch(req: &PlayRequest) -> Result<(), AppError> {
    let mut cmd = Command::new("mpv");
    cmd.arg(&req.url)
        .arg(format!("--title={}", req.title))
        .arg("--force-window=immediate");
    if let Some(sub) = &req.sub_url {
        cmd.arg(format!("--sub-file={sub}"));
    }
    cmd.args(quality_args(&req.quality));
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| AppError::Mpv(e.to_string()))?;
    Ok(())
}

/// Familia de navegador: determina cómo se pide una ventana "app".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrowserKind {
    /// Reproductor dedicado crunchytui-player.
    Direct,
    /// Chromium y derivados: `--app=URL` da una ventana sin chrome.
    Chromium,
    /// Firefox no tiene modo app; kiosk es lo más cercano.
    Firefox,
}

impl BrowserKind {
    fn args(self, url: &str) -> Vec<String> {
        match self {
            BrowserKind::Direct => vec![url.into()],
            BrowserKind::Chromium => vec![format!("--app={url}")],
            BrowserKind::Firefox => vec!["--kiosk".into(), url.into()],
        }
    }
}

/// Navegadores soportados, en orden de preferencia.
const BROWSERS: &[(&str, BrowserKind)] = &[
    ("crunchytui-player", BrowserKind::Direct),
    ("brave", BrowserKind::Chromium),
    ("brave-browser", BrowserKind::Chromium),
    ("google-chrome-stable", BrowserKind::Chromium),
    ("google-chrome", BrowserKind::Chromium),
    ("chromium", BrowserKind::Chromium),
    ("chromium-browser", BrowserKind::Chromium),
    ("firefox", BrowserKind::Firefox),
];

/// `which` sin dependencias: comprueba si `bin` existe en algún dir de PATH.
fn in_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(bin).is_file()))
}

/// Primer navegador compatible encontrado en PATH.
fn detect_browser() -> Option<(&'static str, BrowserKind)> {
    BROWSERS.iter().copied().find(|(bin, _)| in_path(bin))
}

/// Abre `url` en una ventana de navegador en modo aplicación.
pub fn launch_browser(url: &str) -> Result<(), AppError> {
    let Some((bin, kind)) = detect_browser() else {
        return Err(AppError::Browser(
            "no se encontró un navegador compatible (brave, chrome, chromium o firefox)".into(),
        ));
    };
    tracing::info!("abriendo reproductor con {bin}");
    Command::new(bin)
        .args(kind.args(url))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| AppError::Browser(format!("{bin}: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_mapping() {
        assert_eq!(quality_args("1080"), vec!["--hls-bitrate=max"]);
        assert_eq!(quality_args("720"), vec!["--hls-bitrate=4500000"]);
        assert_eq!(quality_args("480"), vec!["--hls-bitrate=2500000"]);
        assert!(quality_args("auto").is_empty());
        assert!(quality_args("cualquier-cosa").is_empty());
    }

    #[test]
    fn browser_args() {
        let url = "https://www.crunchyroll.com/watch/GRDQPM1ZY/x";
        assert_eq!(
            BrowserKind::Chromium.args(url),
            vec![format!("--app={url}")]
        );
        assert_eq!(
            BrowserKind::Firefox.args(url),
            vec!["--kiosk".to_string(), url.to_string()]
        );
    }

    #[test]
    fn path_lookup() {
        assert!(in_path("sh"));
        assert!(!in_path("crunchytui-binary-que-no-existe"));
    }
}
