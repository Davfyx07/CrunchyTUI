//! Pantalla principal / Dashboard con pestañas: Continuar viendo, Mi Lista y Buscar.

use crunchyroll_rs::list::{WatchHistoryEntry, WatchlistEntry};
use crunchyroll_rs::MediaCollection;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

use crate::theme;
use crate::ui::episodes::{draw_play_popup, PlayPopup};
use crate::ui::widgets::{self, fmt_duration, locale_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeTab {
    History = 0,
    Watchlist = 1,
}

pub struct HomeState {
    pub tab: HomeTab,
    pub history: Vec<WatchHistoryEntry>,
    pub history_list: ListState,
    pub watchlist: Vec<WatchlistEntry>,
    pub watchlist_list: ListState,
    pub poster: Option<StatefulProtocol>,
    pub poster_gen: u64,
    pub loading: bool,
    pub play: Option<PlayPopup>,
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            tab: HomeTab::History,
            history: Vec::new(),
            history_list: ListState::default(),
            watchlist: Vec::new(),
            watchlist_list: ListState::default(),
            poster: None,
            poster_gen: 0,
            loading: false,
            play: None,
        }
    }
}

impl HomeState {
    pub fn selected_history(&self) -> Option<&WatchHistoryEntry> {
        let idx = self.history_list.selected()?;
        self.history.get(idx)
    }

    pub fn selected_watchlist(&self) -> Option<&WatchlistEntry> {
        let idx = self.watchlist_list.selected()?;
        self.watchlist.get(idx)
    }

    pub fn selected_poster_url(&self) -> Option<String> {
        match self.tab {
            HomeTab::History => {
                let entry = self.selected_history()?;
                if let Some(MediaCollection::Episode(ref ep)) = entry.panel {
                    ep.images.first().map(|img| img.source.clone())
                } else {
                    None
                }
            }
            HomeTab::Watchlist => {
                let entry = self.selected_watchlist()?;
                if let MediaCollection::Series(ref series) = entry.panel {
                    widgets::poster_url(&series.images)
                } else {
                    None
                }
            }
        }
    }
}

fn fmt_sec(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn progress_bar(playhead: u32, total_sec: u32, fully_watched: bool) -> String {
    if fully_watched || (total_sec > 0 && playhead >= total_sec) {
        return "[██████████] Visto".to_string();
    }
    let pct = if total_sec > 0 {
        ((playhead as f64 / total_sec as f64) * 100.0).clamp(0.0, 100.0) as u32
    } else {
        0
    };
    let filled = (pct as usize * 10) / 100;
    let empty = 10 - filled;
    format!(
        "[{}{}] {}% ({}/{})",
        "█".repeat(filled),
        "░".repeat(empty),
        pct,
        fmt_sec(playhead),
        fmt_sec(total_sec)
    )
}

pub fn draw(f: &mut Frame, s: &mut HomeState) {
    let area = f.area();
    let [header, body, hint] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // --- Header con pestañas ---
    let (tab0_style, tab1_style) = match s.tab {
        HomeTab::History => (theme::selected(), theme::base()),
        HomeTab::Watchlist => (theme::base(), theme::selected()),
    };

    let header_line = Line::from(vec![
        Span::styled("  CrunchyTUI  ", theme::title()),
        Span::styled(" │ ", theme::dim()),
        Span::styled(" [1]  Continuar viendo ", tab0_style),
        Span::raw("  "),
        Span::styled(" [2]  Mi Lista ", tab1_style),
        Span::styled(" │ ", theme::dim()),
        Span::styled(" [/]  Buscar ", theme::dim()),
    ]);

    f.render_widget(
        Paragraph::new(header_line)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .style(theme::base()),
        header,
    );

    // --- Body: Layout de 2 columnas (lista a la izquierda, detalles a la derecha) ---
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(body);

    if s.loading {
        let loading_block = Block::bordered()
            .title(" Panel Principal ")
            .border_style(ratatui::style::Style::default().fg(theme::DIM))
            .style(theme::base());
        f.render_widget(
            Paragraph::new("  Cargando tu contenido de Crunchyroll…")
                .block(loading_block)
                .style(theme::dim()),
            body,
        );
    } else {
        match s.tab {
            HomeTab::History => draw_history(f, s, list_area, detail_area),
            HomeTab::Watchlist => draw_watchlist(f, s, list_area, detail_area),
        }
    }

    // --- Barra inferior de atajos ---
    f.render_widget(
        Paragraph::new("↑↓ navegar · Enter reproducir / ver · Tab/1-2 pestañas · / buscar · r recargar · q salir")
            .style(theme::dim()),
        hint,
    );

    if let Some(play) = &s.play {
        draw_play_popup(f, play);
    }
}

fn draw_history(f: &mut Frame, s: &mut HomeState, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = if s.history.is_empty() {
        vec![ListItem::new("  No hay episodios en el historial.").style(theme::dim())]
    } else {
        s.history
            .iter()
            .map(|entry| {
                if let Some(MediaCollection::Episode(ref ep)) = entry.panel {
                    let number = ep
                        .episode_number
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| ep.sequence_number.to_string());
                    let total_sec = ep.duration.num_seconds().max(1) as u32;
                    let bar = progress_bar(entry.playhead, total_sec, entry.fully_watched);
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(format!("  {} ", ep.series_title), theme::title()),
                            Span::styled(format!("— E{number} "), theme::dim()),
                            Span::raw(ep.title.clone()),
                        ]),
                        Line::from(vec![
                            Span::raw("    "),
                            Span::styled(bar, ratatui::style::Style::default().fg(theme::ORANGE)),
                        ]),
                    ])
                    .style(theme::base())
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(entry.id.clone(), theme::dim()),
                    ]))
                    .style(theme::base())
                }
            })
            .collect()
    };

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .title(format!("  Continuar viendo ({}) ", s.history.len()))
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .highlight_style(theme::selected()),
        list_area,
        &mut s.history_list,
    );

    draw_history_details(f, s, detail_area);
}

fn draw_history_details(f: &mut Frame, s: &mut HomeState, area: Rect) {
    let block = Block::bordered()
        .title(" Detalles del episodio ")
        .border_style(ratatui::style::Style::default().fg(theme::DIM))
        .style(theme::base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let idx = s.history_list.selected();
    let lines = idx.and_then(|i| s.history.get(i)).and_then(|entry| {
        let MediaCollection::Episode(ref ep) = entry.panel.as_ref()? else {
            return None;
        };
        let number = ep
            .episode_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| ep.sequence_number.to_string());
        Some(vec![
            Line::from(Span::styled(ep.series_title.clone(), theme::title())),
            Line::from(vec![
                Span::styled(format!("Episodio {number}: "), theme::dim()),
                Span::raw(ep.title.clone()),
            ]),
            Line::from(vec![
                Span::styled("Duración: ", theme::dim()),
                Span::raw(fmt_duration(&ep.duration)),
                Span::styled(" · Audio: ", theme::dim()),
                Span::raw(locale_label(&ep.audio_locale)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Sinopsis:", theme::dim())),
            Line::from(ep.description.clone()),
        ])
    });

    let Some(lines) = lines else { return };

    let [poster_area, text_area] =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(inner);

    if let Some(protocol) = &mut s.poster {
        f.render_stateful_widget(
            StatefulImage::<StatefulProtocol>::new(),
            poster_area,
            protocol,
        );
    }

    f.render_widget(
        Paragraph::new(lines).style(theme::base()).wrap(Wrap { trim: true }),
        text_area,
    );
}

fn draw_watchlist(f: &mut Frame, s: &mut HomeState, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = if s.watchlist.is_empty() {
        vec![ListItem::new("  Tu lista está vacía. Agrega series desde la búsqueda.").style(theme::dim())]
    } else {
        s.watchlist
            .iter()
            .map(|entry| {
                if let MediaCollection::Series(ref series) = entry.panel {
                    let fav = if entry.is_favorite { " ★ " } else { "   " };
                    let new_badge = if entry.new { " [NUEVO]" } else { "" };
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(fav, ratatui::style::Style::default().fg(theme::ORANGE)),
                            Span::styled(series.title.clone(), theme::title()),
                            Span::styled(new_badge, ratatui::style::Style::default().fg(theme::ORANGE)),
                        ]),
                        Line::from(vec![
                            Span::raw("      "),
                            Span::styled(
                                format!(
                                    "{} temporadas · {} episodios",
                                    series.season_count, series.episode_count
                                ),
                                theme::dim(),
                            ),
                        ]),
                    ])
                    .style(theme::base())
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("Entrada", theme::dim()),
                    ]))
                    .style(theme::base())
                }
            })
            .collect()
    };

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .title(format!("  Mi Lista ({}) ", s.watchlist.len()))
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .highlight_style(theme::selected()),
        list_area,
        &mut s.watchlist_list,
    );

    draw_watchlist_details(f, s, detail_area);
}

fn draw_watchlist_details(f: &mut Frame, s: &mut HomeState, area: Rect) {
    let block = Block::bordered()
        .title(" Detalles de la serie ")
        .border_style(ratatui::style::Style::default().fg(theme::DIM))
        .style(theme::base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let idx = s.watchlist_list.selected();
    let lines = idx.and_then(|i| s.watchlist.get(i)).and_then(|entry| {
        let MediaCollection::Series(ref series) = entry.panel else {
            return None;
        };
        Some(vec![
            Line::from(Span::styled(series.title.clone(), theme::title())),
            Line::from(vec![
                Span::styled(
                    format!(
                        "{} temporadas · {} episodios",
                        series.season_count, series.episode_count
                    ),
                    theme::dim(),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled("Sinopsis:", theme::dim())),
            Line::from(series.description.clone()),
        ])
    });

    let Some(lines) = lines else { return };

    let [poster_area, text_area] =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(inner);

    if let Some(protocol) = &mut s.poster {
        f.render_stateful_widget(
            StatefulImage::<StatefulProtocol>::new(),
            poster_area,
            protocol,
        );
    }

    f.render_widget(
        Paragraph::new(lines).style(theme::base()).wrap(Wrap { trim: true }),
        text_area,
    );
}
