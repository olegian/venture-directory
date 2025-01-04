use std::rc::Rc;

use crate::constants;

pub struct Config {
    pub max_path_length: usize,
    pub max_directory_depth: usize,
    pub n_rows: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_directory_depth: Default::default(),
            n_rows: Default::default(),
            max_path_length: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Directory {
    pub name: String,
    pub contents: Vec<Entry>,
    pub selected_idx: usize,
}

impl Directory {
    pub fn new(name: String, contents: Vec<Entry>, selected_idx: usize) -> Self {
        Self {
            name: name,
            contents: contents,
            selected_idx: selected_idx,
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
