use alloc::{string::String, vec::Vec};

// #[derive(Clone, Serialize, Deserialize, Debug)]
pub trait FileSystem {
    fn dir_entries(&self, dir: &str) -> Vec<File>;
    fn open(&self, path: &str) -> Option<File>;
    fn read(&self, file: &File, buffer: &mut [u8]);
}

#[derive(Default, Debug, Clone)]
pub struct File {
    pub lfn: String,
    pub name: String,
    pub r#type: String,
    pub size: u32,
    pub first_cluster_high: u16,
    pub first_cluster_low: u16,
}
