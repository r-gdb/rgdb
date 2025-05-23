use super::Component;
use crate::tool;
use crate::{action, config::Config};
use color_eyre::{eyre::eyre, eyre::Ok, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use smol::io::AsyncReadExt;
use std::str::FromStr;
use strum::Display;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tracing::error;
use tracing::{debug, info};

#[derive(Default)]
pub struct Gdbtty {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    gdb_writer: Option<Box<dyn std::io::Write + Send>>,
    gdb_reader: Option<Box<dyn std::io::Read + Send>>,
    gdb_process: Option<Box<dyn Child + Send + Sync>>,
    gdb_read_task: Option<JoinHandle<()>>,
    pty_pair: Option<PtyPair>,
    gdb_path: String,
    gdb_args: Vec<String>,
    handle_key: bool,
    is_horizontal: bool,
}

impl Gdbtty {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Out(Vec<u8>),
    Start(String),
    SetGdb(String),
    SetGdbArgs(Vec<String>),
    GdbExit,
}

impl Gdbtty {
    fn is_gdb_exit(&mut self) -> bool {
        let ans = self
            .gdb_process
            .as_mut()
            .is_some_and(|p| matches!(p.try_wait(), std::result::Result::Ok(Some(_))));
        ans
    }
    fn set_gdb_path(&mut self, path: String) {
        self.gdb_path = path;
    }
    fn set_gdb_args(&mut self, args: Vec<String>) {
        self.gdb_args = args;
    }
    async fn gdbtty_reader(
        reader: Box<dyn std::io::Read + Send>,
        send: UnboundedSender<action::Action>,
    ) {
        let mut buf = [0_u8; 32];
        let mut reader = smol::io::BufReader::new(smol::Unblock::new(reader));
        loop {
            // debug!("read start!");
            let n = reader.read(&mut buf).await.map_or(0, |n| n);

            let action = match n {
                0 => None,
                _ => {
                    let out = buf[0..n].to_vec();
                    Some(Action::Out(out))
                }
            };
            if let Some(action) = action {
                // debug!("read finish!");
                if send.send(action::Action::Gdbtty(action)).is_err() {
                    error!("gdb tty read but send fail! {:?}", &buf[0..buf.len()]);
                };
            };
        }
    }
    fn gdbtty_start(&mut self, pts_path: String) -> Result<()> {
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
        let s = format!("new-ui mi3 {}", pts_path.as_str());
        let gdb_args = self.gdb_args.iter().map(|s| s.as_str());
        let args = [self.gdb_path.as_str(), "--nw", "--ex", s.as_str()]
            .iter()
            .copied()
            .chain(gdb_args)
            .map(|s| -> Result<std::ffi::OsString> { Ok(std::ffi::OsString::from_str(s)?) })
            .collect::<Result<Vec<_>>>()?;
        let cwd = std::env::current_dir()?;
        debug!("gdb tty start with {:?} currect dir is {:?}", &args, &cwd);
        // Spawn a shell into the pty
        let mut cmd = CommandBuilder::from_argv(args);
        cmd.cwd(cwd);
        debug!("gdb tty start cwd id {:?}", &cmd.get_cwd());
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

        match pair.master.as_raw_fd() {
            Some(fd) => {
                let pty_name = tool::get_pty_name(fd)?;
                info!("gdb tty start at {}", &pty_name);
                Ok(pty_name.to_string())
            }
            _ => Err(eyre!("gdb mi pty start fail!")),
        }?;

        let gdb_reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| eyre!(format!("{:?}", e)))?;

        let ret = match self.command_tx.clone() {
            Some(send) => {
                let reader_task = Self::gdbtty_reader(gdb_reader, send.clone());
                self.gdb_read_task = Some(tokio::spawn(async {
                    reader_task.await;
                }));
                debug!("gdb start");
                Ok(())
            }
            _ => {
                let msg = "gdb reader thread not start";
                error!("{}", &msg);
                Err(eyre!(msg))
            }
        };
        self.pty_pair = Some(pair);
        ret
    }

    fn handle_pane_key_event(key: &crossterm::event::KeyEvent) -> Option<Vec<u8>> {
        debug!("gdb tty key event {:?}", &key);
        let input_bytes = match key.code {
            crossterm::event::KeyCode::Char(ch) => {
                let mut buf = [0_u8; 4];
                let char = ch.encode_utf8(&mut buf);
                let mut send = char.as_bytes().to_vec();
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
                debug!("gdb tty key event send {:?}", &send);
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
    fn handle_key(&self) -> bool {
        self.handle_key
    }
    fn set_handle_key(&mut self, val: bool) {
        self.handle_key = val;
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

    fn draw(&mut self, _frame: &mut Frame, area: Rect) -> Result<()> {
        let tool::Layouts { gdb: area, .. } = (area, self.is_horizontal).into();
        let area = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        if let Some(pty_pair) = &self.pty_pair {
            pty_pair
                .master
                .resize(PtySize {
                    rows: area.height,
                    cols: area.width,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| eyre!("resize gdb tty fail {:?}", e))?
        }
        Ok(())
    }
    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<Option<action::Action>> {
        if self.handle_key() && key.code != crossterm::event::KeyCode::Esc {
            if let Some(bytes) = Gdbtty::handle_pane_key_event(&key) {
                // let bytes_utf8 = String::from_utf8_lossy(&bytes);
                debug!("gdb tty send {:?}", &bytes);
                // debug!("gdb tty send {:?}", &bytes_utf8);
                if let Some(write) = self.gdb_writer.as_mut() {
                    // write.write(&bytes);
                    write.write_all(&bytes)?;
                    // write!(write, b"{}", bytes)?;
                }
            }
        }

        Ok(None)
    }

    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        let ans = match action {
            action::Action::Gdbtty(Action::SetGdb(path)) => {
                self.set_gdb_path(path);
                None
            }
            action::Action::Gdbtty(Action::SetGdbArgs(args)) => {
                self.set_gdb_args(args);
                None
            }
            action::Action::Gdbtty(Action::Start(pts_path)) => {
                self.gdbtty_start(pts_path)?;
                Some(action::Action::Mode(crate::app::Mode::Gdb))
            }
            action::Action::Tick => match self.is_gdb_exit() {
                true => Some(action::Action::Gdbtty(Action::GdbExit)),
                false => None,
            },
            action::Action::Gdbtty(Action::GdbExit) => Some(action::Action::Quit),
            action::Action::Mode(mode) => {
                match mode {
                    crate::app::Mode::Gdb => self.set_handle_key(true),
                    _ => self.set_handle_key(false),
                };
                None
            }
            action::Action::SwapHV => {
                self.is_horizontal = !self.is_horizontal;
                None
            }
            _ => None,
        };
        Ok(ans)
    }
    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }
}
