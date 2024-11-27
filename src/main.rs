#[macro_use]
extern crate crossterm;

use crossterm::cursor::MoveTo;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use std::cmp::max;
use std::ffi::OsString;
use std::fs::read_dir;
use std::io::{self, stdout};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

struct Config {
    column_height: u8,
}

#[derive(Debug)]
struct Entry {
    is_dir: bool,
    name: String,
}

#[derive(Debug)]
struct Directory {
    is_valid: bool,
    data_width: u8,
    selected_idx: u8,
    start_show_idx: u8,
    contents: Vec<Entry>,
}

impl Directory {
    fn empty_slot() -> Self {
        Directory {
            is_valid: false,
            data_width: 0,
            selected_idx: 0,
            start_show_idx: 0,
            contents: vec![], // I feel like this shouldn't just be an empty vec but
        }
    }
}

struct ProcessDirError;

fn process_dir(p: &Path) -> io::Result<Directory> {
    let dir = read_dir(p)?;
    let dir = match read_dir(p) {
        Ok(d) => d,
        Err(e) => panic!("Unable to process directory: {e}"),
    };

    let mut dirs: Vec<Entry> = Vec::new();
    let mut files: Vec<Entry> = Vec::new();
    let mut max_name_size = 0;

    for entry_path in dir {
        let entry = entry_path?;
        let entry_file_type = entry.file_type()?;
        max_name_size = max(max_name_size, entry.file_name().len());

        if entry_file_type.is_dir() {
            dirs.push(Entry {
                name: entry
                    .file_name()
                    .into_string()
                    .expect("unable to parse file name"),
                is_dir: true,
            });
        } else {
            files.push(Entry {
                name: entry
                    .file_name()
                    .into_string()
                    .expect("unable to parse file name"),
                is_dir: false,
            });
        }
    }

    dirs.append(&mut files);

    Ok(Directory {
        is_valid: true,
        data_width: u8::try_from(max::<usize>(max_name_size, 20)).unwrap(),
        selected_idx: 0,
        start_show_idx: 0,
        contents: dirs,
    })
}

fn init_path_stack(p: &Path) -> (Vec<String>, [Directory; 3]) {
    let mut p_stack: Vec<String> = p
        .iter()
        .map(|component| component.to_os_string().into_string().unwrap())
        .collect();
    p_stack[0] = p_stack[0].split_off(4);

    let mut dirs: [Directory; 3] = [
        Directory::empty_slot(),
        Directory::empty_slot(),
        Directory::empty_slot(),
    ];

    // TODO: REFACTOR THIS LATER, like there has to be a way to turn the processing into it's own function that runs off an iterator
    // a) dirs doesnt have to be in order of appearance, it just has to be reasonable enough for the formatter to accept it
    // b) p_stack.iter().rev() probably gives something like what i want
    // c) macro to extract n'th parent?
    match p_stack.len() {
        0 => panic!(
            "Tried to start application with no working directory. How did that even happen?"
        ),
        1 => {
            // we only have the top most dir to display, so populate it as necessary
            dirs[0] = process_dir(p).expect("failed to process directory");
        }
        2 => {
            // we only have the two top most dirs to display
            dirs[0] = process_dir(p.parent().unwrap()).expect("failed to process directory");
            dirs[1] = process_dir(p).expect("failed to process directory");
        }
        _ => {
            dirs[0] = process_dir(p.parent().unwrap().parent().unwrap())
                .expect("failed to process directory");
            dirs[1] = process_dir(p.parent().unwrap()).expect("failed to process directory");
            dirs[2] = process_dir(p).expect("failed to process directory");
        }
    }

    (p_stack, dirs)
}

fn main() {
    let invoked_dir = Path::new("./").canonicalize().unwrap();
    let (mut stack, mut dirs) = init_path_stack(&invoked_dir);
    println!("{:?}", dirs);

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
    // }

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
}
