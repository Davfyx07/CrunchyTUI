//! Wrapper fino sobre `crunchyroll-rs`. Todas las funciones devuelven
//! `Result<_, String>` para que los errores puedan viajar por canales
//! hacia el loop de la TUI y mostrarse en popups.

use crunchyroll_rs::crunchyroll::{DeviceIdentifier, SessionToken};
use crunchyroll_rs::media::{EpisodeVersion, Stream};
use crunchyroll_rs::search::SearchSeries;
use crunchyroll_rs::{Crunchyroll, Episode, Locale, Season, Series};
use futures::{StreamExt, TryStreamExt};

use crate::player::PlayRequest;

/// Locale en el que se piden los metadatos (títulos, sinopsis…).
// ponytail: fijo; hacer configurable cuando alguien lo pida.
const UI_LOCALE: Locale = Locale::es_419;

fn builder() -> crunchyroll_rs::crunchyroll::CrunchyrollBuilder {
    Crunchyroll::builder().locale(UI_LOCALE)
}

/// Login con email y contraseña. Devuelve la sesión y su token persistible.
pub async fn login(email: &str, password: &str) -> Result<(Crunchyroll, SessionToken), String> {
    let cr = builder()
        .login_with_credentials(email, password, DeviceIdentifier::default())
        .await
        .map_err(|e| e.to_string())?;
    let token = cr.session_token().await;
    Ok((cr, token))
}

/// Login a partir de un token guardado ("refresh:<t>" | "etprt:<t>").
pub async fn login_saved(stored: &str) -> Result<(Crunchyroll, SessionToken), String> {
    let cr = if let Some(t) = stored.strip_prefix("refresh:") {
        builder()
            .login_with_refresh_token(t, DeviceIdentifier::default())
            .await
    } else if let Some(t) = stored.strip_prefix("etprt:") {
        builder()
            .login_with_etp_rt(t, DeviceIdentifier::default())
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

/// Convierte un resultado de búsqueda en serie y carga sus temporadas.
pub async fn seasons(result: SearchSeries) -> Result<(Series, Vec<Season>), String> {
    let series: Series = result.into();
    let seasons = series.seasons().await.map_err(|e| e.to_string())?;
    Ok((series, seasons))
}

/// Carga los episodios de una temporada.
pub async fn episodes(season: Season) -> Result<(Season, Vec<Episode>), String> {
    let episodes = season.episodes().await.map_err(|e| e.to_string())?;
    Ok((season, episodes))
}

/// Resuelve el stream de un episodio aplicando la versión de audio y los
/// subtítulos elegidos, y construye la petición para mpv.
pub async fn resolve_play(
    episode: Episode,
    version: Option<EpisodeVersion>,
    subtitle: Option<Locale>,
    quality: String,
) -> Result<PlayRequest, String> {
    let ep = match version {
        Some(v) => v.episode().await.map_err(|e| e.to_string())?,
        None => episode,
    };
    let stream: Stream = ep.stream().await.map_err(|e| e.to_string())?;
    if stream.session.uses_stream_limits {
        return Err(
            "Este stream está protegido por DRM y no se puede reproducir con mpv directamente."
                .into(),
        );
    }
    let sub_url = subtitle.and_then(|loc| {
        stream
            .subtitles
            .iter()
            .find(|s| s.locale == loc)
            .map(|s| s.url.clone())
    });
    let number = ep
        .episode_number
        .map(|n| n.to_string())
        .unwrap_or_else(|| ep.sequence_number.to_string());
    Ok(PlayRequest {
        url: stream.url.clone(),
        title: format!("{} — E{}", ep.series_title, number),
        sub_url,
        quality,
    })
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
}
