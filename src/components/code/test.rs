#[cfg(test)]
mod tests {
    use crate::components::code::AsmFuncData;
    use crate::components::code::BreakPointData;
    use crate::components::code::Code;
    use crate::components::code::SrcFileData;
    use crate::mi::breakpointmi::BreakPointSignalActionSrc;
    use crate::mi::breakpointmi::{BreakPointAction, BreakPointSignalAction};
    use crate::tool::HashSelf;
    use crate::tool::TextFileData;
    use std::collections::HashMap;
    #[test]
    fn test_crtl_ascii_00_0f() {
        let line = "\u{0}\u{1}\u{2}\u{3}\u{4}\u{5}\u{6}\u{7}\u{8}\u{b}\u{c}\r\u{e}\u{f}";
        let line = SrcFileData::read_file_filter(line.to_string());
        println!("{:?}", line);
        assert!(
            line == r##"\{NUL}\{SOH}\{STX}\{ETX}\{EOT}\{ENQ}\{ACK}\{BEL}\{BS}\{VT}\{FF}\{SO}\{SI}"##
        );
    }
    #[test]

    fn test_crtl_ascii_10_1f() {
        let line = "\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}\u{18}\u{19}\u{1a}\u{1b}\u{1c}\u{1d}\u{1e}\u{1f}\u{7f}";
        let line = SrcFileData::read_file_filter(line.to_string());
        assert!(
            line == r##"\{DLE}\{DC1}\{DC2}\{DC3}\{DC4}\{NAK}\{SYN}\{ETB}\{CAN}\{EM}\{SUB}\{ESC}\{FS}\{GS}\{RS}\{US}\{DEL}"##
        );
    }
    #[test]
    fn test_crtl_ascii_7f() {
        let line = "\u{7f}";
        let line = SrcFileData::read_file_filter(line.to_string());
        assert!(line == r##"\{DEL}"##);
    }
    #[test]
    fn test_crtl_ascii_tab() {
        let line = "\t";
        let line = SrcFileData::read_file_filter(line.to_string());
        assert!(line == "    ");
    }
    #[test]
    fn test_scroll_range() {
        let mut code = Code::default();
        let a = code.legalization_vertical_scroll_range(32, 64);
        println! {"let {:?}",a};
        assert!(a == (17_usize, 49_usize));
    }

    #[test]
    fn test_scroll_range_2() {
        let mut code = Code::new();
        let a = code.legalization_vertical_scroll_range(31, 64);
        println! {"let {:?}",a};
        assert!(a == (16_usize, 49_usize));
    }

    #[test]
    fn test_scroll_range_3() {
        let mut code = Code::new();
        let a = code.legalization_vertical_scroll_range(31, 2);
        println! {"let {:?}",a};
        assert!(a == (16_usize, 16_usize));
    }

    #[test]
    fn test_show_file_range() {
        let mut code = Code::new();
        code.vertical_scroll = 0;
        code.legalization_vertical_scroll_range(32, 64);
        let a = code.get_windows_show_file_range(32);
        println! {"let {:?}",a};
        assert!(a == (1_usize, 33_usize));
    }

    #[test]
    fn test_show_file_range_2() {
        let mut code = Code::new();
        code.vertical_scroll = 200;
        code.legalization_vertical_scroll_range(32, 64);
        let a = code.get_windows_show_file_range(32);
        println! {"let {:?}",a};
        assert!(a == (33_usize, 65_usize));
    }

    #[test]
    fn test_show_file_range_3() {
        let mut code = Code::new();
        code.vertical_scroll = 20;
        code.legalization_vertical_scroll_range(32, 64);
        let a = code.get_windows_show_file_range(32);
        println! {"let {:?}",a};
        assert!(a == (4_usize, 36_usize));
    }

    #[test]
    fn test_show_file_range_4() {
        let mut code = Code::new();
        code.vertical_scroll = 20;
        code.legalization_vertical_scroll_range(31, 64);
        let a = code.get_windows_show_file_range(31);
        println! {"let {:?}",a};
        assert!(a == (5_usize, 36_usize));
    }

    #[test]
    fn test_file_range_1() {
        let mut file = SrcFileData::new("a".to_string());
        (1..62).for_each(|i| {
            file.add_line(format!("{:?}\n", i));
        });
        file.set_read_done();
        let (src, s, e) = file.get_lines_range(4_usize, 36_usize);
        assert!(s == 4_usize);
        assert!(e == 36_usize);
        println!("file range{:?} {} {}", src, s, e);
        (4..37).zip(src.iter()).for_each(|(i, s)| {
            assert!(format!("{:?}\n", i) == **s);
        });
    }

    #[test]
    fn test_file_range_2() {
        let mut file = SrcFileData::new("a".to_string());
        (1..62).for_each(|i| {
            file.add_line(format!("{:?}\n", i));
        });
        file.set_read_done();
        let (src, s, e) = file.get_lines_range(50_usize, 65_usize);
        println!("file range{:?} {} {}", src, s, e);
        assert!(s == 50_usize);
        assert!(e == 62_usize);
        (50..62).zip(src.iter()).for_each(|(i, s)| {
            assert!(format!("{:?}\n", i) == **s);
        });
    }

    #[test]
    fn f_breakpoint_range() {
        use crate::mi::breakpointmi::BreakPointMultipleAction;
        let a = BreakPointAction::Multiple(BreakPointMultipleAction {
            number: "5".to_string(),
            enabled: false,
            bps: vec![
                BreakPointSignalAction::Src(BreakPointSignalActionSrc {
                    number: "5.1".to_string(),
                    enabled: true,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                }),
                BreakPointSignalAction::Src(BreakPointSignalActionSrc {
                    number: "5.1".to_string(),
                    enabled: false,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                }),
            ],
        });
        let a = BreakPointData::from(&a);
        let mut code = Code::new();
        code.breakpoint_set.insert(a.get_key(), a);
        let ans = SrcFileData::new("/home/shizhilvren/tmux/environ.c".to_string())
            .get_breakpoint_need_show_in_range(code.get_breakpoints(), 22, 39);
        println!("{:?}", ans);
        assert!(ans == HashMap::from([(34_u64, false)]));
    }

    #[test]
    fn f_breakpoint_range_2() {
        use crate::mi::breakpointmi::BreakPointMultipleAction;
        let a = BreakPointAction::Multiple(BreakPointMultipleAction {
            number: "5".to_string(),
            enabled: true,
            bps: vec![
                BreakPointSignalAction::Src(BreakPointSignalActionSrc {
                    number: "5.1".to_string(),
                    enabled: true,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                }),
                BreakPointSignalAction::Src(BreakPointSignalActionSrc {
                    number: "5.1".to_string(),
                    enabled: false,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                }),
            ],
        });

        let a = BreakPointData::from(&a);
        let mut code = Code::new();
        code.breakpoint_set.insert(a.get_key(), a);
        let ans = SrcFileData::new("/home/shizhilvren/tmux/environ.c".to_string())
            .get_breakpoint_need_show_in_range(code.get_breakpoints(), 22, 39);

        assert!(ans == HashMap::from([(34_u64, true)]));
    }

    #[test]
    fn f_breakpoint_range_3() {
        let a = BreakPointAction::Signal(BreakPointSignalAction::Src(BreakPointSignalActionSrc {
            number: "2".to_string(),
            enabled: true,
            line: 34_u64,
            fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
        }));
        let b = BreakPointAction::Signal(BreakPointSignalAction::Src(BreakPointSignalActionSrc {
            number: "6".to_string(),
            enabled: true,
            line: 37_u64,
            fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
        }));
        let a = BreakPointData::from(&a);
        let b = BreakPointData::from(&b);
        let mut code = Code::new();
        code.breakpoint_set.insert(a.get_key(), a);
        code.breakpoint_set.insert(b.get_key(), b);
        let ans = SrcFileData::new("/home/shizhilvren/tmux/environ.c".to_string())
            .get_breakpoint_need_show_in_range(code.get_breakpoints(), 22, 36);
        assert!(ans == HashMap::from([(34_u64, true)]));
    }

    #[test]
    fn f_get_line_id() {
        let asm = AsmFuncData {
            func_name: std::rc::Rc::new("main".to_string()),
            addrs: vec![(0x01a as u64, 2), (0x02b as u64, 3), (0x12b as u64, 5)],
            lines: vec![],
            lines_highlight: vec![],
            read_done: true,
            highlight_done: true,
        };
        let id = asm.get_line_id(&"0x000001a".to_string());
        println!("{:?}", &id);
        assert!(id == Some(2));
    }
}
