#[macro_use]
extern crate crossterm;

use crossterm::cursor::{MoveDown, MoveLeft, MoveRight, MoveTo};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use std::cmp::max;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::read_dir;
use std::io::{self, stdout, Stdout};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

struct Config {
    column_height: usize,
    max_entry_width: usize,
}

#[derive(Debug, Clone)]
struct Entry {
    is_dir: bool,
    is_hidden: bool,
    name: String,
}

#[derive(Debug, Clone)]
struct Directory {
    data_width: usize,
    selected_idx: usize,
    start_show_idx: usize,
    contents: Vec<Entry>,
    name: String,
}
struct ProcessDirError;

fn init_path_stack(p: &Path) -> Vec<String> {
    let mut p_stack: Vec<String> = p
        .iter()
        .map(|component| component.to_os_string().into_string().unwrap())
        .collect();

    // remove windows formatting, join together windows root directory name.
    p_stack[0] = p_stack[0].split_off(4);
    p_stack[0] = p_stack[0..2].join("");
    p_stack.remove(1);

    p_stack
}

fn populate_display_dirs(tail_dir: &Path) -> VecDeque<Directory> {
    let mut dirs = VecDeque::with_capacity(3);
    let mut prev_selected_idx: usize = 0;
    for dir in tail_dir.ancestors() {
        if dirs.len() >= 3 {
            break;
        }

        let dir_entry = dir.read_dir().expect("Unable to read directory");
        let mut longest_name = OsString::from("");
        let mut next_selected_idx = 0;
        let bcd_entries = dir_entry.enumerate().map(|(i, entry)| {
            let e = entry.unwrap();
            let e_name = e.file_name();
            let e_metadata = e.metadata().unwrap();
            if longest_name.len() < e_name.len() {
                longest_name = e_name.clone();
            }

            if e_name == dir.file_name().unwrap() {
                next_selected_idx = i;
            }

            Entry {
                is_dir: e_metadata.is_dir(),
                is_hidden: (e_metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0,  // wow this is ugly asf
                name: e_name.into_string().unwrap(),
            }
        });

        let res = Directory {
            contents: bcd_entries.collect(),
            data_width: longest_name.len(),
            selected_idx: prev_selected_idx,
            name: dir.file_name().unwrap().to_str().unwrap().to_string(),
            start_show_idx: 0,
        };

        prev_selected_idx = next_selected_idx;

        dirs.push_front(res);
    }

    dirs
}

fn draw_directory(stdout: &mut Stdout, offset: usize, dir: &Directory, config: &Config) -> (usize, usize) {
    let mut sorted = dir.clone();
    sorted.contents.sort_by(|left, right| {
        left.is_dir.cmp(&right.is_dir)
    });

    for i in 0..sorted.contents.len() {
        if sorted.contents.get(0).unwrap().is_dir {
            break;
        }

        sorted.contents.rotate_left(1)
    }


    let mut dy = 0;
    let pad_size = std::cmp::min(dir.data_width, config.max_entry_width);
    for (row_idx, row_entry) in sorted.contents.iter().enumerate() {
        let mut name = &row_entry.name[..];
        let mut adtl = if row_idx == dir.selected_idx {"----"} else {"    "};
        if config.max_entry_width < name.len() {
            name = &name[..config.max_entry_width];
            adtl = "... "
        }

        let name = if row_idx == dir.selected_idx {
            format!("{:-<pad_size$}{}", name, adtl)
        } else {
            format!("{:<pad_size$}{}", name, adtl)
        };

        execute!(stdout, Print(format!("| {} |", name))).unwrap();
        execute!(stdout, MoveDown(1), MoveLeft((pad_size + 8) as u16)).unwrap();

        dy += 1;
    }

    (pad_size + 8, dy)
}

fn draw_screen(stdout: &mut Stdout, display_data: &VecDeque<Directory>, config: &Config) {
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    draw_directory(stdout, 0, display_data.get(0).unwrap(), config);
}

fn main() {
    let invoked_dir = Path::new("./").canonicalize().unwrap();
    let mut path_stack = init_path_stack(&invoked_dir);
    let mut display_data = populate_display_dirs(&mut invoked_dir.clone());
    let mut stdout = stdout();
    let mut config = Config {
        column_height: 10,
        max_entry_width: 20,
    };

    enable_raw_mode().unwrap();
    draw_screen(&mut stdout, &display_data, &config);



    // println!("{:?}", path_stack);
    // println!("{:?}", display_data);
}

// enable_raw_mode().unwrap();

// let mut stdout = stdout();
// execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), Print("Hello!"),).unwrap();

// loop {
//     match read().unwrap() {
//         Event::Key(KeyEvent {
//             code: KeyCode::Left,
//             modifiers: KeyModifiers::NONE,
//             kind: KeyEventKind::Press,
//             state,
//         }) => {
//             execute!(stdout, Print("LEFT")).unwrap();
//         }
//         Event::Key(KeyEvent {
//             code: KeyCode::Right,
//             modifiers: KeyModifiers::NONE,
//             kind: KeyEventKind::Press,
//             state,
//         }) => {
//             execute!(stdout, Print("LEFT")).unwrap();
//         }
//         Event::Key(KeyEvent {
//             code: KeyCode::Up,
//             modifiers: KeyModifiers::NONE,
//             kind: KeyEventKind::Press,
//             state,
//         }) => {
//             execute!(stdout, Print("LEFT")).unwrap();
//         }
//         Event::Key(KeyEvent {
//             code: KeyCode::Down,
//             modifiers: KeyModifiers::NONE,
//             kind: KeyEventKind::Press,
//             state,
//         }) => {
//             execute!(stdout, Print("LEFT")).unwrap();
//         }
//         Event::Key(KeyEvent {
//             code: KeyCode::Esc,
//             modifiers: KeyModifiers::NONE,
//             kind: KeyEventKind::Press,
//             state,
//         }) => break,
//         _ => (),
//     }

// disable_raw_mode().unwrap();

// let term: Term = Term::stdout();
// let (_height, width) = term.size();
// let text = "hello world!!!";
// let to_print = pad_str(text, usize::from(width), Alignment::Center, None);
// term.write_line(&to_print).unwrap();

// let invoked_dir = Path::new("./");
// println!("{:?}", invoked_dir.canonicalize().unwrap());
// let current_dir = fs::read_dir(invoked_dir).unwrap();

// for path in current_dir {
//     let entry = match path {
//         Ok(dir_entry) => dir_entry,
//         Err(e) => panic!("Unable to open directory metadata: {}", e),
//     };

//     let entry_file_type = match entry.file_type() {
//         Ok(file_type) => file_type,
//         Err(e) => panic!("Unable to extract directory type info: {}", e),
//     };

//     println!("{}, {:?}", entry.file_name().into_string().unwrap(), entry_file_type.is_dir());
// }

// let mut stack: Vec<&str> = Vec::new();
// stack.push("hello world");
// stack.push("aaaa");
// let res = stack.pop().unwrap();
// println!("{}, {:?}", res, stack);
