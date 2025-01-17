#[macro_use]
extern crate crossterm;

mod constants;
mod types;
use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight, MoveTo, MoveToRow}, event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers}, style::{Print, Stylize}, terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType}
};
use std::{
    collections::VecDeque,
    ffi::OsString,
    io::{stdout, Error, Stdout, Write},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

use types::{Config, Directory, Entry};

// Data initialization ////////////////////////////////////////////////////
fn read_dir(path: &Path, next_dir: Option<&str>) -> Result<Directory, Error> {
    // get list of directory contents
    let mut dir_entries: Vec<Entry> = path
        .read_dir()?
        .map(|e| {
            let entry = e.unwrap();
            let md = entry.metadata().unwrap();

            Entry {
                name: String::from(entry.file_name().to_str().unwrap()),
                is_dir: md.is_dir(),
                is_hidden: (md.file_attributes() & constants::FILE_ATTRIBUTE_HIDDEN) != 0, // wow this is ugly asf
            }
        })
        .collect();

    // order it by directories first, then alphabetically
    dir_entries.sort_by(|left, right| right.is_dir.cmp(&left.is_dir));
    let selected_idx: usize = match next_dir {
        Some(target) => dir_entries
            .iter()
            .position(|e| e.name == target)
            .expect("Dir doesn't contain future dir"),
        None => 0,
    };

    // Get name of directory
    let name = String::from(
        path.file_name()
            .unwrap_or(&OsString::from("/"))
            .to_str()
            .unwrap(),
    );

    Ok(Directory::new(name, dir_entries, selected_idx))
}

fn init_directories(path: PathBuf) -> Vec<Directory> {
    let mut res: VecDeque<Directory> = VecDeque::new();
    for p in path.ancestors() {
        let next_name: Option<&str> = match res.front() {
            Some(e) => Some(&e.name),
            None => None,
        };

        let dir = read_dir(p, next_name).expect("Unable to read in directory");
        res.push_front(dir);
    }

    Vec::from(res)
}

// Display Functions ///////////////////////////////////////
fn display_path_at_row(stdout: &mut Stdout, dirs: &Vec<Directory>, row_idx: u16, config: &Config) {
    let mut dirs = dirs.iter();
    dirs.next(); // this resolves fenceposting with the root dir

    let full_name = dirs
        .map(|dir| &dir.name)
        .fold(String::new(), |acc, b| acc + "/" + b);

    queue!(stdout, MoveToRow(row_idx), Print(full_name)).unwrap();
}

fn get_entry_display_start_idx(dir: &Directory, config: &Config) -> usize {
    if dir.selected_idx < config.n_rows {
        // if the selected index is inbounds of the number of rows we are showing, then show from the start
        return 0;
    }

    if dir.selected_idx + config.n_rows > dir.contents.len() {
        // if the selected index is towards the end, show the last page
        return dir.contents.len() - config.n_rows;
    }

    // selected index is out of bounds of n rows, and not towards the end, so start from wherever in the list
    return dir.selected_idx;
}

fn display_dirs(stdout: &mut Stdout, dirs: &Vec<Directory>, row_idx: u16, config: &Config) {
    queue!(stdout, MoveTo(10, 10));

    let n_dirs = dirs.len();
    let n_show = if n_dirs < config.max_directory_depth {
        n_dirs
    } else {
        config.max_directory_depth
    };

    let shown_dirs = &dirs[n_dirs - n_show..];

    let mut dirs_with_idxs = shown_dirs
        .iter()
        .map(|d| (d, get_entry_display_start_idx(d, config)))
        .cycle();

    for e_offset in 0..config.n_rows {
        let mut line_len: u16 = 2;
        queue!(stdout, Print("| "));
        for dir_idx in 0..dirs.len() {
            let current_dir = dirs_with_idxs.next();
            match current_dir {
                Some((d, start)) => {
                    let mut repr = d.get_padded_entry_repr(
                        start + e_offset,
                        dir_idx == dirs.len() - 1,
                        config,
                    );

                    line_len += repr.content().len() as u16;

                    queue!(stdout, Print(repr));
                }
                None => panic!("Cycled iterator returned none. Was there nothing in shown_dirs?"),
            }

            queue!(stdout, Print(" |"));
            line_len += 2
        }

        queue!(stdout, MoveLeft(line_len), MoveDown(1)).unwrap();
    }
}

fn main() {
    let mut config: Config = Default::default();
    config.max_col_width = 20;
    config.max_directory_depth = 5;
    config.n_rows = 20;

    let mut stdout = stdout();
    let invoked_dir = Path::new("./").canonicalize().unwrap();
    let mut dirs: Vec<Directory> = init_directories(invoked_dir);

    enable_raw_mode().unwrap();
    loop {
        queue!(stdout, Clear(ClearType::All)).unwrap();
        display_path_at_row(&mut stdout, &dirs, 2, &config);
        display_dirs(&mut stdout, &dirs, 5, &config);
        stdout.flush().unwrap();

        match read().unwrap() {
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => {
                if dirs.len() <= 1 {
                    continue;
                }

                dirs.pop();
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
                let end = dirs.len() - 1;
                dirs.get_mut( end).unwrap().select_prev();

            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => {
                let end = dirs.len() - 1;
                dirs.get_mut( end).unwrap().select_next();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state,
            }) => break,
            _ => (),
        }
    }

    /*
     TODO:
     - Consider the ordering of the dirs vector, because currently we have to use get_mut to the last element which is im pretty sure worse than accesssing the front, and we only need one ended access honestly
     - Shit scrolling feel especially when approaching out of n_rows bound
     - Somehow get rid of screen flashes on action (could be cause by double buffering, clear only changed contents somehow?)
     - Finish opening directories
     - Return final path
     - Styling
    */

    disable_raw_mode().unwrap();

    queue!(stdout, MoveDown(30)).unwrap();
    stdout.flush().unwrap();
}
