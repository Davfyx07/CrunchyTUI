//! Pantalla de episodios + popup de opciones de reproducción.

use crunchyroll_rs::media::EpisodeVersion;
use crunchyroll_rs::{Episode, Locale, Season};
use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::ui::widgets::{centered_rect, find_locale, fmt_duration, locale_label};

/// Calidades elegibles, en orden.
pub const QUALITIES: [&str; 4] = ["auto", "1080", "720", "480"];

pub struct EpisodesState {
    pub season: Season,
    pub episodes: Vec<Episode>,
    pub list: ListState,
    /// Popup de opciones de reproducción, si está abierto.
    pub play: Option<PlayPopup>,
}

impl EpisodesState {
    pub fn new(season: Season, episodes: Vec<Episode>) -> Self {
        let mut list = ListState::default();
        if !episodes.is_empty() {
            list.select(Some(0));
        }
        Self {
            season,
            episodes,
            list,
            play: None,
        }
    }
}

/// Opciones de reproducción para un episodio concreto.
pub struct PlayPopup {
    pub episode_idx: usize,
    /// (etiqueta, versión de la API). `None` = el propio episodio.
    pub audios: Vec<(String, Option<EpisodeVersion>)>,
    pub audio_sel: usize,
    /// (etiqueta, locale). `None` = sin subtítulos.
    pub subs: Vec<(String, Option<Locale>)>,
    pub sub_sel: usize,
    pub quality_sel: usize,
    /// Sección con foco: 0 audio, 1 subtítulos, 2 calidad.
    pub focus: usize,
}

impl PlayPopup {
    pub fn new(
        episode_idx: usize,
        ep: &Episode,
        want_audio: &str,
        want_sub: &str,
        want_quality: &str,
    ) -> Self {
        let mut versions = ep.versions.clone();
        versions.sort_by_key(|v| !v.original);
        let mut audios: Vec<(String, Option<EpisodeVersion>)> = versions
            .into_iter()
            .map(|v| {
                let label = if v.original {
                    format!("{} (original)", locale_label(&v.audio_locale))
                } else {
                    locale_label(&v.audio_locale)
                };
                (label, Some(v))
            })
            .collect();
        if audios.is_empty() {
            audios.push((locale_label(&ep.audio_locale), None));
        }
        let labels: Vec<String> = audios.iter().map(|(l, _)| l.clone()).collect();
        let audio_sel = find_locale(&labels, want_audio)
            .or_else(|| {
                audios
                    .iter()
                    .position(|(_, v)| v.as_ref().is_some_and(|v| v.original))
            })
            .unwrap_or(0);

        let mut subs: Vec<(String, Option<Locale>)> = vec![("Ninguno".into(), None)];
        subs.extend(
            ep.subtitle_locales
                .iter()
                .map(|l| (locale_label(l), Some(l.clone()))),
        );
        let sub_labels: Vec<String> = subs.iter().map(|(l, _)| l.clone()).collect();
        let sub_sel = find_locale(&sub_labels, want_sub).unwrap_or(0);

        let quality_sel = QUALITIES
            .iter()
            .position(|q| q.eq_ignore_ascii_case(want_quality))
            .unwrap_or(0);

        Self {
            episode_idx,
            audios,
            audio_sel,
            subs,
            sub_sel,
            quality_sel,
            focus: 0,
        }
    }
}

fn radio_list(
    title: &str,
    labels: &[String],
    selected: usize,
    focused: bool,
) -> Paragraph<'static> {
    let mut lines = vec![Line::from(Span::styled(
        title.to_string(),
        if focused {
            theme::title()
        } else {
            theme::dim()
        },
    ))];
    for (i, label) in labels.iter().enumerate() {
        let (mark, style) = if i == selected {
            ("●", ratatui::style::Style::default().fg(theme::ORANGE))
        } else {
            ("○", theme::base())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{mark} "), style),
            Span::styled(label.clone(), style),
        ]));
    }
    Paragraph::new(lines)
}

pub fn draw(f: &mut Frame, s: &mut EpisodesState) {
    let [header, body, hint] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(f.area());

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}", s.season.title),
            theme::title(),
        )))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(ratatui::style::Style::default().fg(theme::DIM)),
        )
        .style(theme::base()),
        header,
    );

    let items: Vec<ListItem> = s
        .episodes
        .iter()
        .map(|ep| {
            let number = ep
                .episode_number
                .map(|n| n.to_string())
                .unwrap_or_else(|| ep.sequence_number.to_string());
            let lock = if ep.is_premium_only { "  " } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{number:>4}  "), theme::dim()),
                Span::raw(ep.title.clone()),
                Span::styled(format!("  {}", fmt_duration(&ep.duration)), theme::dim()),
                Span::styled(lock, ratatui::style::Style::default().fg(theme::ORANGE)),
            ]))
            .style(theme::base())
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .title(" Episodios ")
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .highlight_style(theme::selected()),
        body,
        &mut s.list,
    );

    f.render_widget(
        Paragraph::new("↑↓ mover · Enter opciones · Esc volver").style(theme::dim()),
        hint,
    );

    if let Some(play) = &s.play {
        draw_play_popup(f, play);
    }
}

pub fn draw_play_popup(f: &mut Frame, play: &PlayPopup) {
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 󰐊 Reproducir ")
        .border_style(theme::title())
        .style(theme::base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [cols, hint] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);
    let [audio_area, sub_area, quality_area] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(35),
        Constraint::Percentage(25),
    ])
    .horizontal_margin(1)
    .areas(cols);

    let audio_labels: Vec<String> = play.audios.iter().map(|(l, _)| l.clone()).collect();
    let sub_labels: Vec<String> = play.subs.iter().map(|(l, _)| l.clone()).collect();
    let quality_labels: Vec<String> = QUALITIES.iter().map(|q| q.to_string()).collect();

    f.render_widget(
        radio_list("Audio", &audio_labels, play.audio_sel, play.focus == 0),
        audio_area,
    );
    f.render_widget(
        radio_list("Subtítulos", &sub_labels, play.sub_sel, play.focus == 1),
        sub_area,
    );
    f.render_widget(
        radio_list(
            "Calidad",
            &quality_labels,
            play.quality_sel,
            play.focus == 2,
        ),
        quality_area,
    );

    f.render_widget(
        Paragraph::new("Tab sección · ↑↓ elegir · Enter reproducir · Esc cancelar")
            .style(theme::dim()),
        hint,
    );
}
