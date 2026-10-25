//! Login "como en el navegador": se abre la página oficial de Crunchyroll y,
//! al terminar, el navegador lanza `sso.crunchyroll://auth?code=…`. En Linux
//! este binario se registra como manejador de ese esquema (`--sso-callback`)
//! y deja la URL en un archivo que la TUI vigila. Si falla, se puede pegar.

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::config::Config;

const DESKTOP_FILE: &str = "crunchytui-sso.desktop";

/// Modo `crunchytui --sso-callback <url>`: lo invoca el navegador vía xdg-open.
pub fn write_callback(url: &str) -> Result<(), String> {
    let path = Config::sso_callback_path().map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(path, url).map_err(|e| e.to_string())
}

/// Registra este binario como manejador de `sso.crunchyroll://` (xdg).
pub fn register_handler() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let apps = directories::BaseDirs::new()
        .ok_or("sin directorio home")?
        .data_dir()
        .join("applications");
    fs::create_dir_all(&apps).map_err(|e| e.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=CrunchyTUI login\nNoDisplay=true\n\
         Exec=\"{}\" --sso-callback %u\nMimeType=x-scheme-handler/sso.crunchyroll;\n",
        exe.display()
    );
    fs::write(apps.join(DESKTOP_FILE), entry).map_err(|e| e.to_string())?;
    let ok = Command::new("xdg-mime")
        .args(["default", DESKTOP_FILE, "x-scheme-handler/sso.crunchyroll"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    ok.then_some(()).ok_or_else(|| "xdg-mime no disponible".into())
}

/// Abre la URL en el navegador por defecto (normal, no en modo app).
pub fn open_browser(url: &str) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    Command::new(opener)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{opener}: {e}"))
}

/// Espera hasta 10 min a que llegue la URL de retorno del navegador.
pub async fn wait_callback() -> Option<String> {
    let path = Config::sso_callback_path().ok()?;
    let _ = fs::remove_file(&path); // descarta restos de un intento anterior
    for _ in 0..1200 {
        if let Ok(url) = tokio::fs::read_to_string(&path).await {
            let _ = tokio::fs::remove_file(&path).await;
            return Some(url);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    None
}
