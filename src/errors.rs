//! Errores de la aplicación.

/// Error raíz de CrunchyTUI. Los errores de la API de Crunchyroll se
/// convierten a texto en el límite async (`api.rs`) para poder enviarlos
/// por canales; aquí solo quedan errores de infraestructura local.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("config inválida: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("no se pudo serializar la config: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("no se pudo determinar el directorio de configuración")]
    NoConfigDir,
    #[error("no se pudo lanzar mpv: {0}")]
    Mpv(String),
}
