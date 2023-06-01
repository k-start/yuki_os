use alloc::{string::String, vec::Vec};

// #[derive(Clone, Serialize, Deserialize, Debug)]
pub trait FileSystem {
    fn dir_entries(&self, dir: &str) -> Vec<File>;
    fn open(&self, path: &str) -> File;
}

#[derive(Default, Debug, Clone)]
pub struct File {
    pub name: String,
    pub r#type: String,
    pub size: u32,
}
