use crate::types::{
    CachedCiVariables, CachedIssues, CachedMemberships, CachedPipelineSchedules,
    CachedProjectMetadata, ExportStatus, Membership, SourceIssue, SourceMember,
    SourcePipelineSchedule, SourceProject, SourceUser, SourceVariable,
};
use crate::{gitlab, http};
use itertools::Itertools;
use std::collections::HashMap;
use std::error::Error;

// ---------------------------------------------------------------------------
// Create Target CI Variables
// ---------------------------------------------------------------------------
pub async fn create_target_ci_variables() -> Result<(), Box<dyn Error>> {
    let variables = std::fs::read_to_string("cache/ci_variables.json")?;
    let variables: CachedCiVariables = serde_json::from_str(&variables)?;

    let projects: HashMap<_, _> = gitlab::fetch_all_target_projects()
        .await?
        .into_iter()
        .map(|project| (project.key(), project))
        .collect();

    let futures: Vec<_> = variables
        .into_iter()
        .flat_map(|(key, vars)| {
            let pairs: Vec<_> = vars.into_iter().map(|v| (key.to_owned(), v)).collect();
            pairs
        })
        .filter_map(|(key, var)| {
            let project_option = projects.get(&key);
            project_option.map(|project| gitlab::create_target_ci_variable(var, project))
        })
        .collect();
    http::politely_try_join_all(futures, 8, 500).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Reassign Target Issues
// ---------------------------------------------------------------------------
pub async fn reassign_target_issues() -> Result<(), Box<dyn Error>> {
    let projects: HashMap<_, _> = gitlab::fetch_all_target_projects()
        .await?
        .into_iter()
        .map(|project| (project.key(), project))
        .collect();

    let users: HashMap<_, _> = gitlab::fetch_all_target_users()
        .await?
        .into_iter()
        .map(|user| (user.key(), user))
        .collect();

    let all_issues = std::fs::read_to_string("cache/issues.json")?;
    let all_issues: CachedIssues = serde_json::from_str(&all_issues)?;

    let futures: Vec<_> = all_issues
        .into_iter()
        .flat_map(|(key, issues)| match projects.get(&key) {
            Some(project) => {
                let pairs: Vec<_> = issues
                    .into_iter()
                    .take(20)
                    .filter_map(|issue| {
                        let assignee_username = issue
                            .assignee
                            .as_ref()
                            .map(|x| x.username.to_owned())
                            .unwrap_or_default();
                        let target_user_option = users.get(&assignee_username);
                        target_user_option
                            .map(|user| gitlab::reassign_target_issue(issue, project, user))
                    })
                    .collect();
                pairs
            }
            None => vec![],
        })
        .collect();
    http::politely_try_join_all(futures, 24, 500).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete Target Pipeline Schedules
// ---------------------------------------------------------------------------
pub async fn delete_target_pipeline_schedules() -> Result<(), Box<dyn Error>> {
    let projects: Vec<_> = gitlab::fetch_all_target_projects()
        .await?
        .into_iter()
        .collect();

    let futures: Vec<_> = projects
        .iter()
        .map(gitlab::delete_target_pipeline_schedules)
        .collect();
    http::politely_try_join_all(futures, 24, 500).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Create Target Pipeline Schedules
// ---------------------------------------------------------------------------
pub async fn create_target_pipeline_schedules() -> Result<(), Box<dyn Error>> {
    let projects: HashMap<_, _> = gitlab::fetch_all_target_projects()
        .await?
        .into_iter()
        .map(|project| (project.key(), project))
        .collect();

    let all_schedules = std::fs::read_to_string("cache/pipeline_schedules.json")?;
    let all_schedules: CachedPipelineSchedules = serde_json::from_str(&all_schedules)?;

    let futures: Vec<_> = all_schedules
        .into_iter()
        .filter_map(|(key, schedules)| {
            projects
                .get(&key)
                .map(|project| gitlab::create_target_pipeline_schedules(schedules, project))
        })
        .collect();
    http::politely_try_join_all(futures, 24, 500).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Add Target Users to Projects
// ---------------------------------------------------------------------------
pub async fn add_target_users_to_projects() -> Result<(), Box<dyn Error>> {
    let projects = gitlab::fetch_all_target_projects().await?;
    let project_ids: HashMap<_, _> = projects
        .into_iter()
        .map(|project| (project.key(), project))
        .collect();

    let users = gitlab::fetch_all_target_users().await?;
    let user_ids: HashMap<_, _> = users
        .into_iter()
        .map(|user| (user.username.clone(), user))
        .collect();

    let memberships = std::fs::read_to_string("cache/memberships.json")?;
    let mut memberships: CachedMemberships = serde_json::from_str(&memberships)?;
    let project_memberships = memberships.remove("projects").unwrap_or_default();

    let futures: Vec<_> = project_memberships
        .into_iter()
        .flat_map(|(project_key, members)| {
            members
                .into_iter()
                .map(move |member| (project_key.clone(), member))
        })
        .filter_map(|(project_key, member)| {
            let project_option = project_ids.get(&project_key);
            let user_option = user_ids.get(&member.username);
            match (project_option, user_option) {
                (Some(project), Some(user)) => Some(gitlab::add_target_project_member_to_project(
                    project.clone(),
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
// Add Target Users to Groups
// ---------------------------------------------------------------------------
pub async fn add_target_users_to_groups() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_target_groups().await?;
    let group_ids: HashMap<_, _> = groups
        .into_iter()
        .map(|group| (group.key(), group))
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
                (Some(group), Some(user)) => Some(gitlab::add_target_project_member_to_group(
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
    let email_mapping = std::fs::read_to_string("cache/username_email_mapping.json")?;
    let email_mapping: HashMap<String, String> = serde_json::from_str(&email_mapping)?;
    println!(
        "Using the following username-to-email mapping:\n{:#?}",
        email_mapping
    );

    let existing_users = gitlab::fetch_all_target_users().await?;
    let existing_usernames: Vec<_> = existing_users
        .into_iter()
        .map(|user| user.username)
        .collect();

    let users_to_create = load_users_to_create()?;
    let futures: Vec<_> = users_to_create
        .into_iter()
        .filter(|user| !existing_usernames.contains(&user.username))
        .map(|user| gitlab::create_target_user(user, &email_mapping))
        .collect();
    println!("Creating target users for {} users...", futures.len());
    http::politely_try_join_all(futures, 8, 500).await?;
    Ok(())
}

pub fn load_users_to_create() -> Result<Vec<SourceUser>, Box<dyn Error>> {
    let memberships = std::fs::read_to_string("cache/memberships.json")?;
    let memberships: CachedMemberships = serde_json::from_str(&memberships)?;
    let users_from_memberships = memberships
        .values()
        .flat_map(|member| member.values())
        .flatten()
        .map(|user| user.to_user());

    let issues = std::fs::read_to_string("cache/issues.json")?;
    let issues: CachedIssues = serde_json::from_str(&issues)?;
    let users_from_issues = issues.values().flat_map(|project_issues| {
        let users: Vec<_> = project_issues
            .iter()
            .flat_map(|issue| {
                let author = issue.author.clone();
                let assignee = issue.assignee.clone();
                match assignee {
                    Some(assignee) => vec![assignee, author],
                    None => vec![author],
                }
            })
            .collect();
        users
    });

    let users_to_create = users_from_memberships
        .chain(users_from_issues)
        .unique_by(|user| user.id)
        .collect();
    Ok(users_to_create)
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
// Download Source Pipeline Schedules
// ---------------------------------------------------------------------------
pub async fn download_source_pipeline_schedules() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let projects: Vec<_> = gitlab::fetch_all_source_projects(groups).await?;
    let futures: Vec<_> = projects
        .iter()
        .map(fetch_source_pipeline_schedules)
        .collect();
    let schedules: HashMap<_, _> = http::politely_try_join_all(futures, 24, 500)
        .await?
        .into_iter()
        .collect();
    save_source_pipeline_schedules(&schedules)?;
    Ok(())
}

fn save_source_pipeline_schedules(
    pipeline_schedules: &CachedPipelineSchedules,
) -> Result<(), Box<dyn Error>> {
    let dir_path = "cache";
    std::fs::create_dir_all(dir_path)?;
    let json_path = format!("{}/pipeline_schedules.json", dir_path);
    serde_json::to_writer_pretty(&std::fs::File::create(&json_path)?, &pipeline_schedules)?;
    println!("Successfully wrote to {}!", json_path);
    Ok(())
}

pub async fn fetch_source_pipeline_schedules(
    project: &SourceProject,
) -> Result<(String, Vec<SourcePipelineSchedule>), Box<dyn Error>> {
    let key = project.key();
    let schedules = gitlab::fetch_source_pipeline_schedules(project).await?;
    Ok((key, schedules))
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
    println!("Downloading project id {}...", project_id);
    let mut status = gitlab::fetch_export_status(project_id).await?;
    if status.export_status == "none" {
        println!("Skipping: {:?}", status);
        return Ok(());
    }
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

// ---------------------------------------------------------------------------
// Archive Source Projects
// ---------------------------------------------------------------------------
pub async fn archive_source_projects() -> Result<(), Box<dyn Error>> {
    let groups = gitlab::fetch_all_source_groups().await?;
    let projects: Vec<_> = gitlab::fetch_all_source_projects(groups)
        .await?
        .into_iter()
        .filter(|project| !project.archived)
        .collect();

    for (index, project) in projects.iter().enumerate() {
        gitlab::archive_source_project(project.id).await?;
        println!("Completed ({}/{}) requests!", index + 1, projects.len());
        http::throttle_for_ms(1000);
    }
    Ok(())
}
