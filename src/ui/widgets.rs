//! Helpers compartidos de UI y conversión de tipos de la API.

use crunchyroll_rs::media::PosterImages;
use crunchyroll_rs::Locale;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::ListState;

/// Rect centrado con porcentajes del área dada.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    area
}

/// Mueve la selección de una lista hacia abajo (sin wrap).
pub fn list_next(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let i = state
        .selected()
        .map_or(0, |i| (i + 1).min(len.saturating_sub(1)));
    state.select(Some(i));
}

/// Mueve la selección de una lista hacia arriba (sin wrap).
pub fn list_prev(state: &mut ListState) {
    let i = state.selected().map_or(0, |i| i.saturating_sub(1));
    state.select(Some(i));
}

/// Elige una URL de póster de tamaño medio (≈480px de alto) si existe.
pub fn poster_url(images: &PosterImages) -> Option<String> {
    let tall = &images.poster_tall;
    if tall.is_empty() {
        return None;
    }
    let img = tall
        .iter()
        .min_by_key(|i| (i.height as i64 - 480).abs())
        .unwrap();
    Some(img.source.clone())
}

/// "es_419" → "es-419".
pub fn locale_label(locale: &Locale) -> String {
    format!("{locale:?}").replace('_', "-")
}

/// Índice de la opción cuyo label coincide con `wanted` (config), si existe.
pub fn find_locale(labels: &[String], wanted: &str) -> Option<usize> {
    labels.iter().position(|l| l.eq_ignore_ascii_case(wanted))
}

/// Duración de chrono → "23:40".
pub fn fmt_duration(d: &chrono::Duration) -> String {
    let secs = d.num_seconds().max(0);
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_movement() {
        let mut s = ListState::default();
        list_next(&mut s, 3);
        assert_eq!(s.selected(), Some(0));
        list_next(&mut s, 3);
        list_next(&mut s, 3);
        list_next(&mut s, 3); // no pasa del final
        assert_eq!(s.selected(), Some(2));
        list_prev(&mut s);
        list_prev(&mut s);
        list_prev(&mut s); // no pasa del inicio
        assert_eq!(s.selected(), Some(0));
        list_next(&mut s, 0); // lista vacía
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn locale_matching() {
        let labels = vec![
            "ja-JP".to_string(),
            "es-419".to_string(),
            "en-US".to_string(),
        ];
        assert_eq!(find_locale(&labels, "es-419"), Some(1));
        assert_eq!(find_locale(&labels, "ES-419"), Some(1));
        assert_eq!(find_locale(&labels, "fr-FR"), None);
    }
}
