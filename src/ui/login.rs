//! Pantalla de login: email, contraseña y "recordar sesión".

use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::ui::widgets::centered_rect;

/// Estado del formulario de login. La contraseña vive solo en memoria y
/// se descarta tras el login.
pub struct LoginState {
    pub email: String,
    pub password: String,
    /// 0 = email, 1 = contraseña, 2 = recordar sesión.
    pub field: usize,
    pub remember: bool,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            email: String::new(),
            password: String::new(),
            field: 0,
            remember: true,
        }
    }
}

pub fn draw(f: &mut Frame, s: &LoginState) {
    let area = centered_rect(45, 45, f.area());
    f.render_widget(Clear, area);

    let block = Block::bordered()
        .title("  CrunchyTUI — Iniciar sesión ")
        .border_style(theme::title())
        .style(theme::base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [email_label, email_row, pass_label, pass_row, remember_row, _pad, hint] =
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .horizontal_margin(2)
        .areas(inner);

    let field_style = |idx: usize| {
        if s.field == idx {
            ratatui::style::Style::default().fg(theme::ORANGE)
        } else {
            ratatui::style::Style::default().fg(theme::DIM)
        }
    };

    f.render_widget(Paragraph::new("Correo"), email_label);
    f.render_widget(
        Paragraph::new(s.email.as_str())
            .block(Block::bordered().border_style(field_style(0)))
            .style(theme::base()),
        email_row,
    );

    f.render_widget(Paragraph::new("Contraseña"), pass_label);
    f.render_widget(
        Paragraph::new("•".repeat(s.password.chars().count()))
            .block(Block::bordered().border_style(field_style(1)))
            .style(theme::base()),
        pass_row,
    );

    let check = if s.remember { "[x]" } else { "[ ]" };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(check, field_style(2)),
            Span::styled(" Recordar sesión", theme::base()),
        ])),
        remember_row,
    );

    f.render_widget(
        Paragraph::new("Tab cambia campo · Enter entrar · Esc salir").style(theme::dim()),
        hint,
    );

    // Cursor visible en el campo activo.
    let (row, text_len) = match s.field {
        0 => (email_row, s.email.chars().count()),
        1 => (pass_row, s.password.chars().count()),
        _ => (email_row, 0),
    };
    if s.field <= 1 {
        f.set_cursor_position((row.x + 1 + text_len as u16, row.y + 1));
    }
}
