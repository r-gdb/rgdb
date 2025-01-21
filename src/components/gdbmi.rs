use super::{gdbtty, Component};
use crate::tool;
use crate::tool::lalrpop_mod_doc;
use crate::{action, config::Config};
// use bytes;
use color_eyre::{eyre::eyre, eyre::Ok, Result};
use lalrpop_util::lalrpop_mod;
use lazy_static::lazy_static;
use portable_pty::{native_pty_system, PtySize};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tracing::error;
use tracing::{debug, info};
lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::vec_box)]
    miout,
    "/mi/miout.rs"
);

#[derive(Default)]
pub struct Gdbmi {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    gdb_mi_writer: Option<Box<dyn std::io::Write + Send>>,
    gdb_mi_reader: Option<Box<dyn std::io::Read + Send>>,
    gdb_mi_read_task: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Start,
    Out(String),
}

impl Gdbmi {
    pub fn new() -> Self {
        Self::default()
    }

    async fn gdb_mi_reader(
        mut reader: Box<dyn std::io::Read + Send>,
        send: UnboundedSender<action::Action>,
    ) {
        lazy_static! {
            static ref LINE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
        };
        let mut buf = [0_u8; 32];
        let mut line = LINE.lock().unwrap();

        loop {
            // debug!("read start!");
            let n = reader.read(&mut buf).map_or(0, |n| n);

            let mut actions = vec![];
            match n {
                0 => {}
                _ => {
                    buf[0..n]
                        .into_iter()
                        .map(|c| char::from(*c))
                        .filter(|c| *c != '\r')
                        .for_each(|f| match f {
                            '\n' => {
                                line.push(f);
                                actions.push(Action::Out(line.clone()));
                                line.clear()
                            }
                            _ => line.push(f),
                        });
                }
            };
            actions.into_iter().for_each(|action| {
                if send.send(action::Action::Gdbmi(action)).is_err() {
                    error!("gdb tty read but send fail! {:?}", &buf[0..buf.len()]);
                };
            });
        }
    }

    fn start_gdb_mi(&mut self) -> Result<String> {
        let pty_system = native_pty_system();
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

        // Read and parse output from the pty with reader
        let gdb_im_reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        self.gdb_mi_reader = Some(gdb_im_reader);

        // Send data to the pty by writing to the master
        let gdb_mi_writer = pair
            .master
            .take_writer()
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        self.gdb_mi_writer = Some(gdb_mi_writer);

        let gdb_mi_reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| eyre!(format!("{:?}", e)))?;

        if let Some(send) = self.command_tx.clone() {
            let reader_task = Self::gdb_mi_reader(gdb_mi_reader, send.clone());
            self.gdb_mi_read_task = Some(tokio::spawn(async {
                reader_task.await;
            }));
            debug!("gdb mi start")
        } else {
            let msg = "gdb mi reader thread not start";
            error!("{}", &msg);
            return Err(eyre!(msg));
        }

        let ret = match pair.master.as_raw_fd() {
            Some(fd) => {
                let pty_name = tool::get_pty_name(fd)?;
                info!("gdb mi start at {}", &pty_name);
                Ok(format!("{}", pty_name))
            }
            _ => Err(eyre!("gdb mi pty start fail!")),
        };
        debug!("gdb mi start return {:?} ", &ret);
        // loop {}
        ret
    }
}

impl Component for Gdbmi {
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
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        let ret = match action {
            action::Action::Gdbmi(Action::Start) => {
                let path = self.start_gdb_mi()?;
                Ok(Some(action::Action::Gdbtty(gdbtty::Action::Start(path))))
            }
            _ => Ok(None),
        };
        ret
    }
    fn init(&mut self, _area: Size) -> Result<()> {
        match self.command_tx.clone() {
            Some(s) => Ok(s.send(action::Action::Gdbmi(Action::Start))?),
            _ => Err(eyre!("gdm mi init fail!")),
        }
    }
}

