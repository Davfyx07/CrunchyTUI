//! Pantalla principal.

use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::theme;

/// Ítems del menú: (icono Nerd Font, etiqueta).
pub const ITEMS: &[(&str, &str)] = &[("", "Buscar"), ("󰈆", "Salir")];

#[derive(Default)]
pub struct HomeState {
    pub selected: usize,
}

pub fn draw(f: &mut Frame, s: &HomeState) {
    let area = f.area();
    let [header, body, hint] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled("  CrunchyTUI", theme::title())))
            .block(
                Block::default()
                    .borders(ratatui::widgets::Borders::BOTTOM)
                    .border_style(ratatui::style::Style::default().fg(theme::DIM)),
            )
            .style(theme::base()),
        header,
    );

    let items: Vec<ListItem> = ITEMS
        .iter()
        .map(|(icon, label)| {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(*icon, ratatui::style::Style::default().fg(theme::ORANGE)),
                Span::raw(format!("  {label}")),
            ]))
            .style(theme::base())
        })
        .collect();

    let mut state = ListState::default().with_selected(Some(s.selected));
    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().style(theme::base()))
            .highlight_style(theme::selected()),
        body,
        &mut state,
    );

    f.render_widget(
        Paragraph::new("↑↓ mover · Enter seleccionar · / buscar · ? ayuda · q salir")
            .style(theme::dim()),
        hint,
    );
}
