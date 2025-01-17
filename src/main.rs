#[macro_use]
extern crate crossterm;

use crossterm::cursor::{MoveDown, MoveLeft, MoveRight, MoveTo, MoveUp};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Print, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, stdout, Stdout};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

/* TODO WHEN THE THING KINDA WORKS:
    - change all direct terminal execution to be lazy execution, flush on the end of the user loop on change?
    - make the user input blocking until input is received, rather than constantly rerunning the loop with NOOPs
    - Vec vs array in directory management?
    - Magic numbers (a lot of them move into config?)
    - Fuzzy find files from root directory
    - Conditioanlly hide hidden files
    - Conditionally hide non-directory files
    - a SHITTON of code cleanup wow this is starting off as a messy repo. use files dumbass.
*/

struct Config {
    column_height: usize,
    max_entry_width: usize,
    depth: usize,
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

fn init_path_stack(p: &Path) -> Vec<String> {
    let mut p_stack: Vec<String> = p
        .iter()
        .map(|component| component.to_os_string().into_string().unwrap())
        .collect();

    // remove windows formatting, join together windows root directory name.
    p_stack[0] = p_stack[0].split_off(4);
    p_stack.remove(1);

    p_stack
}

fn populate_display_dirs(tail_dir: &Path, config: &Config) -> VecDeque<Directory> {
    let mut dirs = VecDeque::with_capacity(config.depth);
    let mut next_name = String::new();
    for dir in tail_dir.ancestors() {
        if dirs.len() >= config.depth {
            break;
        }

        let dir_entry = dir.read_dir().expect("Unable to read directory");
        let mut longest_name = OsString::from("");
        let mut bcd_entries = dir_entry.enumerate().map(|(i, entry)| {
            let e = entry.unwrap();
            let e_name = e.file_name();
            let e_metadata = e.metadata().unwrap();
            if longest_name.len() < e_name.len() {
                longest_name = e_name.clone();
            }
            Entry {
                is_dir: e_metadata.is_dir(),
                is_hidden: (e_metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0, // wow this is ugly asf
                name: e_name.into_string().unwrap(),
            }
        });

        let mut contents: Vec<Entry> = bcd_entries.collect();
        contents.sort_by(|left, right| left.is_dir.cmp(&right.is_dir));
        for i in 0..contents.len() {
            if contents.get(0).unwrap().is_dir {
                break;
            }
            contents.rotate_left(1);
        }

        // need to find next_name idx in list
        let mut selected_idx = 0;
        if next_name != "" {
            selected_idx = contents.iter().position(|e| e.name == next_name).unwrap();
        }


        let name = dir.file_name().unwrap_or(&OsString::from("/")).to_str().unwrap().to_string();
        let mut res = Directory {
            contents: contents,
            data_width: longest_name.len(),
            selected_idx: selected_idx,
            name: name,
            start_show_idx: 0,
        };

        if res.selected_idx > config.column_height {
            res.start_show_idx = std::cmp::max(res.selected_idx - 3, 0);
        }

        next_name = res.name.clone();
        dirs.push_front(res);
    }

    dirs
}

fn draw_directory(stdout: &mut Stdout, dir: &Directory, is_current: bool, config: &Config) -> (usize, usize) {
    let mut dy = 0;
    let pad_size = std::cmp::min(dir.data_width, config.max_entry_width);
    let mut it = dir.contents.iter().enumerate();
    if dir.start_show_idx > 0 {
        it.nth(dir.start_show_idx - 1);
    }

    /*
    COOL ASS IDEA: create a new struct that manually implements slicing mechanics to handle shifting / moving around
    something like {startidx, endidx, currentidx}, that indexes into some structure 
    (could be contained in the same struct; to determine what is shown, to not have to reiterate anything when redrawing)

    OK BUT this functionality is basically captured in the start_show_idx or whatever tf you called it, just recalculate it
    or create a macro cause fuck storing additional state. something something cache coherance something something idk
     */

    for (row_idx, row_entry) in it {
        if config.column_height <= row_idx - dir.start_show_idx {
            break;
        }

        let name = if config.max_entry_width < row_entry.name.len() {
            &row_entry.name[..config.max_entry_width]
        } else {
            &row_entry.name[..]
        };

        let adtl = match (
            is_current,  // currently hovered
            config.max_entry_width < row_entry.name.len(),  // needs ellipsis
            row_idx == dir.selected_idx,  // already entered
        ) {
            (true, true, _) => "... ",
            (false, true, false) => "... ",
            (true, false, _) => "    ",
            (false, false, false) => "    ",
            (false, true, true) => "...>",
            (false, false, true) => "--->",
        };

        let formatted_name = if row_idx == dir.selected_idx {
            if is_current {
                format!("{:<pad_size$}{}", name, adtl).red()
            } else {
                format!("{:-<pad_size$}{}", name, adtl).green()
            }
        } else {
            if row_entry.is_dir {
                format!("{:<pad_size$}{}", name, adtl).cyan()
            } else {
                format!("{:<pad_size$}{}", name, adtl).reset()
            }
        };

        execute!(stdout, Print(format!("| {} |", formatted_name))).unwrap();
        execute!(stdout, MoveDown(1), MoveLeft((pad_size + 8) as u16)).unwrap();

        dy += 1;
    }

    while dy < config.column_height {
        execute!(stdout, Print(format!("| {:<pad_size$}     |", ""))).unwrap();
        execute!(stdout, MoveDown(1), MoveLeft((pad_size + 8) as u16)).unwrap();
        dy += 1;
    }

    (pad_size + 8, dy)
}

fn draw_dir_selector(stdout: &mut Stdout, display_data: &VecDeque<Directory>, config: &Config, x: u16, y: u16) {
    execute!(stdout, MoveTo(x, y)).unwrap();
    let mut data_iterator = display_data.iter().peekable();
    while let Some(dir) = data_iterator.next() {
        let (dx, dy) = draw_directory(stdout, dir,data_iterator.peek().is_none(),  config);
        execute!(stdout, MoveUp(dy as u16), MoveRight((dx - 1) as u16)).unwrap();
    }
}

fn draw_current_path(stdout: &mut Stdout, path_stack: &Vec<String>, x: u16, y:u16) {
    let path = path_stack.join("/");
    execute!(stdout, MoveTo(x, y), Print(path)).unwrap();
}

fn exit_directory(path_stack: &Vec<String>, display_data: &VecDeque<Directory>, config: &Config) {
    // lowkey i kinda like having the entire path opened up via Directory structs, and then using another indexing thing to 
    // figure out how much of the tail of that Vec<Directory> to actually display, just simplifies having to conditionally
    // open a head-directory on closing the tail-directory.

    // DO THAT RE-WRITE FIRST, THIS IS SOMETHING NOT WORTH SKIMPING ACTUALLY, ITLL MAKE OPEN AND FUZZY FINDING STUFF MUCH EASIER.
    // although fuzzy finding will be based off some index, so like not really but itll still be marginally easier.
}

fn user_loop(stdout: &mut Stdout, path_stack: &Vec<String>, display_data: &VecDeque<Directory>, config: &Config) -> String {
    loop {
        match read().unwrap() {
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => {
                exit_directory(path_stack, display_data, config);
                execute!(stdout, Clear(ClearType::All)).unwrap();
                draw_current_path(stdout, path_stack, 1, 1);
                draw_dir_selector(stdout, display_data, config, 1, 3);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => {
                execute!(stdout, Print("LEFT")).unwrap();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => {
                execute!(stdout, Print("LEFT")).unwrap();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => {
                execute!(stdout, Print("LEFT")).unwrap();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => { 
                break;
            },
            _ => (),
        };
    }

    String::from("")
}

fn main() {
    let invoked_dir = Path::new("./").canonicalize().unwrap();
    let mut stdout = stdout();
    let mut config = Config {
        column_height: 10,
        max_entry_width: 20,
        depth: 5,
    };

    let mut path_stack = init_path_stack(&invoked_dir);
    let mut display_data = populate_display_dirs(&mut invoked_dir.clone(), &config);

    enable_raw_mode().unwrap();
    execute!(stdout, Clear(ClearType::All)).unwrap();

    draw_current_path(&mut stdout, &path_stack, 1, 1);
    draw_dir_selector(&mut stdout, &display_data, &config, 1, 3);
    let final_path: String = user_loop(&mut stdout, &path_stack, &display_data, &config);
    disable_raw_mode().unwrap();

    execute!(stdout, MoveDown(20)).unwrap();
    println!("{}", final_path);
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
