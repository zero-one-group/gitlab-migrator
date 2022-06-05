use crate::types::{
    ExportStatus, Membership, SourceGroup, SourceMember, SourceProject, SourceVariable,
};
use crate::{env, http};
use reqwest::Response;
use std::error::Error;

lazy_static::lazy_static! {
    pub static ref SOURCE_GITLAB_URL: String = env::load_env("SOURCE_GITLAB_URL");
    pub static ref SOURCE_GITLAB_TOKEN: String = env::load_env("SOURCE_GITLAB_TOKEN");
    pub static ref TARGET_GITLAB_URL: String = env::load_env("TARGET_GITLAB_URL");
    pub static ref TARGET_GITLAB_TOKEN: String = env::load_env("TARGET_GITLAB_TOKEN");
}

pub async fn create_target_user(user: SourceMember) -> Result<String, String> {
    let user_str = format!("{:?}", user);
    let spawn_result =
        tokio::task::spawn_blocking(move || match synchronous_create_target_user(user) {
            Ok(_) => Ok(format!("Successfully created {:?}.", user_str)),
            Err(_) => Err(format!("Failed to create {:?}.", user_str)),
        })
        .await;
    spawn_result.map_err(|_| "Spawn blocking failed!".to_string())?
}

pub fn synchronous_create_target_user(user: SourceMember) -> Result<(), Box<dyn Error>> {
    println!("Creating user {}...", user.username);
    let avatar = synchronous_download_avatar(&user)?;
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/users", *TARGET_GITLAB_URL);
    let email = format!("{}@example.com", user.username); // FIXME: use real emails
    let form = reqwest::blocking::multipart::Form::new()
        .text("name", user.name)
        .text("username", user.username)
        .text("email", email)
        .text("force_random_password", "true")
        //.text("reset_password", "true") // TODO: uncomment
        .text("skip_confirmation", "true")
        .file("avatar", avatar)?;

    client
        .post(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .multipart(form)
        .send()?;
    Ok(())
}

pub fn synchronous_download_avatar(user: &SourceMember) -> Result<String, Box<dyn Error>> {
    println!("Downloading avatar for {}...", user.username);
    let client = reqwest::blocking::Client::new();
    let response = client.get(&user.avatar_url).send()?;
    let dir_path = "cache/avatars";
    std::fs::create_dir_all(dir_path)?;
    let png_path = format!("{}/{}.png", dir_path, user.username);
    let mut file = std::fs::File::create(&png_path)?;
    let mut content = std::io::Cursor::new(response.bytes()?);
    std::io::copy(&mut content, &mut file)?;
    Ok(png_path)
}

pub async fn fetch_source_ci_variables(
    project: &SourceProject,
) -> Result<Vec<SourceVariable>, Box<dyn Error>> {
    let url = format!("{}/projects/{}/variables", *SOURCE_GITLAB_URL, project.id);
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    if response.status().is_success() {
        let payload = &response.text().await?;
        let variables: Vec<SourceVariable> = serde_json::from_str(payload)?;
        Ok(variables)
    } else {
        Ok(vec![])
    }
}

pub async fn download_source_project_zip(
    status: &ExportStatus,
) -> Result<Response, Box<dyn Error>> {
    let url = format!(
        "{}/projects/{}/export/download",
        *SOURCE_GITLAB_URL, status.id
    );
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?;
    Ok(response)
}

pub async fn send_export_request(project_id: u32) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/projects/{}/export", *SOURCE_GITLAB_URL, project_id);
    http::CLIENT
        .post(url)
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?;
    println!("Requested export for project ID {}!", project_id);
    Ok(())
}

pub async fn fetch_export_status(project_id: u32) -> Result<ExportStatus, Box<dyn Error>> {
    let url = format!("{}/projects/{}/export", *SOURCE_GITLAB_URL, project_id);
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    let payload = &response.text().await?;
    let status: ExportStatus = serde_json::from_str(payload)?;
    Ok(status)
}

pub async fn fetch_source_members(
    membership: Membership,
) -> Result<Vec<SourceMember>, Box<dyn Error>> {
    let url = format!(
        "{}/{}/{}/members",
        *SOURCE_GITLAB_URL,
        membership.url_prefix(),
        membership.id()
    );
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100")])
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    let payload = &response.text().await?;
    let members: Vec<SourceMember> = serde_json::from_str(payload)?;
    Ok(members)
}

pub async fn fetch_all_source_projects(
    groups: Vec<SourceGroup>,
) -> Result<Vec<SourceProject>, Box<dyn Error>> {
    let futures: Vec<_> = groups
        .into_iter()
        .map(|group| fetch_all_source_groups_projects(group.id))
        .collect();
    let projects: Vec<_> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .flatten()
        .collect();
    Ok(projects)
}

pub async fn fetch_all_source_groups_projects(
    group_id: u32,
) -> Result<Vec<SourceProject>, Box<dyn Error>> {
    let mut all_projects = vec![];
    let mut latest_page = 1;
    let mut latest_len = 0;
    while latest_len == 100 || latest_page == 1 {
        let mut projects = fetch_source_groups_projects(group_id, latest_page).await?;
        latest_len = projects.len();
        latest_page += 1;
        all_projects.append(&mut projects);
    }
    Ok(all_projects)
}

pub async fn fetch_source_groups_projects(
    group_id: u32,
    page: u32,
) -> Result<Vec<SourceProject>, Box<dyn Error>> {
    let url = format!("{}/groups/{}/projects", *SOURCE_GITLAB_URL, group_id);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    let payload = &response.text().await?;
    let projects: Vec<SourceProject> = serde_json::from_str(payload)?;
    Ok(projects)
}
pub async fn fetch_all_source_groups() -> Result<Vec<SourceGroup>, Box<dyn Error>> {
    let mut all_groups = vec![];
    let mut latest_page = 1;
    let mut latest_len = 0;
    while latest_len == 100 || latest_page == 1 {
        let mut groups = fetch_source_groups(latest_page).await?;
        latest_len = groups.len();
        latest_page += 1;
        all_groups.append(&mut groups);
    }
    Ok(all_groups)
}

async fn fetch_source_groups(page: u32) -> Result<Vec<SourceGroup>, Box<dyn Error>> {
    let url = format!("{}/groups/", *SOURCE_GITLAB_URL);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    let payload = &response.text().await?;
    let groups: Vec<SourceGroup> = serde_json::from_str(payload)?;
    Ok(groups)
}
