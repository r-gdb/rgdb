use std::io::Write;

use arboard::Clipboard;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use color_eyre::{eyre, Result};
use ratatui::{layout::Rect, Frame};
use tracing::debug;

use super::Component;
use crate::action::Action;

pub struct CopyString {
    board: Option<Clipboard>,
}

impl CopyString {
    pub fn new() -> Self {
        CopyString {
            board: Clipboard::new().ok(),
        }
    }
    fn send_to_clipboard(&mut self, s: &String) -> Result<()> {
        match &mut self.board {
            Some(clipboard) => match clipboard.set_text(s.clone()) {
                Ok(_) => {
                    debug!("Clipboard set to: {:?}", s);
                }
                Err(e) => {
                    return Err(eyre::eyre!("Failed to set clipboard: {}", e));
                }
            },
            None => {
                return Err(eyre::eyre!("Clipboard not available"));
            }
        }
        Ok(())
    }
    fn send_to_ssh_clipboard(&self, s: &String) -> Result<()> {
        // 检查是否在SSH会话中
        if std::env::var("SSH_TTY").is_err() && std::env::var("SSH_CLIENT").is_err() {
            return Err(eyre::eyre!("Not in SSH session"));
        }
        // Base64编码内容
        let encoded = STANDARD.encode(s);
        debug!("Base64 encoded string: {:?} {:?}", &encoded, &s);

        // 构建OSC 52序列
        let osc52 = format!("\x1B]52;c;{}\x07", encoded);
        debug!("OSC 52 sequence: {:?}", &osc52);

        // 写入到标准输出
        if let Err(e) = std::io::stdout().write_all(osc52.as_bytes()) {
            return Err(eyre::eyre!("Failed to send to SSH clipboard: {}", e));
        }

        // 确保立即刷新输出
        if let Err(e) = std::io::stdout().flush() {
            return Err(eyre::eyre!("Failed to flush output: {}", e));
        }

        Ok(())
    }
}

impl Component for CopyString {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::CopyStr(s) => {
                // 尝试写入本地剪贴板
                let local_result = self.send_to_clipboard(&s);

                // 尝试写入SSH剪贴板
                let ssh_result = self.send_to_ssh_clipboard(&s);

                // 如果两者都失败，返回错误
                if local_result.is_err() && ssh_result.is_err() {
                    return Err(eyre::eyre!(
                        "Failed to copy: local error: {:?}, ssh error: {:?}",
                        local_result.err().unwrap(),
                        ssh_result.err().unwrap()
                    ));
                }
            }
            _ => {}
        };
        Ok(None)
    }

    fn draw(&mut self, _frame: &mut Frame, _area: Rect) -> Result<()> {
        Ok(())
    }
}
