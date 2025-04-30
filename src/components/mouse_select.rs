pub trait TextSelection {
    /// 处理鼠标事件，返回是否更新选择状态
    fn handle_selection(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> Option<(bool, MouseSelect)>;

    /// 获取选中文本
    fn get_selected_text(&self, select: &MouseSelect) -> Option<String>;

    /// 获取选中的位置
    /// 返回值为 Option<Vec<SelectionRange>>，其中：
    /// - line_number: 行号
    /// - start_column: 选中区域的起始列
    /// - end_column: 选中区域的结束列
    fn get_selected_area(&self, select: &MouseSelect) -> Option<Vec<SelectionRange>>;
}

/// 表示一个选中区域的范围
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionRange {
    pub line_number: usize,
    pub start_column: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MouseSelect {
    pub start: (u16, u16),
    pub end: (u16, u16),
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
