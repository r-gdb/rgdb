use super::Component;
use crate::action;
use color_eyre::eyre::Ok;
use crossterm::event::MouseButton;
use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    text::Text,
    widgets::Paragraph,
    Frame,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};
use strum::Display;
use tracing::{debug, error};

#[derive(Debug, Clone, PartialEq, Hash, Eq, Display, Serialize, Deserialize)]
pub enum SelectionRangeType {
    SrcWindow,
    GdbtTTYWindeow,
}

#[derive(Debug, Clone, Eq, PartialEq, Display, Serialize, Deserialize)]
pub enum Action {
    SelectionRange((bool, MouseSelect)),
    AddSelectionRange((SelectionRangeType, Vec<SelectionRange>)),
    DelectSelectionRange(SelectionRangeType),
}

/// 表示一个选中区域的范围
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    pub line_number: usize,
    pub start_column: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct MouseSelect {
    pub start: (u16, u16),
    pub end: (u16, u16),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MouseSelectComponent {
    select_range_now: Option<MouseSelect>,
    select_ranges: HashMap<SelectionRangeType, Vec<SelectionRange>>,
}

pub trait TextSelection {
    /// 获取选中文本
    fn get_selected_text(&self, select: &MouseSelect) -> Option<String>;

    /// 获取选中的位置
    /// 返回值为 Option<Vec<SelectionRange>>，其中：
    /// - line_number: 行号
    /// - start_column: 选中区域的起始列
    /// - end_column: 选中区域的结束列
    fn get_selected_area(&self, select: &MouseSelect) -> Option<Vec<SelectionRange>>;
}

impl MouseSelectComponent {
    pub fn new() -> Self {
        MouseSelectComponent {
            select_range_now: None,
            select_ranges: HashMap::new(),
        }
    }

    fn draw_select(&self, frame: &mut Frame, select_ranges: &Vec<&SelectionRange>) {
        select_ranges.iter().for_each(|s| {
            let select_len = s.end_column.saturating_sub(s.start_column);
            // let text = Text::from_iter(Line::from_iter(std::iter::repeat_n("", select_len)));
            let text = Text::from("");
            let paragraph = Paragraph::new(text).bg(Color::Rgb(255, 204, 153));
            let area = Rect::new(
                s.start_column as u16,
                s.line_number as u16,
                select_len as u16,
                1,
            );
            frame.render_widget(paragraph, area);
            debug!("draw select area {:?}", &area);
        });
    }

    fn handle_selection(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> Option<(bool, MouseSelect)> {
        let pos = (mouse.row, mouse.column);
        let ret = match mouse.kind {
            crossterm::event::MouseEventKind::Down(MouseButton::Left) => {
                self.select_range_now = Some(MouseSelect {
                    start: pos,
                    end: pos,
                });
                self.select_range_now.clone().and_then(|s| Some((false, s)))
            }
            crossterm::event::MouseEventKind::Drag(MouseButton::Left) => {
                match self.select_range_now {
                    Some(ref mut select) => {
                        select.end = pos;
                    }
                    None => {
                        error!("Mouse selection not initialized");
                    }
                }
                self.select_range_now.clone().and_then(|s| Some((false, s)))
            }
            crossterm::event::MouseEventKind::Up(MouseButton::Left) => {
                match self.select_range_now {
                    Some(ref mut select) => {
                        select.end = pos;
                    }
                    None => {
                        error!("Mouse selection not initialized");
                    }
                }
                let ret = self.select_range_now.clone().and_then(|s| Some((true, s)));
                self.select_range_now = None;
                ret
            }
            _ => None,
        };
        let ret = ret.and_then(|(flag, select)| Some((flag, select.legalization())));
        ret
    }
}
impl MouseSelect {
    pub fn legalization(&self) -> MouseSelect {
        match self.start < self.end {
            true => MouseSelect {
                start: self.start,
                end: self.end,
            },
            false => MouseSelect {
                start: self.end,
                end: self.start,
            },
        }
    }
}

impl Component for MouseSelectComponent {
    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> color_eyre::eyre::Result<()> {
        let range = self
            .select_ranges
            .iter()
            .map(|(_, v)| v)
            .flatten()
            .collect::<Vec<_>>();
        self.draw_select(frame, &range);
        Ok(())
    }
    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> color_eyre::eyre::Result<Option<crate::action::Action>> {
        let action = match mouse.kind {
            crossterm::event::MouseEventKind::Up(MouseButton::Left)
            | crossterm::event::MouseEventKind::Down(MouseButton::Left)
            | crossterm::event::MouseEventKind::Drag(MouseButton::Left) => {
                // 处理鼠标选择事件
                match self.handle_selection(mouse.clone()) {
                    Some(v) => Some(action::Action::MouseSelect(Action::SelectionRange(v))),
                    _ => None,
                }
            }
            _ => None,
        };
        Ok(action)
    }
    fn update(
        &mut self,
        action: action::Action,
    ) -> color_eyre::eyre::Result<Option<action::Action>> {
        match action {
            action::Action::MouseSelect(Action::AddSelectionRange((range_type, ranges))) => {
                // 更新或插入新的选择范围
                self.select_ranges.insert(range_type, ranges);
                Ok(None)
            }
            action::Action::MouseSelect(Action::DelectSelectionRange(range_type)) => {
                // 删除指定类型的选择范围
                self.select_ranges.remove(&range_type);
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
