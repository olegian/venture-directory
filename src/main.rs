#[macro_use]
extern crate crossterm;

mod constants;
mod types;
use crossterm::{
    cursor::{MoveDown, MoveToRow},
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
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

fn display_dirs(stdout: &mut Stdout, dirs: &Vec<Directory>, row_idx: u16, config: &Config) {
    let n_dirs = dirs.len();
    let show_idx = std::cmp::max(0, n_dirs - config.max_directory_depth);
    let shown_dirs = &dirs[(config.max_directory_depth - show_idx)..];
    let entry_iters = shown_dirs.iter().map(|dir| dir.contents.iter());

    for _ in 0..config.n_rows {
        entry_iters.
    }
}

fn main() {
    let mut config: Config = Default::default();
    let mut stdout = stdout();
    let invoked_dir = Path::new("./").canonicalize().unwrap();

    let mut dirs: Vec<Directory> = init_directories(invoked_dir);

    queue!(stdout, Clear(ClearType::All)).unwrap();

    display_path_at_row(&mut stdout, &dirs, 2, &config);
    display_dirs(&mut stdout, &dirs, 5, &config);

    queue!(stdout, MoveDown(20)).unwrap();
    stdout.flush().unwrap();

    // dirs.iter().for_each(|e| {println!("{}", e.name)});
    // // println!("{dirs:?}")
}
