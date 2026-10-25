//! Wrapper fino sobre `crunchyroll-rs`. Todas las funciones devuelven
//! `Result<_, String>` para que los errores puedan viajar por canales
//! hacia el loop de la TUI y mostrarse en popups.

use crunchyroll_rs::auth::platform_credentials::{
    get_platform_credentials, PlatformCredentials,
};
use crunchyroll_rs::auth::{
    CrunchyrollBuilder, DeviceIdentifier, DevicePlatform, SessionToken,
};
use crunchyroll_rs::list::{WatchHistoryEntry, WatchlistEntry, WatchlistOptions};
use crunchyroll_rs::media::{EpisodeVersion, Stream};
use crunchyroll_rs::search::SearchSeries;
use crunchyroll_rs::{Crunchyroll, Episode, Locale, Season, Series};
use futures::{StreamExt, TryStreamExt};
use tokio::sync::OnceCell;

use crate::player::{PlayRequest, PlaybackTarget};

/// Locale en el que se piden los metadatos (títulos, sinopsis…).
// ponytail: fijo; hacer configurable cuando alguien lo pida.
const UI_LOCALE: Locale = Locale::es_419;

/// Credenciales frescas cacheadas en memoria (una sola vez por proceso).
static PLATFORM_CREDS: OnceCell<PlatformCredentials> = OnceCell::const_new();

/// Obtiene las credenciales de plataforma, cacheándolas tras la primera llamada.
async fn fresh_creds() -> Result<&'static PlatformCredentials, String> {
    PLATFORM_CREDS
        .get_or_try_init(|| async {
            tracing::info!("descargando credenciales de plataforma desde GitHub…");
            get_platform_credentials()
                .await
                .map_err(|e| format!("no se pudieron obtener credenciales de plataforma: {e}"))
        })
        .await
}

/// Identificador de dispositivo del login OAuth; debe ser el mismo al refrescar.
fn phone_device() -> DeviceIdentifier {
    DeviceIdentifier {
        device_type: "ANDROID".into(),
        ..Default::default()
    }
}

/// Builder con credenciales de Android Phone: Crunchyroll ya rechaza
/// email+contraseña (`invalid_grant`) y esta plataforma acepta el login OAuth.
async fn builder() -> Result<CrunchyrollBuilder, String> {
    let creds = fresh_creds().await?;
    Ok(Crunchyroll::builder()
        .locale(UI_LOCALE)
        .platform(
            DevicePlatform::AndroidPhone,
            creds.android_phone.basic_auth_token.clone(),
            Some(creds.android_phone_user_agent()),
        ))
}

/// URL de la página oficial de login (OAuth + PKCE "plain"). Al terminar, el
/// navegador redirige a `sso.crunchyroll://auth?code=…`.
pub async fn sso_url(verifier: &str) -> Result<String, String> {
    let client_id = &fresh_creds().await?.android_phone.client_id;
    let anonymous_id = uuid::Uuid::new_v4();
    Ok(format!(
        "https://sso.crunchyroll.com/authorize?client_id={client_id}\
         &redirect_uri=sso.crunchyroll://auth&scope=offline_access\
         &code_challenge={verifier}&code_challenge_method=plain\
         &response_type=code&anonymous_id={anonymous_id}"
    ))
}

/// Canjea el código OAuth por una sesión.
pub async fn login_oauth(code: &str, verifier: &str) -> Result<(Crunchyroll, SessionToken), String> {
    let cr = builder()
        .await?
        .login_with_oauth_code(code, verifier, phone_device())
        .await
        .map_err(|e| e.to_string())?;
    let token = cr.session_token().await;
    Ok((cr, token))
}

/// Saca el `code` de `sso.crunchyroll://auth?code=…`; sin `?` se asume que
/// lo pegado ya es el código.
pub fn extract_code(input: &str) -> Option<String> {
    let input = input.trim();
    let code = match input.split_once('?') {
        Some((_, query)) => query.split('&').find_map(|kv| kv.strip_prefix("code="))?,
        None => input,
    };
    (!code.is_empty()).then(|| code.to_string())
}

/// Login a partir de un token guardado ("refresh:<t>" | "etprt:<t>").
pub async fn login_saved(stored: &str) -> Result<(Crunchyroll, SessionToken), String> {
    let b = builder().await?;
    let cr = if let Some(t) = stored.strip_prefix("refresh:") {
        b.login_with_refresh_token(t, phone_device())
            .await
    } else if let Some(t) = stored.strip_prefix("etprt:") {
        b.login_with_etp_rt(t, DeviceIdentifier::default())
            .await
    } else {
        return Err("token de sesión inválido".into());
    }
    .map_err(|e| e.to_string())?;
    let token = cr.session_token().await;
    Ok((cr, token))
}

/// Serializa el token de sesión para guardarlo en disco.
pub fn encode_token(token: &SessionToken) -> Option<String> {
    match token {
        SessionToken::RefreshToken(t) => Some(format!("refresh:{t}")),
        SessionToken::EtpRt(t) => Some(format!("etprt:{t}")),
        SessionToken::Anonymous => None,
    }
}

/// Búsqueda de series, hasta `limit` resultados.
pub async fn search(cr: &Crunchyroll, query: String) -> Result<Vec<SearchSeries>, String> {
    cr.query(query)
        .series
        .take(20)
        .try_collect()
        .await
        .map_err(|e| e.to_string())
}

/// Obtiene los episodios en reproducción reciente ("Continuar viendo").
pub async fn continue_watching(cr: &Crunchyroll) -> Result<Vec<WatchHistoryEntry>, String> {
    cr.watch_history()
        .take(20)
        .try_collect()
        .await
        .map_err(|e| e.to_string())
}

/// Obtiene la lista de seguimiento del usuario ("Mi Lista").
pub async fn watchlist(cr: &Crunchyroll) -> Result<Vec<WatchlistEntry>, String> {
    cr.watchlist(WatchlistOptions::default())
        .await
        .map_err(|e| e.to_string())
}

/// Convierte un resultado de búsqueda en serie y carga sus temporadas.
pub async fn seasons(result: SearchSeries) -> Result<(Series, Vec<Season>), String> {
    let series: Series = result.into();
    series_seasons(series).await
}

/// Carga temporadas directamente a partir de un struct `Series`.
pub async fn series_seasons(series: Series) -> Result<(Series, Vec<Season>), String> {
    let seasons = series.seasons().await.map_err(|e| e.to_string())?;
    Ok((series, seasons))
}

/// Carga los episodios de una temporada.
pub async fn episodes(season: Season) -> Result<(Season, Vec<Episode>), String> {
    let episodes = season.episodes().await.map_err(|e| e.to_string())?;
    Ok((season, episodes))
}

/// URL oficial del episodio, usada cuando el stream tiene DRM y la
/// reproducción se delega a una ventana de navegador.
fn watch_url(ep: &Episode) -> String {
    if ep.slug.is_empty() {
        format!("https://www.crunchyroll.com/watch/{}", ep.id)
    } else {
        format!("https://www.crunchyroll.com/watch/{}/{}", ep.id, ep.slug)
    }
}

/// Resuelve el stream de un episodio aplicando la versión de audio y los
/// subtítulos elegidos. Devuelve mpv si el stream es directo o una ventana
/// de navegador si está protegido con DRM.
pub async fn resolve_play(
    episode: Episode,
    version: Option<EpisodeVersion>,
    subtitle: Option<Locale>,
    quality: String,
) -> Result<PlaybackTarget, String> {
    let ep = match version {
        Some(v) => v.episode().await.map_err(|e| e.to_string())?,
        None => episode,
    };
    let number = ep
        .episode_number
        .map(|n| n.to_string())
        .unwrap_or_else(|| ep.sequence_number.to_string());
    let title = format!("{} — E{}", ep.series_title, number);

    let stream: Stream = ep.stream().await.map_err(|e| e.to_string())?;
    if stream.session.uses_stream_limits || !stream.drm.name.is_empty() {
        tracing::info!(episode = %ep.id, drm = %stream.drm.name, "stream con DRM; usando navegador");
        return Ok(PlaybackTarget::Browser {
            url: watch_url(&ep),
            title,
        });
    }

    let sub_url = subtitle.and_then(|loc| {
        stream
            .subtitles
            .iter()
            .find(|s| s.locale == loc)
            .map(|s| s.url.clone())
    });
    Ok(PlaybackTarget::Mpv(PlayRequest {
        url: stream.url.clone(),
        title,
        sub_url,
        quality,
    }))
}

/// Descarga y decodifica una portada usando el cliente HTTP interno de la
/// sesión (hereda su user-agent y TLS).
pub async fn poster(cr: &Crunchyroll, url: String) -> Result<image::DynamicImage, String> {
    let bytes = cr
        .client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trip() {
        let rt = SessionToken::RefreshToken("tok123".into());
        assert_eq!(encode_token(&rt).as_deref(), Some("refresh:tok123"));
        let etp = SessionToken::EtpRt("cookie".into());
        assert_eq!(encode_token(&etp).as_deref(), Some("etprt:cookie"));
        assert!(encode_token(&SessionToken::Anonymous).is_none());
    }

    #[test]
    fn code_extraction() {
        assert_eq!(extract_code("sso.crunchyroll://auth?code=abc&state=x").as_deref(), Some("abc"));
        assert_eq!(extract_code("  abc123\n").as_deref(), Some("abc123"));
        assert_eq!(extract_code("sso.crunchyroll://auth?error=denied"), None);
        assert_eq!(extract_code("   "), None);
    }

    #[test]
    fn watch_url_format() {
        let mut ep = Episode::default();
        ep.id = "GRDQPM1ZY".into();
        ep.slug = "alone-and-lonesome".into();
        assert_eq!(
            watch_url(&ep),
            "https://www.crunchyroll.com/watch/GRDQPM1ZY/alone-and-lonesome"
        );
        ep.slug.clear();
        assert_eq!(
            watch_url(&ep),
            "https://www.crunchyroll.com/watch/GRDQPM1ZY"
        );
    }
}
