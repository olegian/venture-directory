use crossterm::style::{StyledContent, Stylize};

use crate::constants;

pub struct Config {
    pub max_path_length: usize,
    pub max_directory_depth: usize,
    pub n_rows: usize,
    pub max_col_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_directory_depth: Default::default(),
            n_rows: Default::default(),
            max_path_length: Default::default(),
            max_col_width: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Directory {
    pub name: String,
    pub contents: Vec<Entry>,
    pub selected_idx: usize,
    pub longest_name_len: usize,
}

impl Directory {
    pub fn select_next(&mut self) {
        if self.selected_idx < self.contents.len() {
            self.selected_idx += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn new(name: String, contents: Vec<Entry>, selected_idx: usize) -> Self {
        let len = contents.iter().max_by_key(|a| { a.name.len() }).unwrap().name.len();
        Self {
            name: name,
            contents: contents,
            selected_idx: selected_idx,
            longest_name_len: len,
        }
    }

    pub fn get_padded_entry_repr(&self, idx: usize, is_tail: bool, config: &Config) -> StyledContent<String> {
        let pad_size = config.max_col_width.min(self.longest_name_len);
        if idx >= self.contents.len() {
            return format!("{:<pad_size$}       ", "").reset()
        }

        let entry = &self.contents[idx];
        let name = if idx == self.selected_idx && !is_tail {
            let (name_len, adtl) = if config.max_col_width < entry.name.len() {
                (config.max_col_width, "...-")
            } else {
                (entry.name.len(), "----")
            };

            format!("{:-<pad_size$}{}-->", &entry.name[..name_len], adtl)
        } else {
            let (name_len, adtl) = if config.max_col_width < entry.name.len() {
                (config.max_col_width, "... ")
            } else {
                (entry.name.len(), "    ")
            };

            format!("{:<pad_size$}{}   ", &entry.name[..name_len], adtl)
        };

        if is_tail && idx == self.selected_idx {
            name.red()
        } else if idx == self.selected_idx {
            name.green()
        } else if entry.is_dir {
            name.cyan()
        } else {
            name.reset()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub is_hidden: bool,
}

pub struct ClientState {
    pub selected_idx: usize,
}
