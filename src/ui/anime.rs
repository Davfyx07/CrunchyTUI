//! Pantalla de anime: metadatos + lista de temporadas.

use crunchyroll_rs::{Season, Series};
use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme;
use crate::ui::widgets::locale_label;

pub struct AnimeState {
    pub series: Series,
    pub seasons: Vec<Season>,
    pub list: ListState,
}

impl AnimeState {
    pub fn new(series: Series, seasons: Vec<Season>) -> Self {
        let mut list = ListState::default();
        if !seasons.is_empty() {
            list.select(Some(0));
        }
        Self {
            series,
            seasons,
            list,
        }
    }

    pub fn selected_season(&self) -> Option<&Season> {
        self.list.selected().and_then(|i| self.seasons.get(i))
    }
}

pub fn draw(f: &mut Frame, s: &mut AnimeState) {
    let [header, body, hint] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(f.area());

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}", s.series.title),
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

    let [list_area, info_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(body);

    let items: Vec<ListItem> = s
        .seasons
        .iter()
        .map(|season| {
            ListItem::new(Line::from(vec![
                Span::raw(format!("󰄉  {}", season.title)),
                Span::styled(
                    format!("  ({} ep.)", season.number_of_episodes),
                    theme::dim(),
                ),
            ]))
            .style(theme::base())
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .title(" Temporadas ")
                    .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .highlight_style(theme::selected()),
        list_area,
        &mut s.list,
    );

    let series = &s.series;
    let year = series
        .series_launch_year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "—".into());
    let audios = if series.audio_locales.is_empty() {
        "—".to_string()
    } else {
        series
            .audio_locales
            .iter()
            .map(locale_label)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let subs = if series.subtitle_locales.is_empty() {
        "—".to_string()
    } else {
        series
            .subtitle_locales
            .iter()
            .map(locale_label)
            .collect::<Vec<_>>()
            .join(", ")
    };

    let info = vec![
        Line::from(vec![Span::styled("Año: ", theme::dim()), Span::raw(year)]),
        Line::from(vec![
            Span::styled("Episodios: ", theme::dim()),
            Span::raw(series.episode_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Audio: ", theme::dim()),
            Span::raw(audios),
        ]),
        Line::from(vec![
            Span::styled("Subtítulos: ", theme::dim()),
            Span::raw(subs),
        ]),
        Line::from(""),
        Line::from(series.description.as_str()),
    ];

    let info_block = Block::bordered()
        .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
        .border_style(ratatui::style::Style::default().fg(theme::DIM))
        .style(theme::base());
    let info_inner = info_block.inner(info_area);
    f.render_widget(info_block, info_area);
    f.render_widget(
        Paragraph::new(info)
            .wrap(Wrap { trim: true })
            .style(theme::base()),
        info_inner,
    );

    f.render_widget(
        Paragraph::new("↑↓ mover · Enter ver episodios · Esc volver").style(theme::dim()),
        hint,
    );
}
