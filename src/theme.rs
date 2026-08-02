//! Paleta de colores (tema oscuro, acento Crunchyroll).

use ratatui::style::{Color, Modifier, Style};

/// Naranja Crunchyroll.
pub const ORANGE: Color = Color::Rgb(244, 117, 33);
/// Morado de acento secundario.
pub const PURPLE: Color = Color::Rgb(167, 139, 250);
/// Fondo principal.
pub const BG: Color = Color::Rgb(18, 18, 26);
/// Texto principal.
pub const FG: Color = Color::Rgb(230, 230, 238);
/// Texto secundario / deshabilitado.
pub const DIM: Color = Color::Rgb(120, 120, 135);
/// Fondo de elemento seleccionado.
pub const SEL_BG: Color = Color::Rgb(42, 42, 58);
/// Errores.
pub const RED: Color = Color::Rgb(240, 90, 90);

/// Estilo base de texto sobre el fondo.
pub fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}

/// Estilo de texto atenuado.
pub fn dim() -> Style {
    Style::default().fg(DIM).bg(BG)
}

/// Estilo de la línea seleccionada en listas.
pub fn selected() -> Style {
    Style::default()
        .fg(ORANGE)
        .bg(SEL_BG)
        .add_modifier(Modifier::BOLD)
}

/// Estilo de títulos de bloque.
pub fn title() -> Style {
    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
}
