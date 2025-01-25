use super::Component;
use crate::components::gdbmi;
use crate::tool;
use crate::{action, config::Config};
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;
use std::usize;
use strum::Display;
use symbols::scrollbar;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

#[derive(Default)]
pub struct Code {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    files_set: HashSet<SrcFileData>,
    file_need_show: Option<(String, u64)>,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    FileReadOutLine((String, String)),
    FileReadEnd(String),
}

#[derive(Clone, Eq, Debug)]
pub struct SrcFileData {
    pub file_name: String,
    lines: Vec<String>,
    read_done: bool,
}

impl PartialEq for SrcFileData {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
    fn ne(&self, other: &Self) -> bool {
        self.file_name != other.file_name
    }
}

impl SrcFileData {
    pub fn new(file_name: String) -> Self {
        Self {
            file_name,
            lines: vec![],
            read_done: false,
        }
    }
    pub fn add_line(&mut self, line: String) {
        self.lines.push(line);
    }
    pub fn get_read_done(&self) -> bool {
        self.read_done
    }
    pub fn set_read_done(&mut self) {
        self.read_done = true;
    }
    pub fn get_lines(&self) -> &Vec<String> {
        &self.lines
    }
    pub fn get_lines_len(&self) -> usize {
        self.lines.len()
    }
    pub fn get_lines_range(&self, start: usize, end: usize) -> (Vec<&String>, usize, usize) {
        let n = self.lines.len().saturating_add(1);
        let end = n.min(end);
        (
            self.lines
                .iter()
                .skip(start.saturating_sub(1))
                .take(end.saturating_sub(start))
                .map(|s| s)
                .collect(),
            start,
            end,
        )
    }
}

impl Hash for SrcFileData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file_name.hash(state);
    }
}

impl Code {
    pub fn new() -> Self {
        Self::default()
    }
    async fn read_file(file: String, send: UnboundedSender<action::Action>) {
        match File::open(&file).await {
            std::result::Result::Ok(f) => {
                let mut f = tokio::io::BufReader::new(f);
                loop {
                    let mut line = String::new();
                    match f.read_line(&mut line).await {
                        std::result::Result::Ok(0) => {
                            match send.send(action::Action::Code(Action::FileReadEnd(file))) {
                                std::result::Result::Ok(_) => {}
                                std::result::Result::Err(e) => {
                                    error!("send error: {:?}", e);
                                }
                            }
                            break;
                        }
                        std::result::Result::Ok(_n) => {
                            line = line.replace("\t", "    ");
                            match send.send(action::Action::Code(Action::FileReadOutLine((
                                file.clone(),
                                line,
                            )))) {
                                std::result::Result::Ok(_) => {}
                                std::result::Result::Err(e) => {
                                    error!("send error: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("file {} parse error: {:?}", &file, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("open file {} error: {:?}", &file, e);
            }
        }
    }
    fn get_need_show_file(&self) -> Option<(&SrcFileData, u64)> {
        match self.file_need_show {
            Some((ref file, line_id)) => {
                match self.files_set.get(&SrcFileData::new(file.clone())) {
                    Some(file_data) => {
                        if file_data.get_read_done() {
                            Some((file_data, line_id))
                        } else {
                            info!("file {} not read done", &file);
                            None
                        }
                    }
                    _ => {
                        error!("file {} not found", &file);
                        None
                    }
                }
            }
            _ => None,
        }
    }
    fn file_down(&mut self, n: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(n);
    }
    fn file_up(&mut self, n: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(n);
    }
    fn legalization_scroll_range(&mut self, hight: usize, n: usize) -> (usize, usize) {
        let up_half = hight.div_euclid(2);
        let down_half = hight.div_ceil(2);
        let start = up_half.saturating_add(1);
        let end = n.saturating_sub(down_half).saturating_add(1).max(start);
        self.vertical_scroll = self.vertical_scroll.max(start);
        self.vertical_scroll = self.vertical_scroll.min(end);
        (start, end)
    }
    fn get_windows_show_file_range(&self, hight: usize) -> (usize, usize) {
        let up_half = hight.div_euclid(2);
        let start = self.vertical_scroll.saturating_sub(up_half);
        let end = start.saturating_add(hight);
        (start, end)
    }
    fn draw_all(
        &mut self,
        frame: &mut Frame,
        src: Vec<String>,
        start_line: usize,
        end_line: usize,
        line_id: usize,
        n: usize,
        file_name: String,
        area_ids: Rect,
        area_split: Rect,
        area_src: Rect,
        area_status: Rect,
    ) {
        let line_id_start_0 = if start_line <= line_id {
            Some(line_id.saturating_sub(start_line))
        } else {
            None
        };
        let mark_as_id = self.draw_src(frame, src, &line_id_start_0, area_src);
        self.draw_id(frame, start_line, end_line, line_id, area_ids);
        self.draw_split(frame, &line_id_start_0, mark_as_id, area_split);
        self.draw_status(frame, n, file_name, area_status);
    }
    fn draw_src(
        &mut self,
        frame: &mut Frame,
        src: Vec<String>,
        line_id_start_0: &Option<usize>,
        area_src: Rect,
    ) -> bool {
        let mut mark_as_id = true;
        let block_src = Block::new()
            .borders(Borders::RIGHT)
            .style(Style::default())
            // .title(title)
            .title_alignment(Alignment::Center);
        let text_src = Text::from_iter(src.iter().enumerate().map(|(id, s)| {
            let first_litter_id = match *line_id_start_0 == Some(id) {
                true => s.chars().into_iter().enumerate().find(|(_, c)| *c != ' '),
                false => None,
            };
            // debug!("line stop {} {:?}", id, first_litter_id);
            let str_iter = s.chars().into_iter().map(|c| Span::raw(c.to_string()));
            Line::from(match first_litter_id {
                Some((0, _)) => str_iter.collect::<Vec<_>>(),
                Some((1, _)) => str_iter.collect::<Vec<_>>(),
                Some((n, _)) => {
                    mark_as_id = false;
                    std::iter::repeat_n(
                        Span::raw('─'.to_string()).style(Style::default().light_green()),
                        n.saturating_sub(2),
                    )
                    .chain(std::iter::repeat_n(
                        Span::raw('>'.to_string()).style(Style::default().light_green()),
                        1,
                    ))
                    .chain(str_iter.skip(n.saturating_sub(1)).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
                }
                _ => str_iter.collect::<Vec<_>>(),
            })
        }));

        let paragraph_src = Paragraph::new(text_src);

        frame.render_widget(paragraph_src, area_src);
        frame.render_widget(block_src, area_src);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area_src, &mut self.vertical_scroll_state);
        return mark_as_id;
    }
    fn draw_status(&mut self, frame: &mut Frame, n: usize, file_name: String, area_status: Rect) {
        let title = format!("{} cmd {}/{} ", &file_name, self.vertical_scroll, n);
        let paragraph_status = Paragraph::new(title).fg(Color::Black).bg(Color::White);
        frame.render_widget(paragraph_status, area_status);
    }
    fn draw_id(
        &mut self,
        frame: &mut Frame,
        start_line: usize,
        end_line: usize,
        line_id: usize,

        area_ids: Rect,
    ) {
        let ids: Vec<usize> = (start_line..end_line.saturating_add(1))
            .into_iter()
            .collect::<Vec<_>>();
        let text_ids = Text::from_iter(ids.iter().map(|s| {
            let line = Line::from_iter(
                s.to_string()
                    .chars()
                    .into_iter()
                    .map(|c| Span::raw(c.to_string())),
            );
            if *s == line_id as usize {
                line.style(Style::default().light_green())
            } else {
                line
            }
        }));

        let paragraph_id = Paragraph::new(text_ids).right_aligned();
        frame.render_widget(paragraph_id, area_ids);
    }

    fn draw_split(
        &mut self,
        frame: &mut Frame,
        line_id_start_0: &Option<usize>,
        mark_as_id: bool,
        area_split: Rect,
    ) {
        let test_split = Text::from_iter(
            std::iter::repeat_n(Span::raw("│ "), area_split.height as usize)
                .enumerate()
                .map(|(id, s)| match (*line_id_start_0 == Some(id), mark_as_id) {
                    (true, true) => Span::raw("├>").style(Style::default().light_green()),
                    (true, false) => Span::raw("├─").style(Style::default().light_green()),
                    (false, _) => s,
                })
                .map(|s| Line::from(s)),
        );
        let paragraph_split = Paragraph::new(test_split);
        frame.render_widget(paragraph_split, area_split);
    }
}

impl Component for Code {
    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<action::Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }
    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> Result<Option<action::Action>> {
        // debug!("gen mouseEvent {:?}", &mouse);
        let ret = match mouse.kind {
            crossterm::event::MouseEventKind::ScrollUp => Ok(Some(action::Action::Up)),
            crossterm::event::MouseEventKind::ScrollDown => Ok(Some(action::Action::Down)),
            _ => Ok(None),
        };
        ret
    }
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        match action {
            action::Action::Tick => {
                // add any logic here that should run on every tick
            }
            action::Action::Render => {
                // add any logic here that should run on every render
            }
            action::Action::Up => {
                self.file_up(1);
            }
            action::Action::Down => {
                self.file_down(1);
            }
            action::Action::Gdbmi(gdbmi::Action::ShowFile((file, line_id))) => {
                self.file_need_show = Some((file.clone(), line_id));
                self.vertical_scroll = line_id as usize;
                match self.files_set.contains(&SrcFileData::new(file.clone())) {
                    false => {
                        if let Some(send) = self.command_tx.clone() {
                            self.files_set.insert(SrcFileData::new(file.clone()));
                            let read_therad = Self::read_file(file.clone(), send.clone());
                            Some(tokio::spawn(async {
                                read_therad.await;
                            }));
                            debug!("read file {} start", file)
                        } else {
                            let msg = format!("read file {} thread not start", &file);
                            error!("{}", &msg);
                        }
                    }
                    true => {
                        debug!("file {} has read", &file);
                    }
                }
            }
            action::Action::Code(Action::FileReadOutLine((file, line))) => {
                match self.files_set.take(&SrcFileData::new(file.clone())) {
                    Some(mut file_data) => {
                        file_data.add_line(line);
                        self.files_set.insert(file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
            }
            action::Action::Code(Action::FileReadEnd(file)) => {
                match self.files_set.take(&SrcFileData::new(file.clone())) {
                    Some(mut file_data) => {
                        file_data.set_read_done();
                        self.files_set.insert(file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let ans = self.get_need_show_file().and_then(|(file, line_id)| {
            let n = file.get_lines_len();
            let num_len = n.to_string().len() as u16;
            let [area, area_status, _] = tool::get_layout(area);
            let [area_ids, area_split, area_src] = Layout::horizontal([
                Constraint::Min(num_len),
                Constraint::Min(2),
                Constraint::Percentage(100),
            ])
            .areas(area);
            Some((n, area_ids, area_split, area_src, area_status))
        });

        ans.map(|(n, _, _, area_src, _)| (area_src.height.clone(), n.clone()))
            .map(|(height, n)| {
                self.legalization_scroll_range(height as usize, n);

                Some(())
            });

        let ans = if let (
            Some((n, area_ids, area_split, area_src, area_status)),
            Some((file, line_id)),
        ) = (ans, self.get_need_show_file())
        {
            let (start_line, end_line) = self.get_windows_show_file_range(area_src.height as usize);
            let (src, start_line, end_line) = file.get_lines_range(start_line, end_line);
            let src = src.iter().map(|s| s.to_string()).collect::<Vec<String>>();
            Some((
                src,
                start_line,
                end_line,
                line_id,
                n,
                file.file_name.clone(),
                area_ids,
                area_split,
                area_src,
                area_status,
            ))
        } else {
            None
        };

        ans.and_then(
            |(
                src,
                start_line,
                end_line,
                line_id,
                n,
                file_name,
                area_ids,
                area_split,
                area_src,
                area_status,
            )| {
                self.draw_all(
                    frame,
                    src,
                    start_line,
                    end_line,
                    line_id as usize,
                    n,
                    file_name,
                    area_ids,
                    area_split,
                    area_src,
                    area_status,
                );
                Some(())
            },
        );
        Ok(())
    }
}

#[test]
fn test_scroll_range() {
    let mut code = Code::default();
    let a = code.legalization_scroll_range(32, 64);
    println! {"let {:?}",a};
    assert!(a == (17 as usize, 49 as usize));
}

#[test]
fn test_scroll_range_2() {
    let mut code = Code::default();
    let a = code.legalization_scroll_range(31, 64);
    println! {"let {:?}",a};
    assert!(a == (16 as usize, 49 as usize));
}

#[test]
fn test_scroll_range_3() {
    let mut code = Code::default();
    let a = code.legalization_scroll_range(31, 2);
    println! {"let {:?}",a};
    assert!(a == (16 as usize, 16 as usize));
}

#[test]
fn test_show_file_range() {
    let mut code = Code::default();
    code.vertical_scroll = 0;
    code.legalization_scroll_range(32, 64);
    let a = code.get_windows_show_file_range(32);
    println! {"let {:?}",a};
    assert!(a == (1 as usize, 33 as usize));
}

#[test]
fn test_show_file_range_2() {
    let mut code = Code::default();
    code.vertical_scroll = 200;
    code.legalization_scroll_range(32, 64);
    let a = code.get_windows_show_file_range(32);
    println! {"let {:?}",a};
    assert!(a == (33 as usize, 65 as usize));
}

#[test]
fn test_show_file_range_3() {
    let mut code = Code::default();
    code.vertical_scroll = 20;
    code.legalization_scroll_range(32, 64);
    let a = code.get_windows_show_file_range(32);
    println! {"let {:?}",a};
    assert!(a == (4 as usize, 36 as usize));
}

#[test]
fn test_show_file_range_4() {
    let mut code = Code::default();
    code.vertical_scroll = 20;
    code.legalization_scroll_range(31, 64);
    let a = code.get_windows_show_file_range(31);
    println! {"let {:?}",a};
    assert!(a == (5 as usize, 36 as usize));
}

#[test]
fn test_file_range_1() {
    let mut file = SrcFileData::new("a".to_string());
    (1..62).into_iter().for_each(|i| {
        file.add_line(format!("{:?}\n", i));
    });
    file.set_read_done();
    let (src, s, e) = file.get_lines_range(4 as usize, 36 as usize);
    assert!(s == 4 as usize);
    assert!(e == 36 as usize);
    println!("file range{:?} {} {}", src, s, e);
    (4..37).zip(src.iter()).for_each(|(i, s)| {
        assert!(format!("{:?}\n", i) == **s);
    });
}

#[test]
fn test_file_range_2() {
    let mut file = SrcFileData::new("a".to_string());
    (1..62).into_iter().for_each(|i| {
        file.add_line(format!("{:?}\n", i));
    });
    file.set_read_done();
    let (src, s, e) = file.get_lines_range(50 as usize, 65 as usize);
    println!("file range{:?} {} {}", src, s, e);
    assert!(s == 50 as usize);
    assert!(e == 62 as usize);
    (50..62).zip(src.iter()).for_each(|(i, s)| {
        assert!(format!("{:?}\n", i) == **s);
    });
}
