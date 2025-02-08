use super::{gdbtty, Component};
use crate::mi::breakpointmi::{show_bkpt, show_breakpoint_deleted, BreakPointAction};
use crate::mi::disassemble::DisassembleFunction;
use crate::mi::token::*;
use crate::mi::{disassemble, miout};
use crate::tool;
use crate::{action, config::Config};
use color_eyre::{eyre::eyre, eyre::Ok, Result};
use portable_pty::{native_pty_system, PtySize};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use smol::io::AsyncReadExt;
use strum::Display;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tracing::error;
use tracing::{debug, info};

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
    ShowFile((String, u64)),
    ShowAsm((String, String)),
    ReadAsmFunc(DisassembleFunction),
    Breakpoint(BreakPointAction),
    BreakpointDeleted(u64),
}

impl Gdbmi {
    pub fn new() -> Self {
        Self::default()
    }

    async fn gdb_mi_reader(
        reader: Box<dyn std::io::Read + Send>,
        send: UnboundedSender<action::Action>,
    ) {
        // lazy_static! {
        //     static ref LINE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
        // };
        let mut buf = [0_u8; 32];
        let mut line = String::new();
        let mut reader = smol::io::BufReader::new(smol::Unblock::new(reader));

        loop {
            // debug!("read start!");
            // let n = reader.read(&mut buf).await.map_or(0, |n| n);
            let n = reader.read(&mut buf).await.map_or(0, |n| n);

            let mut out_line = vec![];
            match n {
                0 => {}
                _ => {
                    buf[0..n]
                        .iter()
                        .map(|c| char::from(*c))
                        .filter(|c| *c != '\r')
                        .for_each(|f| match f {
                            '\n' => {
                                line.push(f);
                                out_line.push(line.clone());
                                line.clear()
                            }
                            _ => line.push(f),
                        });
                }
            };
            let mut actions = vec![];
            out_line.into_iter().for_each(|line| {
                match miout::TokOutputOnelineParser::new().parse(line.as_str()) {
                    std::result::Result::Ok(OutputOneline::OutOfBandRecord(a)) => {
                        if let Some(show) = show_file(&a) {
                            actions.push(Action::ShowFile(show));
                        } else if let Some(show) = show_asm(&a) {
                            actions.push(Action::ShowAsm(show));
                        }
                        if let Some(bkpt) = show_bkpt(&a) {
                            actions.push(Action::Breakpoint(bkpt));
                        }
                        if let Some(id) = show_breakpoint_deleted(&a) {
                            actions.push(Action::BreakpointDeleted(id));
                        }
                    }
                    std::result::Result::Ok(OutputOneline::ResultRecord(a)) => {
                        if let Some(asmfunc) = disassemble::get_disassemble_function(a) {
                            actions.push(Action::ReadAsmFunc(asmfunc));
                        }
                    }
                    std::result::Result::Err(e) => {
                        error!("unknow read gdb mi line {} {:?} ", &e, &line);
                    }
                }
                // actions.push(Action::Out(line));
                info!("gdb mi read {:?}", &line);
            });
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
                Ok(pty_name.to_string())
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
        match action {
            action::Action::Gdbmi(Action::Start) => {
                let path = self.start_gdb_mi()?;
                Ok(Some(action::Action::Gdbtty(gdbtty::Action::Start(path))))
            }
            action::Action::Gdbmi(Action::ShowAsm((func, _))) => {
                if let Some(write) = self.gdb_mi_writer.as_mut() {
                    write!(write, "-data-disassemble -a {} -- 1\n", func)?;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }
}

fn show_asm(a: &OutOfBandRecordType) -> Option<(String, String)> {
    let get_from_frame = |r: &ResultType| -> Option<(String, String)> {
        let mut addr = None;
        let mut func = None;
        if r.variable.as_str() == "frame" {
            if let ValueType::Tuple(Tuple::Results(rs)) = &r.value {
                rs.iter().for_each(|r| match r.variable.as_str() {
                    "addr" => {
                        if let ValueType::Const(f) = &r.value {
                            addr = Some(f.clone())
                        }
                    }
                    "func" => {
                        if let ValueType::Const(f) = &r.value {
                            func = Some(f.clone())
                        }
                    }
                    _ => {}
                });
            }
        }
        match (func, addr) {
            (Some(func), Some(addr)) => Some((func, addr)),
            _ => None,
        }
    };
    let mut ret = None;
    let OutOfBandRecordType::AsyncRecord(a) = a;
    match a {
        AsyncRecordType::ExecAsyncOutput(a) => {
            if a.async_output.async_class == AsyncClassType::Stopped {
                a.async_output.resaults.iter().for_each(|r| {
                    if let Some(a) = get_from_frame(r) {
                        ret = Some(a);
                    }
                });
            }
        }
        _ => {}
    }
    ret
}

fn show_file(a: &OutOfBandRecordType) -> Option<(String, u64)> {
    let get_from_frame = |r: &ResultType| -> Option<(String, u64)> {
        let mut file = "".to_string();
        let mut line = 0_u64;
        if r.variable.as_str() == "frame" {
            if let ValueType::Tuple(Tuple::Results(rs)) = &r.value {
                rs.iter().for_each(|r| match r.variable.as_str() {
                    "fullname" => {
                        if let ValueType::Const(f) = &r.value {
                            file = f.clone()
                        }
                    }
                    "line" => {
                        if let ValueType::Const(l) = &r.value {
                            if let std::result::Result::Ok(l) = l.parse::<u64>() {
                                line = l
                            }
                        }
                    }
                    _ => {}
                });
            }
        }
        if !file.is_empty() && line != 0 {
            Some((file, line))
        } else {
            None
        }
    };
    let mut ret = None;
    let OutOfBandRecordType::AsyncRecord(a) = a;
    match a {
        AsyncRecordType::ExecAsyncOutput(a) => {
            if a.async_output.async_class == AsyncClassType::Stopped {
                a.async_output.resaults.iter().for_each(|r| {
                    if let Some(a) = get_from_frame(r) {
                        ret = Some(a);
                    }
                });
            }
        }
        AsyncRecordType::NotifyAsyncOutput(a) => {
            if a.async_output.async_class == AsyncClassType::ThreadSelected {
                a.async_output.resaults.iter().for_each(|r| {
                    if let Some(a) = get_from_frame(r) {
                        ret = Some(a);
                    }
                });
            }
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use crate::components::gdbmi::show_asm;
    use crate::components::gdbmi::show_file;
    use crate::mi::miout;
    #[test]
    fn f_show_file() {
        let a = miout::TokOutOfBandRecordParser::new()
            .parse(r##"*stopped,reason="end-stepping-range",frame={addr="0x00000000004006ff",func="main",args=[],file="a.c",fullname="/home/shizhilvren/c++/a.c",line="27"},thread-id="1",stopped-threads="all",core="6"
"##);
        let b = show_file(a.as_ref().unwrap());
        println!("{:?} {:?}", &a, &b);
        assert!(b == Some(("/home/shizhilvren/c++/a.c".to_string(), 27_u64)));
    }

    #[test]
    fn f_show_file_2() {
        let a = miout::TokOutOfBandRecordParser::new()
            .parse("=thread-selected,id=\"1\",frame={level=\"1\",addr=\"0x000000000020198c\",func=\"main\",args=[],file=\"args.c\",fullname=\"/remote/x/x/code/c++/args.c\",line=\"7\",arch=\"i386:x86-64\"}\n");
        let b = show_file(a.as_ref().unwrap());
        println!("{:?} {:?}", &a, &b);
        assert!(b == Some(("/remote/x/x/code/c++/args.c".to_string(), 7_u64)));
    }

    #[test]
    fn f_show_asm() {
        let a = miout::TokOutOfBandRecordParser::new()
            .parse("*stopped,reason=\"breakpoint-hit\",disp=\"del\",bkptno=\"1\",frame={addr=\"0x0000555555581c20\",func=\"main\",args=[],arch=\"i386:x86-64\"},thread-id=\"1\",stopped-threads=\"all\",core=\"5\"\n");
        let b = show_asm(a.as_ref().unwrap());
        println!("{:?} {:?}", &a, &b);
        assert!(b == Some(("main".to_string(), "0x0000555555581c20".to_string())));
    }
}
