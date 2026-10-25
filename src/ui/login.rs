//! Pantalla de login: abre la página oficial de Crunchyroll en el navegador
//! y recibe el código OAuth (solo, o pegado a mano).

use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;
use tokio::task::AbortHandle;

use crate::theme;

/// Estado del login. El `verifier` (PKCE) vive solo en memoria.
pub struct LoginState {
    /// URL de autorización abierta en el navegador.
    pub url: Option<String>,
    pub verifier: String,
    /// Lo pegado a mano: URL `sso.crunchyroll://…` o el código.
    pub code: String,
    /// 0 = código, 1 = recordar sesión.
    pub field: usize,
    pub remember: bool,
    /// Tarea que espera la vuelta del navegador.
    pub waiting: Option<AbortHandle>,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            url: None,
            verifier: String::new(),
            code: String::new(),
            field: 0,
            remember: true,
            waiting: None,
        }
    }
}

pub fn draw(f: &mut Frame, s: &LoginState) {
    // Tamaño fijo (recortado a la terminal): con porcentajes se aplastaba.
    let full = f.area();
    let area = full.centered(
        Constraint::Length(76.min(full.width)),
        Constraint::Length(20.min(full.height)),
    );
    f.render_widget(Clear, area);

    let block = Block::bordered()
        .title("  CrunchyTUI — Iniciar sesión ")
        .border_style(theme::title())
        .style(theme::base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [info, code_label, code_row, remember_row, hint] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .horizontal_margin(2)
    .areas(inner);

    let info_lines = match &s.url {
        None => vec![
            Line::from(""),
            Line::from("Pulsa Enter para abrir el login de Crunchyroll en tu navegador."),
            Line::from("Al terminar, acepta abrir «CrunchyTUI login» y volverás aquí solo."),
        ],
        Some(url) => vec![
            Line::from(Span::styled("Esperando al navegador…", theme::title())),
            Line::from("Si no se abrió, copia esta URL (Enter la vuelve a abrir):"),
            Line::from(Span::styled(url.as_str(), theme::dim())),
        ],
    };
    f.render_widget(Paragraph::new(info_lines).wrap(Wrap { trim: false }), info);

    let field_style = |idx: usize| {
        let fg = if s.field == idx { theme::ORANGE } else { theme::DIM };
        ratatui::style::Style::default().fg(fg)
    };

    f.render_widget(
        Paragraph::new("¿No volvió solo? Pega la URL sso.crunchyroll://… o el código:"),
        code_label,
    );
    // Solo la cola del texto pegado: una URL larga no debe salirse del campo.
    let max = code_row.width.saturating_sub(3) as usize;
    let n = s.code.chars().count();
    let shown: String = s.code.chars().skip(n.saturating_sub(max)).collect();
    f.render_widget(
        Paragraph::new(shown.as_str())
            .block(Block::bordered().border_style(field_style(0)))
            .style(theme::base()),
        code_row,
    );

    let check = if s.remember { "[x]" } else { "[ ]" };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(check, field_style(1)),
            Span::styled(" Recordar sesión", theme::base()),
        ])),
        remember_row,
    );

    f.render_widget(
        Paragraph::new("Tab cambia campo · Enter abrir/entrar · Esc salir").style(theme::dim()),
        hint,
    );

    if s.field == 0 {
        f.set_cursor_position((code_row.x + 1 + shown.chars().count() as u16, code_row.y + 1));
    }
}
