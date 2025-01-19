use std::{io::Read, str::Bytes};

use super::Component;
use crate::{action, config::Config};
// use bytes;
use color_eyre::{eyre::eyre, eyre::Ok, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tracing::debug;
use tracing::error;

#[derive(Default)]
pub struct Gdbtty {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    gdb_writer: Option<Box<dyn std::io::Write + Send>>,
    gdb_reader: Option<Box<dyn std::io::Read + Send>>,
    gdb_process: Option<Box<dyn Child + Send + Sync>>,
    gdb_read_task: Option<JoinHandle<()>>,
}

impl Gdbtty {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Out(Vec<u8>),
}

impl Gdbtty {
    async fn gdbtty_reader(
        mut reader: Box<dyn std::io::Read + Send>,
        send: UnboundedSender<action::Action>,
    ) {
        let mut buf = [0_u8; 32];
        loop {
            // debug!("read start!");
            let n = reader.read(&mut buf).map_or(0, |n| n);

            let action = match n {
                0 => None,
                _ => {
                    let out = buf[0..n].into_iter().map(|c| *c).collect::<Vec<_>>();
                    Some(Action::Out(out))
                }
            };
            if let Some(action) = action {
                // debug!("read finish!");
                if send.send(action::Action::GdbRead(action)).is_err() {
                    error!("gdb tty read but send fail! {:?}", &buf[0..buf.len()]);
                };
            };
        }
    }

    fn handle_pane_key_event(key: &crossterm::event::KeyEvent) -> Option<Vec<u8>> {
        let input_bytes = match key.code {
            crossterm::event::KeyCode::Char(ch) => {
                let mut send = vec![ch as u8];
                let upper = ch.to_ascii_uppercase();
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL {
                    match upper {
                        // https://github.com/fyne-io/terminal/blob/master/input.go
                        // https://gist.github.com/ConnerWill/d4b6c776b509add763e17f9f113fd25b
                        '2' | '@' | ' ' => send = vec![0],
                        '3' | '[' => send = vec![27],
                        '4' | '\\' => send = vec![28],
                        '5' | ']' => send = vec![29],
                        '6' | '^' => send = vec![30],
                        '7' | '-' | '_' => send = vec![31],
                        char if ('A'..='_').contains(&char) => {
                            // Since A == 65,
                            // we can safely subtract 64 to get
                            // the corresponding control character
                            let ascii_val = char as u8;
                            let ascii_to_send = ascii_val - 64;
                            send = vec![ascii_to_send];
                        }
                        _ => {}
                    }
                }
                send
            }
            #[cfg(unix)]
            crossterm::event::KeyCode::Enter => vec![b'\n'],
            #[cfg(windows)]
            crossterm::event::KeyCode::Enter => vec![b'\r', b'\n'],
            crossterm::event::KeyCode::Backspace => vec![8],
            crossterm::event::KeyCode::Left => vec![27, 91, 68],
            crossterm::event::KeyCode::Right => vec![27, 91, 67],
            crossterm::event::KeyCode::Up => vec![27, 91, 65],
            crossterm::event::KeyCode::Down => vec![27, 91, 66],
            crossterm::event::KeyCode::Tab => vec![9],
            crossterm::event::KeyCode::Home => vec![27, 91, 72],
            crossterm::event::KeyCode::End => vec![27, 91, 70],
            crossterm::event::KeyCode::PageUp => vec![27, 91, 53, 126],
            crossterm::event::KeyCode::PageDown => vec![27, 91, 54, 126],
            crossterm::event::KeyCode::BackTab => vec![27, 91, 90],
            crossterm::event::KeyCode::Delete => vec![27, 91, 51, 126],
            crossterm::event::KeyCode::Insert => vec![27, 91, 50, 126],
            crossterm::event::KeyCode::Esc => vec![27],
            _ => return None,
        };
        Some(input_bytes)
    }
}

impl Component for Gdbtty {
    fn register_action_handler(&mut self, tx: UnboundedSender<action::Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn draw(&mut self, _frame: &mut Frame, _area: Rect) -> Result<()> {
        Ok(())
    }
    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<Option<action::Action>> {
        if let Some(bytes) = Gdbtty::handle_pane_key_event(&key) {
            let bytes = bytes.into_iter().map(|c| char::from(c)).collect::<String>();
            if let Some(write) = self.gdb_writer.as_mut() {
                write!(write, "{}", bytes.as_str())?;
            }
        };
        Ok(None)
    }

    fn update(&mut self, _action: action::Action) -> Result<Option<action::Action>> {
        if let Some(t) = &self.gdb_read_task {
            if t.is_finished() {
                error!("gdb task finish!");
            };
        }
        Ok(None)
    }
    fn init(&mut self, _area: Size) -> Result<()> {
        // Use the native pty implementation for the system
        let pty_system = native_pty_system();

        // Create a new pty
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                // Not all systems support pixel_width, pixel_height,
                // but it is good practice to set it to something
                // that matches the size of the selected font.  That
                // is more complex than can be shown here in this
                // brief example though!
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        

        // Spawn a shell into the pty
        let cmd = CommandBuilder::new("gdb");
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        self.gdb_process = Some(child);

        // Read and parse output from the pty with reader
        let gdb_reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        self.gdb_reader = Some(gdb_reader);

        // Send data to the pty by writing to the master
        let gdb_writer = pair
            .master
            .take_writer()
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        self.gdb_writer = Some(gdb_writer);

        let gdb_reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| eyre!(format!("{:?}", e)))?;

        if let Some(send) = self.command_tx.clone() {
            let reader_task = Self::gdbtty_reader(gdb_reader, send.clone());
            self.gdb_read_task = Some(tokio::spawn(async {
                reader_task.await;
            }));
        } else {
            let msg = "gdb reader thread not start";
            error!("{}", &msg);
            return Err(eyre!(msg));
        }
        Ok(())
    }
}
