//! Popups modales: errores, información, ayuda y "cargando".

use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme;
use crate::ui::widgets::centered_rect;

/// Popup modal de la aplicación. Mientras hay uno abierto, las teclas
/// quedan capturadas por él.
pub enum Popup {
    /// Error con mensaje; se cierra con Esc/Enter/q.
    Error(String),
    /// Información; se cierra con Esc/Enter/q.
    Info(String),
    /// Ayuda de atajos; se cierra con Esc/Enter/q/?.
    Help,
    /// Operación en curso; bloquea la entrada hasta que termine.
    Busy(String),
}

const HELP_LINES: &[&str] = &[
    "↑/↓        Moverse en listas",
    "Enter      Seleccionar / confirmar",
    "Esc        Atrás / cerrar",
    "/          Ir a búsqueda",
    "Tab        Cambiar campo o sección",
    "?          Esta ayuda",
    "q          Salir",
    "",
    "En búsqueda y login el texto se escribe directamente;",
    "ahí las letras no son atajos.",
];

pub fn draw(f: &mut Frame, popup: &Popup) {
    let (title, color, lines): (&str, ratatui::style::Color, Vec<Line>) = match popup {
        Popup::Error(msg) => (
            " Error ",
            theme::RED,
            vec![
                Line::from(""),
                Line::from(msg.as_str()),
                Line::from(""),
                Line::from(Span::styled("Esc / Enter para cerrar", theme::dim())),
            ],
        ),
        Popup::Info(msg) => (
            " Info ",
            theme::ORANGE,
            vec![
                Line::from(""),
                Line::from(msg.as_str()),
                Line::from(""),
                Line::from(Span::styled("Esc / Enter para cerrar", theme::dim())),
            ],
        ),
        Popup::Help => (
            " Ayuda ",
            theme::PURPLE,
            HELP_LINES.iter().map(|l| Line::from(*l)).collect(),
        ),
        Popup::Busy(msg) => (
            " Cargando ",
            theme::PURPLE,
            vec![Line::from(""), Line::from(msg.as_str())],
        ),
    };

    let height = match popup {
        Popup::Help => 55,
        _ => 30,
    };
    let area = centered_rect(55, height, f.area());
    f.render_widget(Clear, area);

    let block = Block::bordered()
        .title(title)
        .border_style(ratatui::style::Style::default().fg(color))
        .style(theme::base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [content] = Layout::vertical([Constraint::Min(1)])
        .horizontal_margin(2)
        .areas(inner);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), content);
}
