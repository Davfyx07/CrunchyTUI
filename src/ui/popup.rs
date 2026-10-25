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

/// Un `\n` dentro de un `Line` llega crudo a la terminal y descuadra la UI:
/// cada línea del mensaje va en su propio `Line`.
fn body(msg: &str, close_hint: bool) -> Vec<Line<'_>> {
    let mut lines = vec![Line::from("")];
    lines.extend(msg.lines().map(Line::from));
    if close_hint {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Esc / Enter para cerrar", theme::dim())));
    }
    lines
}

pub fn draw(f: &mut Frame, popup: &Popup) {
    let (title, color, lines): (&str, ratatui::style::Color, Vec<Line>) = match popup {
        Popup::Error(msg) => (
            " Error ",
            theme::RED,
            body(msg, true),
        ),
        Popup::Info(msg) => (
            " Info ",
            theme::ORANGE,
            body(msg, true),
        ),
        Popup::Help => (
            " Ayuda ",
            theme::PURPLE,
            HELP_LINES.iter().map(|l| Line::from(*l)).collect(),
        ),
        Popup::Busy(msg) => (
            " Cargando ",
            theme::PURPLE,
            body(msg, false),
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
