use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceProject {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub archived: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceGroup {
    pub id: u32,
    pub name: String,
    pub full_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceMember {
    pub id: u32,
    pub name: String,
    pub username: String,
}

pub enum Membership {
    Group(SourceGroup),
    Project(SourceProject),
}

impl Membership {
    pub fn url_prefix(&self) -> &'static str {
        match self {
            Self::Group(_) => "groups",
            Self::Project(_) => "projects",
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            Self::Group(x) => x.id,
            Self::Project(x) => x.id,
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Group(x) => x.full_path.to_string(),
            Self::Project(x) => x.path_with_namespace.to_string(),
        }
    }
}
