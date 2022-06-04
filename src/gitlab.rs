use crate::types::{
    ExportStatus, Membership, SourceGroup, SourceMember, SourceProject, SourceVariable,
};
use crate::{env, http};
use reqwest::Response;
use std::error::Error;

pub async fn fetch_source_ci_variables(
    project: &SourceProject,
) -> Result<(String, Vec<SourceVariable>), Box<dyn Error>> {
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/projects/{}/variables", gitlab_url, project.id);
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?;
    if response.status().is_success() {
        let payload = &response.text().await?;
        let variables: Vec<SourceVariable> = serde_json::from_str(payload)?;
        Ok((project.key(), variables))
    } else {
        Ok((project.key(), vec![]))
    }
}

pub async fn download_source_project_zip(
    status: &ExportStatus,
) -> Result<Response, Box<dyn Error>> {
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/projects/{}/export/download", gitlab_url, status.id);
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?
        .error_for_status()?;
    Ok(response)
}

pub async fn send_export_request(project_id: u32) -> Result<(), Box<dyn Error>> {
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/projects/{}/export", gitlab_url, project_id);
    http::CLIENT
        .post(url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?
        .error_for_status()?;
    println!("Requested export for project ID {}!", project_id);
    Ok(())
}

pub async fn fetch_export_status(project_id: u32) -> Result<ExportStatus, Box<dyn Error>> {
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/projects/{}/export", gitlab_url, project_id);
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?;
    let payload = &response.text().await?;
    let status: ExportStatus = serde_json::from_str(payload)?;
    Ok(status)
}

pub async fn fetch_source_members(
    membership: Membership,
) -> Result<Vec<SourceMember>, Box<dyn Error>> {
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!(
        "{}/{}/{}/members",
        gitlab_url,
        membership.url_prefix(),
        membership.id()
    );
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100")])
        .header("PRIVATE-TOKEN", token)
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
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/groups/{}/projects", gitlab_url, group_id);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", token)
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
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/groups/", gitlab_url);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?;
    let payload = &response.text().await?;
    let groups: Vec<SourceGroup> = serde_json::from_str(payload)?;
    Ok(groups)
}
