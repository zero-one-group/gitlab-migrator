use crate::types::{
    CachedCiVariables, CachedIssues, CachedMemberships, CachedProjectMetadata, ExportStatus,
    Membership, SourceIssue, SourceMember, SourceProject, SourceVariable,
};
use crate::{gitlab, http};
use itertools::Itertools;
use std::collections::HashMap;
use std::error::Error;

// TODO: download source issues
// TODO: reassign target issues

// ---------------------------------------------------------------------------
// Add Target Users to Groups
// ---------------------------------------------------------------------------
pub async fn add_target_users_to_groups() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_target_groups().await?;
    let group_ids: HashMap<_, _> = groups
        .into_iter()
        .map(|group| (group.full_path.clone(), group))
        .collect();

    let users = gitlab::fetch_all_target_users().await?;
    let user_ids: HashMap<_, _> = users
        .into_iter()
        .map(|user| (user.username.clone(), user))
        .collect();

    let memberships = std::fs::read_to_string("cache/memberships.json")?;
    let mut memberships: CachedMemberships = serde_json::from_str(&memberships)?;
    let group_memberships = memberships.remove("groups").unwrap_or_default();

    let futures: Vec<_> = group_memberships
        .into_iter()
        .flat_map(|(group_path, members)| {
            members
                .into_iter()
                .map(move |member| (group_path.clone(), member))
        })
        .filter_map(|(group_path, member)| {
            let group_option = group_ids.get(&group_path);
            let user_option = user_ids.get(&member.username);
            match (group_option, user_option) {
                (Some(group), Some(user)) => Some(gitlab::add_target_project_member(
                    group.clone(),
                    user.clone(),
                    member,
                )),
                _ => None,
            }
        })
        .collect();
    http::politely_try_join_all(futures, 8, 500).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Delete Target Projects
// ---------------------------------------------------------------------------
pub async fn delete_target_projects() -> Result<(), Box<dyn Error>> {
    let metadata = std::fs::read_to_string("cache/project_metadata.json")?;
    let metadata: CachedProjectMetadata = serde_json::from_str(&metadata)?;
    let project_paths: Vec<_> = metadata
        .values()
        .map(|project| project.path_with_namespace.to_string())
        .collect();

    let all_projects = gitlab::fetch_all_target_projects().await?;
    let futures: Vec<_> = all_projects
        .into_iter()
        .filter(|project| project_paths.contains(&project.path_with_namespace))
        .map(gitlab::delete_target_project)
        .collect();
    http::politely_try_join_all(futures, 8, 500).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Import Target Projects
// ---------------------------------------------------------------------------
pub async fn import_target_projects() -> Result<(), Box<dyn Error>> {
    let metadata = std::fs::read_to_string("cache/project_metadata.json")?;
    let metadata: CachedProjectMetadata = serde_json::from_str(&metadata)?;

    let existing_projects = gitlab::fetch_all_target_projects().await?;
    let existing_paths: Vec<_> = existing_projects
        .into_iter()
        .map(|project| project.path_with_namespace)
        .collect();

    let remaining_projects: Vec<_> = metadata
        .into_iter()
        .map(|(_, project)| project)
        .filter(|project| !existing_paths.contains(&project.path_with_namespace))
        .collect();
    let num_remaining = remaining_projects.len();
    for (index, project) in remaining_projects.into_iter().enumerate() {
        let _ = gitlab::import_target_project(project).await;
        println!("Num. remaining projects: {}", num_remaining - index - 1);
        http::throttle_for_ms(10 * 1000);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete Target Users
// ---------------------------------------------------------------------------
pub async fn delete_target_users() -> Result<(), Box<dyn Error>> {
    let memberships = std::fs::read_to_string("cache/memberships.json")?;
    let memberships: CachedMemberships = serde_json::from_str(&memberships)?;
    let usernames: Vec<_> = memberships
        .values()
        .flat_map(|user| user.values())
        .flatten()
        .unique_by(|user| user.id)
        .map(|user| user.username.to_string())
        .collect();

    let all_target_users = gitlab::fetch_all_target_users().await?;
    let futures: Vec<_> = all_target_users
        .into_iter()
        .filter(|user| usernames.contains(&user.username))
        .map(gitlab::delete_target_user)
        .collect();
    http::politely_try_join_all(futures, 8, 500).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Create Target Users
// ---------------------------------------------------------------------------
pub async fn create_target_users() -> Result<(), Box<dyn Error>> {
    let memberships = std::fs::read_to_string("cache/memberships.json")?;
    let memberships: CachedMemberships = serde_json::from_str(&memberships)?;

    let email_mapping = std::fs::read_to_string("cache/username_email_mapping.json")?;
    let email_mapping: HashMap<String, String> = serde_json::from_str(&email_mapping)?;

    let existing_users = gitlab::fetch_all_target_users().await?;
    let existing_usernames: Vec<_> = existing_users
        .into_iter()
        .map(|user| user.username)
        .collect();

    let futures: Vec<_> = memberships
        .values()
        .flat_map(|member| member.values())
        .flatten()
        .unique_by(|member| member.id)
        .filter(|member| !existing_usernames.contains(&member.username))
        .map(|member| gitlab::create_target_user(member.to_user(), &email_mapping))
        .collect();
    println!("Creating target users for {} users...", futures.len());
    http::politely_try_join_all(futures, 8, 500).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Download Source CI Variables
// ---------------------------------------------------------------------------
pub async fn download_source_ci_variables() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let projects: Vec<_> = gitlab::fetch_all_source_projects(groups).await?;
    let futures: Vec<_> = projects.iter().map(fetch_source_ci_variables).collect();
    let pairs = http::politely_try_join_all(futures, 24, 500).await?;
    let all_ci_variables: HashMap<_, _> = pairs.into_iter().collect();
    save_ci_variables(&all_ci_variables)?;
    Ok(())
}

fn save_ci_variables(ci_variables: &CachedCiVariables) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/ci_variables.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &ci_variables)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

pub async fn fetch_source_ci_variables(
    project: &SourceProject,
) -> Result<(String, Vec<SourceVariable>), Box<dyn Error>> {
    let key = project.key();
    let variables = gitlab::fetch_source_ci_variables(project).await?;
    Ok((key, variables))
}

// ---------------------------------------------------------------------------
// Download Source Issues
// ---------------------------------------------------------------------------
pub async fn download_source_issues() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let projects: Vec<_> = gitlab::fetch_all_source_projects(groups).await?;

    let futures: Vec<_> = projects.into_iter().map(fetch_all_source_issues).collect();
    let issues: HashMap<_, _> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .collect();
    save_source_issues(&issues)?;
    Ok(())
}

fn save_source_issues(issues: &CachedIssues) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/issues.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &issues)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

pub async fn fetch_all_source_issues(
    project: SourceProject,
) -> Result<(String, Vec<SourceIssue>), Box<dyn Error>> {
    let key = project.key();
    let issues = gitlab::fetch_all_source_issues(&project).await?;
    Ok((key, issues))
}

// ---------------------------------------------------------------------------
// Download Source Projects
// ---------------------------------------------------------------------------
pub async fn download_source_projects() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let projects: Vec<_> = gitlab::fetch_all_source_projects(groups)
        .await?
        .into_iter()
        .filter(|project| {
            let dir_path = "cache/projects";
            let gz_path = format!("{}/{}.gz", dir_path, project.id);
            !std::path::Path::new(&gz_path).exists()
        })
        .collect();

    for (index, project) in projects.iter().enumerate() {
        gitlab::send_export_request(project.id).await?;
        println!("Completed ({}/{}) requests!", index + 1, projects.len());
        http::throttle_for_ms(15 * 1000);
    }

    for (index, project) in projects.iter().enumerate() {
        wait_and_save_project_gz(project.id).await?;
        println!("Completed ({}/{}) downloads!", index + 1, projects.len());
        http::throttle_for_ms(60 * 1000);
    }
    Ok(())
}

pub async fn wait_and_save_project_gz(project_id: u32) -> Result<(), Box<dyn Error>> {
    let mut status = gitlab::fetch_export_status(project_id).await?;
    while status.export_status != "finished" {
        println!("Waiting for the following to complete: {:?}", status);
        http::throttle_for_ms(15 * 1000);
        status = gitlab::fetch_export_status(project_id).await?;
    }
    download_project_gz(&status).await?;
    println!("Exported project saved! {:?}", status);
    Ok(())
}

pub async fn download_project_gz(status: &ExportStatus) -> Result<(), Box<dyn Error>> {
    let response = gitlab::download_source_project_gz(status).await?;
    let dir_path = "cache/projects";
    std::fs::create_dir_all(dir_path)?;
    let gz_path = format!("{}/{}.gz", dir_path, status.id);
    let mut file = std::fs::File::create(gz_path)?;
    let mut content = std::io::Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Download Source Project Metadata
// ---------------------------------------------------------------------------
pub async fn download_source_project_metadata() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let projects: Vec<_> = gitlab::fetch_all_source_projects(groups).await?;

    let project_metadata: HashMap<_, _> = projects
        .into_iter()
        .map(|project| (project.id, project))
        .collect();
    save_source_project_metadata(&project_metadata)?;
    Ok(())
}

fn save_source_project_metadata(
    project_metadata: &CachedProjectMetadata,
) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/project_metadata.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &project_metadata)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

// ---------------------------------------------------------------------------
// Download Source Memberships
// ---------------------------------------------------------------------------
pub async fn download_source_memberships() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let futures: Vec<_> = groups
        .iter()
        .map(|group| fetch_source_members(Membership::Group(group.clone())))
        .collect();
    let group_members: HashMap<_, _> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .collect();

    let projects = gitlab::fetch_all_source_projects(groups).await?;
    let futures: Vec<_> = projects
        .into_iter()
        .map(|project| fetch_source_members(Membership::Project(project)))
        .collect();
    let project_members: HashMap<_, _> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .collect();

    let all_memberships = HashMap::from([
        ("groups".to_string(), group_members),
        ("projects".to_string(), project_members),
    ]);
    save_source_memberships(&all_memberships)?;
    Ok(())
}

fn save_source_memberships(memberships: &CachedMemberships) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/memberships.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &memberships)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

pub async fn fetch_source_members(
    membership: Membership,
) -> Result<(String, Vec<SourceMember>), Box<dyn Error>> {
    let key = membership.key();
    let members = gitlab::fetch_source_members(membership).await?;
    Ok((key, members))
}
