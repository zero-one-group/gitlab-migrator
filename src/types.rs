use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceProject {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub archived: bool,
}

impl SourceProject {
    pub fn key(&self) -> String {
        self.path_with_namespace.to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceGroup {
    pub id: u32,
    pub name: String,
    pub full_path: String,
}

impl SourceGroup {
    pub fn key(&self) -> String {
        self.full_path.to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceUser {
    pub id: u32,
    pub name: String,
    pub username: String,
    pub avatar_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceMember {
    pub id: u32,
    pub name: String,
    pub username: String,
    pub avatar_url: String,
    pub access_level: u32,
}

impl SourceMember {
    pub fn to_user(&self) -> SourceUser {
        SourceUser {
            id: self.id,
            name: self.name.to_owned(),
            username: self.username.to_owned(),
            avatar_url: self.avatar_url.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceIssue {
    pub iid: u32,
    pub title: String,
    pub author: SourceUser,
    pub assignee: Option<SourceUser>,
    pub project_id: u32,
    pub labels: Vec<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceVariable {
    pub variable_type: String,
    pub key: String,
    pub value: String,
    pub protected: bool,
    pub masked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourcePipelineScheduleWithoutVariables {
    pub id: u32,
    pub description: String,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub ref_: String,
    pub cron: String,
    pub cron_timezone: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourcePipelineSchedule {
    pub id: u32,
    pub description: String,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub ref_: String,
    pub cron: String,
    pub cron_timezone: String,
    pub active: bool,
    pub variables: Option<Vec<SourcePipelineVariable>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourcePipelineVariable {
    pub variable_type: String,
    pub key: String,
    pub value: String,
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
            Self::Group(x) => x.key(),
            Self::Project(x) => x.key(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportStatus {
    pub id: u32,
    pub path_with_namespace: String,
    pub export_status: String,
}

pub type CachedProjectMetadata = HashMap<u32, SourceProject>;
pub type CachedCiVariables = HashMap<String, Vec<SourceVariable>>;
pub type CachedMemberships = HashMap<String, HashMap<String, Vec<SourceMember>>>;
pub type CachedIssues = HashMap<String, Vec<SourceIssue>>;
pub type CachedPipelineSchedules = HashMap<String, Vec<SourcePipelineSchedule>>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TargetUser {
    pub id: u32,
    pub name: String,
    pub username: String,
    pub email: String,
}

impl TargetUser {
    pub fn key(&self) -> String {
        self.username.to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TargetProject {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub archived: bool,
}

impl TargetProject {
    pub fn key(&self) -> String {
        self.path_with_namespace.to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TargetGroup {
    pub id: u32,
    pub name: String,
    pub full_path: String,
}

impl TargetGroup {
    pub fn key(&self) -> String {
        self.full_path.to_string()
    }
}
