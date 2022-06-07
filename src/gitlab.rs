use crate::types::{
    ExportStatus, Membership, SourceGroup, SourceProject, SourceUser, SourceVariable,
    TargetProject, TargetUser,
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

// TODO: wire up SoureProject to import_target_project
// TODO: dry run

pub async fn delete_target_project(project: TargetProject) -> Result<(), Box<dyn Error>> {
    println!("Deleting project {:?}...", project);
    let url = format!("{}/projects/{}", *TARGET_GITLAB_URL, project.id);
    http::CLIENT
        .delete(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn fetch_all_target_projects() -> Result<Vec<TargetProject>, Box<dyn Error>> {
    let mut all_projects = vec![];
    let mut latest_page = 1;
    let mut latest_len = 0;
    while latest_len == 100 || latest_page == 1 {
        let mut projects = fetch_target_projects(latest_page).await?;
        latest_len = projects.len();
        latest_page += 1;
        all_projects.append(&mut projects);
    }
    Ok(all_projects)
}

pub async fn fetch_target_projects(page: u32) -> Result<Vec<TargetProject>, Box<dyn Error>> {
    let url = format!("{}/projects", *TARGET_GITLAB_URL);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?;
    let payload = &response.text().await?;
    let projects: Vec<TargetProject> = serde_json::from_str(payload)?;
    Ok(projects)
}

pub async fn import_target_project() -> Result<(), String> {
    let spawn_result =
        tokio::task::spawn_blocking(move || match synchronous_import_target_project() {
            Ok(x) => Ok(x),
            Err(_) => Err("Failed to import target project!".to_owned()),
        })
        .await;
    spawn_result.map_err(|_| "Spawn blocking failed!".to_string())?
}

pub fn synchronous_import_target_project() -> Result<(), Box<dyn Error>> {
    let project_gz_path = "cache/projects/20076483.gz";
    let form = reqwest::blocking::multipart::Form::new()
        .text("namespace", "zo-group/software")
        .text("name", "infra")
        .text("path", "infra")
        .file("file", project_gz_path)?;

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/projects/import", *TARGET_GITLAB_URL);
    let response = client
        .post(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .multipart(form)
        .send()?;

    let payload = response.text()?;
    println!("{}", payload);

    Ok(())
}

pub async fn fetch_all_target_users() -> Result<Vec<TargetUser>, Box<dyn Error>> {
    let mut all_users = vec![];
    let mut latest_page = 1;
    let mut latest_len = 0;
    while latest_len == 100 || latest_page == 1 {
        let mut users = fetch_target_users(latest_page).await?;
        latest_len = users.len();
        latest_page += 1;
        all_users.append(&mut users);
    }
    Ok(all_users)
}

pub async fn fetch_target_users(page: u32) -> Result<Vec<TargetUser>, Box<dyn Error>> {
    let url = format!("{}/users", *TARGET_GITLAB_URL);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?;
    let payload = &response.text().await?;
    let users: Vec<TargetUser> = serde_json::from_str(payload)?;
    Ok(users)
}

pub async fn delete_target_user(user: TargetUser) -> Result<(), Box<dyn Error>> {
    println!("Deleting user {:?}...", user);
    let url = format!("{}/users/{}", *TARGET_GITLAB_URL, user.id);
    http::CLIENT
        .delete(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn create_target_user(user: SourceUser) -> Result<TargetUser, String> {
    let user_str = format!("{:?}", user);
    let spawn_result =
        tokio::task::spawn_blocking(move || match synchronous_create_target_user(user) {
            Ok(x) => Ok(x),
            Err(_) => Err(format!("Failed to create {:?}.", user_str)),
        })
        .await;
    spawn_result.map_err(|_| "Spawn blocking failed!".to_string())?
}

pub fn synchronous_create_target_user(user: SourceUser) -> Result<TargetUser, Box<dyn Error>> {
    println!("Creating user {:?}...", user);
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

    let response = client
        .post(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .multipart(form)
        .send()?;

    let payload = response.text()?;
    let member: TargetUser = serde_json::from_str(&payload)?;
    Ok(member)
}

pub fn synchronous_download_avatar(user: &SourceUser) -> Result<String, Box<dyn Error>> {
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

pub async fn download_source_project_gz(status: &ExportStatus) -> Result<Response, Box<dyn Error>> {
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
) -> Result<Vec<SourceUser>, Box<dyn Error>> {
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
    let members: Vec<SourceUser> = serde_json::from_str(payload)?;
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
