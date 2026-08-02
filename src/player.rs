//! Lanzamiento de mpv como proceso externo.

use std::process::{Command, Stdio};

use crate::errors::AppError;

/// Todo lo necesario para reproducir un episodio.
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
}
