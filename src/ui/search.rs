//! Pantalla de búsqueda incremental con portadas.

use crunchyroll_rs::search::SearchSeries;
use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

use crate::theme;
use crate::ui::widgets;

#[derive(Default)]
pub struct SearchState {
    pub input: String,
    pub results: Vec<SearchSeries>,
    pub list: ListState,
    /// Generación del debounce; respuestas con generación vieja se descartan.
    pub gen: u64,
    pub poster: Option<StatefulProtocol>,
    /// Generación de la petición de portada en curso.
    pub poster_gen: u64,
    pub loading: bool,
}

impl SearchState {
    /// URL del póster del resultado seleccionado, si hay.
    pub fn selected_poster_url(&self) -> Option<String> {
        let idx = self.list.selected()?;
        self.results
            .get(idx)
            .and_then(|s| widgets::poster_url(&s.images))
    }

    pub fn selected(&self) -> Option<&SearchSeries> {
        self.list.selected().and_then(|i| self.results.get(i))
    }
}

pub fn draw(f: &mut Frame, s: &mut SearchState) {
    let [input_area, body, hint] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(f.area());

    f.render_widget(
        Paragraph::new(s.input.as_str())
            .block(
                Block::bordered()
                    .title("  Buscar ")
                    .border_style(theme::title()),
            )
            .style(theme::base()),
        input_area,
    );
    f.set_cursor_position((
        input_area.x + 1 + s.input.chars().count() as u16,
        input_area.y + 1,
    ));

    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(body);

    let items: Vec<ListItem> = s
        .results
        .iter()
        .map(|r| {
            let year = r
                .series_launch_year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "—".into());
            ListItem::new(Line::from(vec![
                Span::raw(r.title.clone()),
                Span::styled(
                    format!("  ({year} · {} temp.)", r.season_count),
                    theme::dim(),
                ),
            ]))
            .style(theme::base())
        })
        .collect();

    let title = if s.loading {
        " Resultados… "
    } else {
        " Resultados "
    };
    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .title(title)
                    .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .highlight_style(theme::selected()),
        list_area,
        &mut s.list,
    );

    let detail_block = Block::bordered()
        .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
        .border_style(ratatui::style::Style::default().fg(theme::DIM))
        .style(theme::base());
    let detail_inner = detail_block.inner(detail_area);
    f.render_widget(detail_block, detail_area);

    match &mut s.poster {
        Some(protocol) => {
            f.render_stateful_widget(
                StatefulImage::<StatefulProtocol>::new(),
                detail_inner,
                protocol,
            );
        }
        None => {
            let text = match s.selected() {
                Some(sel) => sel.description.clone(),
                None => "Escribe para buscar.".to_string(),
            };
            f.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: true })
                    .style(theme::dim()),
                detail_inner,
            );
        }
    }

    f.render_widget(
        Paragraph::new("Escribe para buscar · ↑↓ mover · Enter ver serie · Esc volver")
            .style(theme::dim()),
        hint,
    );
}
