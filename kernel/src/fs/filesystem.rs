use alloc::{string::String, vec::Vec};

// #[derive(Clone, Serialize, Deserialize, Debug)]
pub trait FileSystem {
    fn dir_entries(&self, dir: &str) -> Vec<File>;
    fn open(&self, path: &str) -> File;
}

pub struct File {
    pub name: String,
    pub size: i32,
}
