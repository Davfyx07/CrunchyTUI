//! Configuración persistente en `~/.config/crunchytui/config.toml`.

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;

/// Configuración de la aplicación. `session_token` lo escribe la propia
/// app tras un login con "recordar sesión"; nunca contiene la contraseña.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Locale de audio preferido, p.ej. "ja-JP".
    pub audio: String,
    /// Locale de subtítulos preferido, p.ej. "es-419".
    pub subtitle: String,
    /// "auto", "1080", "720" o "480".
    pub quality: String,
    /// Recordar sesión entre ejecuciones.
    pub remember_session: bool,
    /// "refresh:<token>" | "etprt:<token>".
    pub session_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: "ja-JP".into(),
            subtitle: "es-419".into(),
            quality: "auto".into(),
            remember_session: true,
            session_token: None,
        }
    }
}

fn dirs() -> Result<ProjectDirs, AppError> {
    ProjectDirs::from("", "", "crunchytui").ok_or(AppError::NoConfigDir)
}

impl Config {
    /// Ruta del archivo de configuración.
    pub fn path() -> Result<PathBuf, AppError> {
        Ok(dirs()?.config_dir().join("config.toml"))
    }

    /// Directorio de logs: `~/.local/share/crunchytui/logs/`.
    pub fn log_dir() -> Result<PathBuf, AppError> {
        Ok(dirs()?.data_dir().join("logs"))
    }

    /// Carga la config; si no existe o es inválida, usa defaults.
    pub fn load() -> Self {
        (|| -> Result<Self, AppError> { Ok(toml::from_str(&fs::read_to_string(Self::path()?)?)?) })(
        )
        .unwrap_or_default()
    }

    /// Guarda la config con permisos 0600 (contiene el token de sesión).
    pub fn save(&self) -> Result<(), AppError> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, toml::to_string_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_round_trip() {
        let cfg = Config {
            session_token: Some("refresh:abc123".into()),
            ..Config::default()
        };
        let parsed: Config = toml::from_str(&toml::to_string_pretty(&cfg).unwrap()).unwrap();
        assert_eq!(parsed.audio, "ja-JP");
        assert_eq!(parsed.subtitle, "es-419");
        assert_eq!(parsed.quality, "auto");
        assert!(parsed.remember_session);
        assert_eq!(parsed.session_token.as_deref(), Some("refresh:abc123"));
    }

    #[test]
    fn partial_toml_uses_defaults() {
        let parsed: Config = toml::from_str("quality = \"720\"").unwrap();
        assert_eq!(parsed.quality, "720");
        assert_eq!(parsed.audio, "ja-JP");
        assert!(parsed.session_token.is_none());
    }
}
