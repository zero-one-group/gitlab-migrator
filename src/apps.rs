use crate::types::{
    ExportStatus, Membership, SourceGroup, SourceMember, SourceProject, SourceVariable,
};
use crate::{env, http};
use itertools::Itertools;
use std::collections::HashMap;
use std::error::Error;

// ---------------------------------------------------------------------------
// Create Users
// ---------------------------------------------------------------------------
pub async fn create_all_target_users() -> Result<(), Box<dyn Error>> {
    let memberships = std::fs::read_to_string("cache/memberships.json")?;
    let memberships: AllMemberships = serde_json::from_str(&memberships)?;

    let all_members: Vec<_> = memberships
        .values()
        .flat_map(|x| x.values())
        .flatten()
        .unique_by(|x| x.id)
        .collect();

    println!("{:#?}", all_members);
    println!("{:#?}", all_members.len());

    // TODO: organise CLI into apps
    // TODO: add email, avatar to SourceMember

    //let gitlab_url = env::load_env("TARGET_GITLAB_URL");
    //let token = env::load_env("TARGET_GITLAB_TOKEN");
    //let url = format!("{}/groups/", gitlab_url);
    //let response = http::CLIENT
    //.get(url)
    //.query(&[("per_page", "100")])
    //.header("PRIVATE-TOKEN", token)
    //.send()
    //.await?;
    //let payload = &response.text().await?;
    //let groups: Vec<SourceGroup> = serde_json::from_str(payload)?;
    //println!("{:#?}", groups);
    Ok(())
}

// ---------------------------------------------------------------------------
// Fetch All CI Variables
// ---------------------------------------------------------------------------
type AllCiVariables = HashMap<String, Vec<SourceVariable>>;

pub async fn fetch_and_save_all_ci_variables() -> Result<AllCiVariables, Box<dyn Error>> {
    let groups = fetch_all_source_groups().await?;
    let projects: Vec<_> = fetch_all_source_projects(groups).await?;
    let futures: Vec<_> = projects.iter().map(fetch_ci_variables).collect();
    let pairs = http::politely_try_join_all(futures, 24, 500).await?;
    let all_ci_variables: HashMap<_, _> = pairs.into_iter().collect();
    save_ci_variables(&all_ci_variables)?;
    Ok(all_ci_variables.into_iter().collect())
}

fn save_ci_variables(ci_variables: &AllCiVariables) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/ci_variables.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &ci_variables)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

pub async fn fetch_ci_variables(
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

// ---------------------------------------------------------------------------
// Fetch All Exported Projects
// ---------------------------------------------------------------------------
pub async fn wait_and_save_all_project_zips() -> Result<(), Box<dyn Error>> {
    let groups = fetch_all_source_groups().await?;
    let projects: Vec<_> = fetch_all_source_projects(groups)
        .await?
        .into_iter()
        .filter(|project| {
            let dir_path = "cache/projects";
            let zip_path = format!("{}/{}.zip", dir_path, project.id);
            !std::path::Path::new(&zip_path).exists()
        })
        .collect();

    for (index, project) in projects.iter().enumerate() {
        send_export_request(project.id).await?;
        println!("Completed ({}/{}) requests!", index + 1, projects.len());
        http::throttle_for_ms(15 * 1000);
    }

    for (index, project) in projects.iter().enumerate() {
        wait_and_save_project_zip(project.id).await?;
        println!("Completed ({}/{}) downloads!", index + 1, projects.len());
        http::throttle_for_ms(60 * 1000);
    }
    Ok(())
}

pub async fn wait_and_save_project_zip(project_id: u32) -> Result<(), Box<dyn Error>> {
    let mut status = fetch_export_status(project_id).await?;
    while status.export_status != "finished" {
        println!("Waiting for the following to complete: {:?}", status);
        http::throttle_for_ms(15 * 1000);
        status = fetch_export_status(project_id).await?;
    }
    download_project_zip(&status).await?;
    println!("Exported project saved! {:?}", status);
    Ok(())
}

pub async fn download_project_zip(status: &ExportStatus) -> Result<(), Box<dyn Error>> {
    let gitlab_url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let url = format!("{}/projects/{}/export/download", gitlab_url, status.id);
    let response = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?
        .error_for_status()?;

    let dir_path = "cache/projects";
    std::fs::create_dir_all(dir_path)?;
    let zip_path = format!("{}/{}.zip", dir_path, status.id);
    let mut file = std::fs::File::create(zip_path)?;
    let mut content = std::io::Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
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

// ---------------------------------------------------------------------------
// Fetch All Memberships
// ---------------------------------------------------------------------------
type AllMemberships = HashMap<String, HashMap<String, Vec<SourceMember>>>;

pub async fn fetch_and_save_all_memberships() -> Result<AllMemberships, Box<dyn Error>> {
    let groups = fetch_all_source_groups().await?;
    let futures: Vec<_> = groups
        .iter()
        .map(|group| fetch_members(Membership::Group(group.clone())))
        .collect();
    let group_members: HashMap<_, _> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .collect();

    let projects = fetch_all_source_projects(groups).await?;
    let futures: Vec<_> = projects
        .into_iter()
        .map(|project| fetch_members(Membership::Project(project)))
        .collect();
    let project_members: HashMap<_, _> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .collect();

    let all_memberships = HashMap::from([
        ("groups".to_string(), group_members),
        ("projects".to_string(), project_members),
    ]);
    save_memberships(&all_memberships)?;
    Ok(all_memberships)
}

fn save_memberships(memberships: &AllMemberships) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/memberships.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &memberships)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

pub async fn fetch_members(
    membership: Membership,
) -> Result<(String, Vec<SourceMember>), Box<dyn Error>> {
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
    Ok((membership.key(), members))
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
