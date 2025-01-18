use super::Component;
use crate::{action, config::Config};
use color_eyre::{eyre::eyre, eyre::Ok, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
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
    None,
    Oldline(Vec<u8>),
    Newline(Vec<u8>),
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
            let actions =
                match n {
                    0 => vec![Action::None],
                    _ => {
                        let bufs = buf[0..n].split(|c| char::from(*c) == '\n').enumerate().map(
                            |(id, buf)| {
                                let buf: Vec<u8> = buf
                                    .iter()
                                    .filter(|c| char::from(**c) != '\r')
                                    .copied()
                                    .collect();

                                if id == 0 {
                                    Action::Newline(buf)
                                } else {
                                    Action::Oldline(buf)
                                }
                            },
                        );
                        bufs.collect()
                    }
                };
            actions.into_iter().for_each(|action| {
                // debug!("read finish!");
                if send.send(action::Action::GdbRead(action)).is_err() {
                    error!("gdb tty read but send fail! {:?}", &buf[0..10]);
                };
            });
        }
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
