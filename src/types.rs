use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceProject {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceGroup {
    pub id: u32,
    pub name: String,
    pub full_path: String,
}
