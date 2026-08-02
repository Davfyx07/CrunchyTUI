//! Estado global de la aplicación, loop de eventos y transiciones.

use std::io::Stdout;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use crunchyroll_rs::crunchyroll::SessionToken;
use crunchyroll_rs::search::SearchSeries;
use crunchyroll_rs::{Crunchyroll, Episode, Season, Series};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::Block;
use ratatui::{Frame, Terminal};
use ratatui_image::picker::Picker;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::config::Config;
use crate::errors::AppError;
use crate::player::PlayRequest;
use crate::ui::anime::AnimeState;
use crate::ui::episodes::{EpisodesState, PlayPopup};
use crate::ui::home::HomeState;
use crate::ui::login::LoginState;
use crate::ui::popup::Popup;
use crate::ui::search::SearchState;
use crate::ui::widgets::{list_next, list_prev};
use crate::{api, player, theme, ui};

/// Mensajes de vuelta de las tareas async hacia el loop de la TUI.
enum AppEvent {
    LoginDone {
        restore: bool,
        remember: bool,
        result: Result<(Crunchyroll, SessionToken), String>,
    },
    SearchFire(u64, String),
    SearchDone(u64, Result<Vec<SearchSeries>, String>),
    PosterDone(u64, Result<image::DynamicImage, String>),
    SeasonsDone(Result<Box<(Series, Vec<Season>)>, String>),
    EpisodesDone(Result<Box<(Season, Vec<Episode>)>, String>),
    PlayDone(Result<PlayRequest, String>),
}

/// Pantalla activa.
enum Screen {
    Login(LoginState),
    Home(HomeState),
    Search(SearchState),
    Anime(AnimeState),
    Episodes(EpisodesState),
}

pub struct App {
    config: Config,
    crunchy: Option<Crunchyroll>,
    screen: Screen,
    popup: Option<Popup>,
    picker: Picker,
    tx: UnboundedSender<AppEvent>,
    rx: UnboundedReceiver<AppEvent>,
    /// Estado de búsqueda conservado al entrar en una serie (para Esc).
    saved_search: Option<SearchState>,
    saved_anime: Option<AnimeState>,
    quit: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = unbounded_channel();
        // Debe llamarse con la terminal ya en raw mode. Si la consulta
        // falla, se usa un tamaño de celda típico (renderiza con halfblocks).
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        Self {
            config,
            crunchy: None,
            screen: Screen::Login(LoginState::default()),
            popup: None,
            picker,
            tx,
            rx,
            saved_search: None,
            saved_anime: None,
            quit: false,
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), AppError> {
        self.try_restore_session();
        let mut events = EventStream::new();
        while !self.quit {
            terminal.draw(|f| self.draw(f))?;
            tokio::select! {
                maybe_event = events.next() => {
                    if let Some(Ok(event)) = maybe_event {
                        self.handle_terminal_event(event);
                    }
                }
                maybe_app = self.rx.recv() => {
                    if let Some(event) = maybe_app {
                        self.handle_app_event(event);
                    }
                }
            }
        }
        Ok(())
    }

    // ----- arranque -------------------------------------------------------

    fn try_restore_session(&mut self) {
        match self.config.session_token.clone() {
            Some(stored) => {
                self.popup = Some(Popup::Busy("Restaurando sesión…".into()));
                let tx = self.tx.clone();
                tokio::spawn(async move {
                    let result = api::login_saved(&stored).await;
                    let _ = tx.send(AppEvent::LoginDone {
                        restore: true,
                        remember: true,
                        result,
                    });
                });
            }
            None => {
                self.screen = Screen::Login(LoginState {
                    remember: self.config.remember_session,
                    ..LoginState::default()
                });
            }
        }
    }

    // ----- dibujo ---------------------------------------------------------

    fn draw(&mut self, f: &mut Frame) {
        f.render_widget(
            Block::default().style(ratatui::style::Style::default().bg(theme::BG)),
            f.area(),
        );
        match &mut self.screen {
            Screen::Login(s) => ui::login::draw(f, s),
            Screen::Home(s) => ui::home::draw(f, s),
            Screen::Search(s) => ui::search::draw(f, s),
            Screen::Anime(s) => ui::anime::draw(f, s),
            Screen::Episodes(s) => ui::episodes::draw(f, s),
        }
        if let Some(popup) = &self.popup {
            ui::popup::draw(f, popup);
        }
    }

    // ----- entrada de teclado ---------------------------------------------

    fn handle_terminal_event(&mut self, event: Event) {
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press {
            return;
        }
        if self.popup.is_some() {
            self.handle_popup_key(key);
            return;
        }
        match &mut self.screen {
            Screen::Login(_) => self.login_key(key),
            Screen::Home(_) => self.home_key(key),
            Screen::Search(_) => self.search_key(key),
            Screen::Anime(_) => self.anime_key(key),
            Screen::Episodes(_) => self.episodes_key(key),
        }
    }

    fn handle_popup_key(&mut self, key: KeyEvent) {
        match (&self.popup, key.code) {
            (Some(Popup::Busy(_)), _) => {} // bloqueante
            (Some(Popup::Help), KeyCode::Char('?'))
            | (Some(_), KeyCode::Esc)
            | (Some(_), KeyCode::Enter)
            | (Some(_), KeyCode::Char('q')) => self.popup = None,
            _ => {}
        }
    }

    fn login_key(&mut self, key: KeyEvent) {
        let Screen::Login(s) = &mut self.screen else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.quit = true,
            KeyCode::Tab | KeyCode::Down => s.field = (s.field + 1) % 3,
            KeyCode::BackTab | KeyCode::Up => s.field = (s.field + 2) % 3,
            KeyCode::Char(' ') if s.field == 2 => s.remember = !s.remember,
            KeyCode::Char(c) if s.field <= 1 => {
                if s.field == 0 {
                    s.email.push(c);
                } else {
                    s.password.push(c);
                }
            }
            KeyCode::Backspace if s.field <= 1 => {
                if s.field == 0 {
                    s.email.pop();
                } else {
                    s.password.pop();
                }
            }
            KeyCode::Enter => {
                if s.email.trim().is_empty() || s.password.is_empty() {
                    self.popup = Some(Popup::Error("Introduce correo y contraseña.".into()));
                    return;
                }
                let (email, password, remember) = (s.email.clone(), s.password.clone(), s.remember);
                self.popup = Some(Popup::Busy("Iniciando sesión…".into()));
                let tx = self.tx.clone();
                tokio::spawn(async move {
                    let result = api::login(&email, &password).await;
                    let _ = tx.send(AppEvent::LoginDone {
                        restore: false,
                        remember,
                        result,
                    });
                });
            }
            _ => {}
        }
    }

    fn home_key(&mut self, key: KeyEvent) {
        let Screen::Home(s) = &mut self.screen else {
            return;
        };
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Up => s.selected = s.selected.saturating_sub(1),
            KeyCode::Down => s.selected = (s.selected + 1).min(ui::home::ITEMS.len() - 1),
            KeyCode::Char('/') | KeyCode::Enter => {
                let idx = s.selected;
                if key.code == KeyCode::Enter && idx == 1 {
                    self.quit = true;
                } else {
                    self.screen = Screen::Search(self.saved_search.take().unwrap_or_default());
                }
            }
            _ => {}
        }
    }

    fn search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Home(HomeState::default()),
            KeyCode::Down | KeyCode::Up => {
                let Screen::Search(s) = &mut self.screen else {
                    return;
                };
                if key.code == KeyCode::Down {
                    list_next(&mut s.list, s.results.len());
                } else {
                    list_prev(&mut s.list);
                }
                self.trigger_poster();
            }
            KeyCode::Enter => {
                let Screen::Search(s) = &mut self.screen else {
                    return;
                };
                if let Some(result) = s.selected().cloned() {
                    self.popup = Some(Popup::Busy("Cargando serie…".into()));
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(AppEvent::SeasonsDone(
                            api::seasons(result).await.map(Box::new),
                        ));
                    });
                }
            }
            KeyCode::Backspace => {
                let Screen::Search(s) = &mut self.screen else {
                    return;
                };
                s.input.pop();
                self.trigger_search();
            }
            KeyCode::Char(c) => {
                let Screen::Search(s) = &mut self.screen else {
                    return;
                };
                s.input.push(c);
                self.trigger_search();
            }
            _ => {}
        }
    }

    fn anime_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                let Screen::Anime(_) = &self.screen else {
                    return;
                };
                let prev = std::mem::replace(&mut self.screen, Screen::Home(HomeState::default()));
                if let Screen::Anime(state) = prev {
                    self.saved_anime = Some(state);
                }
                self.screen = Screen::Search(self.saved_search.take().unwrap_or_default());
            }
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Down | KeyCode::Up => {
                let Screen::Anime(s) = &mut self.screen else {
                    return;
                };
                if key.code == KeyCode::Down {
                    list_next(&mut s.list, s.seasons.len());
                } else {
                    list_prev(&mut s.list);
                }
            }
            KeyCode::Enter => {
                let Screen::Anime(s) = &mut self.screen else {
                    return;
                };
                if let Some(season) = s.selected_season().cloned() {
                    self.popup = Some(Popup::Busy("Cargando episodios…".into()));
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(AppEvent::EpisodesDone(
                            api::episodes(season).await.map(Box::new),
                        ));
                    });
                }
            }
            _ => {}
        }
    }

    fn episodes_key(&mut self, key: KeyEvent) {
        let Screen::Episodes(s) = &mut self.screen else {
            return;
        };

        if let Some(play) = &mut s.play {
            match key.code {
                KeyCode::Esc => s.play = None,
                KeyCode::Tab | KeyCode::Right => play.focus = (play.focus + 1) % 3,
                KeyCode::BackTab | KeyCode::Left => play.focus = (play.focus + 2) % 3,
                KeyCode::Down => match play.focus {
                    0 => play.audio_sel = (play.audio_sel + 1).min(play.audios.len() - 1),
                    1 => play.sub_sel = (play.sub_sel + 1).min(play.subs.len() - 1),
                    _ => {
                        play.quality_sel =
                            (play.quality_sel + 1).min(ui::episodes::QUALITIES.len() - 1)
                    }
                },
                KeyCode::Up => match play.focus {
                    0 => play.audio_sel = play.audio_sel.saturating_sub(1),
                    1 => play.sub_sel = play.sub_sel.saturating_sub(1),
                    _ => play.quality_sel = play.quality_sel.saturating_sub(1),
                },
                KeyCode::Enter => {
                    let ep = s.episodes[play.episode_idx].clone();
                    let version = play.audios[play.audio_sel].1.clone();
                    let subtitle = play.subs[play.sub_sel].1.clone();
                    let quality = ui::episodes::QUALITIES[play.quality_sel].to_string();
                    self.popup = Some(Popup::Busy("Obteniendo stream…".into()));
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(AppEvent::PlayDone(
                            api::resolve_play(ep, version, subtitle, quality).await,
                        ));
                    });
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.screen = match self.saved_anime.take() {
                    Some(anime) => Screen::Anime(anime),
                    None => Screen::Search(self.saved_search.take().unwrap_or_default()),
                };
            }
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Down => list_next(&mut s.list, s.episodes.len()),
            KeyCode::Up => list_prev(&mut s.list),
            KeyCode::Enter => {
                if let Some(idx) = s.list.selected() {
                    let ep = &s.episodes[idx];
                    s.play = Some(PlayPopup::new(
                        idx,
                        ep,
                        &self.config.audio,
                        &self.config.subtitle,
                        &self.config.quality,
                    ));
                }
            }
            _ => {}
        }
    }

    // ----- disparadores async ----------------------------------------------

    /// Debounce de búsqueda: espera y dispara `SearchFire`; si el input
    /// cambió mientras tanto, el evento se descarta.
    fn trigger_search(&mut self) {
        let Screen::Search(s) = &mut self.screen else {
            return;
        };
        s.gen += 1;
        s.loading = true;
        let (tx, gen, query) = (self.tx.clone(), s.gen, s.input.clone());
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(350)).await;
            let _ = tx.send(AppEvent::SearchFire(gen, query));
        });
    }

    /// Carga la portada del resultado seleccionado.
    fn trigger_poster(&mut self) {
        let (Some(cr), Screen::Search(s)) = (&self.crunchy, &mut self.screen) else {
            return;
        };
        let Some(url) = s.selected_poster_url() else {
            return;
        };
        s.poster_gen += 1;
        let (tx, gen, cr) = (self.tx.clone(), s.poster_gen, cr.clone());
        tokio::spawn(async move {
            let _ = tx.send(AppEvent::PosterDone(gen, api::poster(&cr, url).await));
        });
    }

    // ----- eventos async ----------------------------------------------------

    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::LoginDone {
                restore,
                remember,
                result,
            } => self.on_login_done(restore, remember, result),
            AppEvent::SearchFire(gen, query) => {
                let Screen::Search(s) = &self.screen else {
                    return;
                };
                if gen != s.gen || query != s.input {
                    return; // obsoleto
                }
                if query.trim().is_empty() {
                    let Screen::Search(s) = &mut self.screen else {
                        return;
                    };
                    s.results.clear();
                    s.list.select(None);
                    s.poster = None;
                    s.loading = false;
                    return;
                }
                let Some(cr) = &self.crunchy else { return };
                let (tx, cr) = (self.tx.clone(), cr.clone());
                tokio::spawn(async move {
                    let _ = tx.send(AppEvent::SearchDone(gen, api::search(&cr, query).await));
                });
            }
            AppEvent::SearchDone(gen, result) => {
                let Screen::Search(s) = &mut self.screen else {
                    return;
                };
                if gen != s.gen {
                    return;
                }
                s.loading = false;
                match result {
                    Ok(results) => {
                        s.results = results;
                        s.list.select((!s.results.is_empty()).then_some(0));
                        self.trigger_poster();
                    }
                    Err(e) => self.popup = Some(Popup::Error(e)),
                }
            }
            AppEvent::PosterDone(gen, result) => {
                let Screen::Search(s) = &mut self.screen else {
                    return;
                };
                if gen != s.poster_gen {
                    return;
                }
                if let Ok(img) = result {
                    s.poster = Some(self.picker.new_resize_protocol(img));
                }
            }
            AppEvent::SeasonsDone(result) => {
                self.popup = None;
                match result {
                    Ok(payload) => {
                        let (series, seasons) = *payload;
                        let prev = std::mem::replace(
                            &mut self.screen,
                            Screen::Anime(AnimeState::new(series, seasons)),
                        );
                        if let Screen::Search(state) = prev {
                            self.saved_search = Some(state);
                        }
                    }
                    Err(e) => self.popup = Some(Popup::Error(e)),
                }
            }
            AppEvent::EpisodesDone(result) => {
                self.popup = None;
                match result {
                    Ok(payload) => {
                        let (season, episodes) = *payload;
                        let prev = std::mem::replace(
                            &mut self.screen,
                            Screen::Episodes(EpisodesState::new(season, episodes)),
                        );
                        if let Screen::Anime(state) = prev {
                            self.saved_anime = Some(state);
                        }
                    }
                    Err(e) => self.popup = Some(Popup::Error(e)),
                }
            }
            AppEvent::PlayDone(result) => {
                self.popup = None;
                match result {
                    Ok(req) => match player::launch(&req) {
                        Ok(()) => {
                            if let Screen::Episodes(s) = &mut self.screen {
                                s.play = None;
                            }
                            self.popup =
                                Some(Popup::Info(format!("Reproduciendo en mpv: {}", req.title)));
                        }
                        Err(e) => self.popup = Some(Popup::Error(e.to_string())),
                    },
                    Err(e) => self.popup = Some(Popup::Error(e)),
                }
            }
        }
    }

    fn on_login_done(
        &mut self,
        restore: bool,
        remember: bool,
        result: Result<(Crunchyroll, SessionToken), String>,
    ) {
        self.popup = None;
        match result {
            Ok((cr, token)) => {
                self.crunchy = Some(cr);
                self.config.remember_session = remember;
                self.config.session_token = if remember {
                    api::encode_token(&token)
                } else {
                    None
                };
                if let Err(e) = self.config.save() {
                    tracing::warn!("no se pudo guardar la config: {e}");
                }
                self.screen = Screen::Home(HomeState::default());
            }
            Err(e) => {
                tracing::warn!("login falló: {e}");
                if restore {
                    // Token caducado: se descarta y se vuelve al login.
                    self.config.session_token = None;
                    let _ = self.config.save();
                    self.screen = Screen::Login(LoginState {
                        remember: self.config.remember_session,
                        ..LoginState::default()
                    });
                }
                self.popup = Some(Popup::Error(format!("No fue posible iniciar sesión:\n{e}")));
            }
        }
    }
}
