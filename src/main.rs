//! CrunchyTUI — cliente TUI de Crunchyroll con reproducción vía mpv.

mod api;
mod app;
mod config;
mod errors;
mod player;
mod sso;
mod theme;
mod ui;

use std::fs::File;
use std::io::{self, Stdout};
use std::sync::Arc;

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::config::Config;

fn init_logging() {
    let Ok(dir) = Config::log_dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(file) = File::create(dir.join("crunchytui.log")) else {
        return;
    };
    // `&File` implementa Write, así que Arc<File> sirve de MakeWriter.
    tracing_subscriber::fmt()
        .with_writer(Arc::new(file))
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .init();
}

/// Restaura la terminal aunque haya un panic: nunca dejamos la shell rota.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableBracketedPaste, LeaveAlternateScreen);
        eprintln!("{info}");
    }));
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), DisableBracketedPaste, LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

#[tokio::main]
async fn main() {
    // El navegador nos lanza así al terminar el login (ver sso.rs).
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, url] = args.as_slice() {
        if flag == "--sso-callback" {
            if let Err(e) = sso::write_callback(url) {
                eprintln!("No se pudo entregar el login a CrunchyTUI: {e}");
                std::process::exit(1);
            }
            return;
        }
    }

    init_logging();
    install_panic_hook();

    let mut terminal = match setup_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("No se pudo inicializar la terminal: {e}");
            std::process::exit(1);
        }
    };

    let mut app = App::new(Config::load());
    let result = app.run(&mut terminal).await;

    restore_terminal(&mut terminal);

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
