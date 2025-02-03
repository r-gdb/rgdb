use crate::components::gdbmi::Action as GdbmiAction;
use crate::components::gdbtty::Action as GdbttyAction;
use crate::{
    action,
    components::{
        code::Code, fps::FpsCounter, gdbmi::Gdbmi, gdbtty::Gdbtty, home::Home,
        startpage::StartPage, statusbar::StatusBar, Component,
    },
    config::Config,
    tui::{Event, Tui},
};
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<action::Action>,
    action_rx: mpsc::UnboundedReceiver<action::Action>,
    gdb_path: String,
    gdb_args: Vec<String>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Gdb,
    Code,
}

impl App {
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        gdb_path: String,
        gdb_args: Vec<String>,
    ) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(Home::new()),
                Box::new(Code::new()),
                Box::new(FpsCounter::new()),
                Box::new(Gdbmi::new()),
                Box::new(Gdbtty::new()),
                Box::new(StartPage::new()),
                Box::new(StatusBar::new()),
            ],
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Gdb,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            gdb_path,
            gdb_args,
        })
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        self.init()?;

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(action::Action::Resume)?;
                action_tx.send(action::Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(action::Action::Quit)?,
            Event::Tick => action_tx.send(action::Action::Tick)?,
            Event::Render => action_tx.send(action::Action::Render)?,
            Event::Resize(x, y) => action_tx.send(action::Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != action::Action::Tick && action != action::Action::Render {
                debug!("{action:?}");
            }
            match action {
                action::Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                action::Action::Quit => self.should_quit = true,
                action::Action::Suspend => self.should_suspend = true,
                action::Action::Resume => self.should_suspend = false,
                action::Action::ClearScreen => tui.terminal.clear()?,
                action::Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                action::Action::Render => self.render(tui)?,
                action::Action::Mode(mode) => self.set_mode(mode),
                _ => {}
            }
            for component in self.components.iter_mut() {
                if let Some(action) = component.update(action.clone())? {
                    self.action_tx.send(action)?
                };
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(action::Action::Error(format!("Failed to draw: {:?}", err)));
                }
            }
        })?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        let s = self.action_tx.clone();
        s.send(action::Action::Gdbtty(GdbttyAction::SetGdb(
            self.gdb_path.clone(),
        )))?;
        s.send(action::Action::Gdbtty(GdbttyAction::SetGdbArgs(
            self.gdb_args.clone(),
        )))?;
        s.send(action::Action::Gdbmi(GdbmiAction::Start))?;
        Ok(())
    }
}
