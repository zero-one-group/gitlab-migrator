use crate::types::{
    ExportStatus, Membership, SourceGroup, SourceIssue, SourceMember, SourcePipelineSchedule,
    SourcePipelineScheduleWithoutVariables, SourceProject, SourceUser, SourceVariable, TargetGroup,
    TargetPipelineSchedule, TargetProject, TargetUser,
};
use crate::{env, http};
use reqwest::Response;
use std::collections::HashMap;
use std::error::Error;

lazy_static::lazy_static! {
    pub static ref SOURCE_GITLAB_URL: String = env::load_env("SOURCE_GITLAB_URL");
    pub static ref SOURCE_GITLAB_TOKEN: String = env::load_env("SOURCE_GITLAB_TOKEN");
    pub static ref TARGET_GITLAB_URL: String = env::load_env("TARGET_GITLAB_URL");
    pub static ref TARGET_GITLAB_TOKEN: String = env::load_env("TARGET_GITLAB_TOKEN");
}

pub async fn delete_target_pipeline_schedules(
    project: &TargetProject,
) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "{}/projects/{}/pipeline_schedules",
        *TARGET_GITLAB_URL, project.id
    );
    let payload = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let schedules: Vec<TargetPipelineSchedule> = serde_json::from_str(&payload)?;

    for schedule in schedules {
        println!(
            "Deleting pipeline schedule '{}' in {}...",
            schedule.description,
            project.key()
        );
        let url = format!(
            "{}/projects/{}/pipeline_schedules/{}",
            *TARGET_GITLAB_URL, project.id, schedule.id
        );
        http::CLIENT
            .delete(url)
            .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
            .send()
            .await?;
    }
    Ok(())
}

pub async fn create_target_ci_variable(
    variable: SourceVariable,
    project: &TargetProject,
) -> Result<(), Box<dyn Error>> {
    println!("Creating variable {} in {}...", variable.key, project.key());
    let url = format!("{}/projects/{}/variables", *TARGET_GITLAB_URL, project.id);
    let response = http::CLIENT
        .post(url)
        .form(&[
            ("key", variable.key),
            ("value", variable.value),
            ("variable_type", variable.variable_type),
            ("protected", variable.protected.to_string()),
            ("masked", variable.masked.to_string()),
        ])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?;
    if let Err(err) = response.error_for_status() {
        println!("{}", err);
    }
    Ok(())
}

pub async fn create_target_pipeline_schedules(
    schedules: Vec<SourcePipelineSchedule>,
    project: &TargetProject,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Creating schedules {:#?} in {}...",
        schedules,
        project.key()
    );
    for schedule in schedules {
        let url = format!(
            "{}/projects/{}/pipeline_schedules",
            *TARGET_GITLAB_URL, project.id
        );
        let result = http::CLIENT
            .post(url)
            .form(&[
                ("description", schedule.description),
                ("ref", schedule.ref_),
                ("cron", schedule.cron),
                ("cron_timezone", schedule.cron_timezone),
                ("active", schedule.active.to_string()),
            ])
            .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
            .send()
            .await?
            .error_for_status();
        match result {
            Ok(response) => {
                let payload = response.text().await?;
                let created: TargetPipelineSchedule = serde_json::from_str(&payload)?;
                println!(
                    "Created pipeline schedule '{}' in {}!",
                    created.description,
                    project.key()
                );

                for variable in schedule.variables.unwrap_or_default() {
                    let url = format!(
                        "{}/projects/{}/pipeline_schedules/{}/variables",
                        *TARGET_GITLAB_URL, project.id, created.id
                    );
                    http::CLIENT
                        .post(url)
                        .form(&[
                            ("key", variable.key),
                            ("value", variable.value),
                            ("variable_type", variable.variable_type),
                        ])
                        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
                        .send()
                        .await?;
                }
            }
            Err(err) => println!("{:#?}", err),
        }
    }

    Ok(())
}

pub async fn reassign_target_issue(
    issue: SourceIssue,
    project: &TargetProject,
    assignee: &TargetUser,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Reassigning issue\n{:?}\nin project\n{:?}\nto\n{:?}\n__________",
        issue, project, assignee
    );
    let url = format!(
        "{}/projects/{}/issues/{}",
        *TARGET_GITLAB_URL, project.id, issue.iid
    );
    let response = http::CLIENT
        .put(url)
        .form(&[("assignee_id", assignee.id)])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?;
    if let Err(err) = response.error_for_status() {
        println!("Error: {}", err);
        println!(
            "Context: \n{:?}\nin project\n{:?}\nto\n{:?}\n__________",
            issue, project, assignee
        )
    }
    Ok(())
}

pub async fn add_target_project_member_to_project(
    project: TargetProject,
    user: TargetUser,
    member: SourceMember,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Adding user {:?} to project {:?} from access level {:?}...",
        user, project, member.access_level
    );
    let url = format!("{}/projects/{}/members", *TARGET_GITLAB_URL, project.id);
    let response = http::CLIENT
        .post(url)
        .form(&[
            ("user_id", &user.id.to_string()),
            ("access_level", &member.access_level.to_string()),
        ])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?;
    if let Err(err) = response.error_for_status() {
        println!("{}", err);
    }
    Ok(())
}

pub async fn add_target_project_member_to_group(
    group: TargetGroup,
    user: TargetUser,
    member: SourceMember,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Adding user {:?} to group {:?} from access level {:?}...",
        user, group, member.access_level
    );
    let url = format!("{}/groups/{}/members", *TARGET_GITLAB_URL, group.id);
    let response = http::CLIENT
        .post(url)
        .form(&[
            ("user_id", &user.id.to_string()),
            ("access_level", &member.access_level.to_string()),
        ])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?;
    if let Err(err) = response.error_for_status() {
        println!("{}", err);
    }
    Ok(())
}

pub async fn fetch_all_target_groups() -> Result<Vec<TargetGroup>, Box<dyn Error>> {
    let mut all_groups = vec![];
    let mut latest_page = 1;
    let mut latest_len = 0;
    while latest_len == 100 || latest_page == 1 {
        let mut groups = fetch_target_groups(latest_page).await?;
        latest_len = groups.len();
        latest_page += 1;
        all_groups.append(&mut groups);
    }
    Ok(all_groups)
}

async fn fetch_target_groups(page: u32) -> Result<Vec<TargetGroup>, Box<dyn Error>> {
    let url = format!("{}/groups/", *TARGET_GITLAB_URL);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .send()
        .await?;
    let payload = &response.text().await?;
    let groups: Vec<TargetGroup> = serde_json::from_str(payload)?;
    Ok(groups)
}

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

pub async fn import_target_project(project: SourceProject) -> Result<(), String> {
    let spawn_result =
        tokio::task::spawn_blocking(move || match synchronous_import_target_project(project) {
            Ok(x) => Ok(x),
            Err(err) => {
                println!("{:#?}", err);
                Err("Failed to import target project!".to_owned())
            }
        })
        .await;
    spawn_result.map_err(|_| "Spawn blocking failed!".to_string())?
}

pub fn synchronous_import_target_project(project: SourceProject) -> Result<(), Box<dyn Error>> {
    println!("Importing project {:?}...", project);
    let gz_path = format!("cache/projects/{}.gz", project.id);
    let namespace = parse_namespace(&project);
    let form = reqwest::blocking::multipart::Form::new()
        .text("namespace", namespace)
        .text("name", project.name)
        .text("path", project.path)
        .file("file", gz_path)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(900))
        .build()?;
    let url = format!("{}/projects/import", *TARGET_GITLAB_URL);
    client
        .post(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .multipart(form)
        .send()?
        .error_for_status()?;
    Ok(())
}

fn parse_namespace(project: &SourceProject) -> String {
    let mut path = project.path_with_namespace.split('/').rev();
    path.next();
    path.rev().fold(String::new(), |x, y| {
        if x.is_empty() {
            y.to_string()
        } else {
            x + "/" + y
        }
    })
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

pub async fn create_target_user(
    user: SourceUser,
    email_mapping: &HashMap<String, String>,
) -> Result<TargetUser, String> {
    let user_str = format!("{:?}", user);
    let email = match email_mapping.get(&user.username) {
        Some(x) => x.to_string(),
        None => format!("{}@test.com", user.username),
    };
    let email_str = email.to_string();
    let spawn_result =
        tokio::task::spawn_blocking(move || match synchronous_create_target_user(user, email) {
            Ok(x) => Ok(x),
            Err(err) => Err(format!(
                "Failed to create {}\n{}\n{}.",
                user_str, email_str, err
            )),
        })
        .await;
    spawn_result.map_err(|_| "Spawn blocking failed!".to_string())?
}

pub fn synchronous_create_target_user(
    user: SourceUser,
    email: String,
) -> Result<TargetUser, Box<dyn Error>> {
    println!("Creating user {:?} with email {}...", user, email);
    let avatar = synchronous_download_avatar(&user)?;
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/users", *TARGET_GITLAB_URL);
    let form = reqwest::blocking::multipart::Form::new()
        .text("name", user.name)
        .text("username", user.username)
        .text("email", email)
        .text("force_random_password", "true")
        .text("reset_password", "true")
        .text("skip_confirmation", "true")
        .file("avatar", avatar)?;

    let response = client
        .post(url)
        .header("PRIVATE-TOKEN", &*TARGET_GITLAB_TOKEN)
        .multipart(form)
        .send()?
        .error_for_status()?;

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

pub async fn fetch_source_pipeline_schedules(
    project: &SourceProject,
) -> Result<Vec<SourcePipelineSchedule>, Box<dyn Error>> {
    let url = format!(
        "{}/projects/{}/pipeline_schedules",
        *SOURCE_GITLAB_URL, project.id
    );
    let payload = http::CLIENT
        .get(url)
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let pipeline_schedules: Vec<SourcePipelineScheduleWithoutVariables> =
        serde_json::from_str(&payload)?;

    let mut with_variables = vec![];
    for schedule in pipeline_schedules {
        let url = format!(
            "{}/projects/{}/pipeline_schedules/{}",
            *SOURCE_GITLAB_URL, project.id, schedule.id
        );
        let payload = http::CLIENT
            .get(url)
            .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let pipeline_schedule: SourcePipelineSchedule = serde_json::from_str(&payload)?;
        with_variables.push(pipeline_schedule)
    }
    Ok(with_variables)
}

pub async fn fetch_all_source_issues(
    project: &SourceProject,
) -> Result<Vec<SourceIssue>, Box<dyn Error>> {
    println!("Fetching all issues for {:?}...", project);
    let mut all_groups = vec![];
    let mut latest_page = 1;
    let mut latest_len = 0;
    while latest_len == 100 || latest_page == 1 {
        let mut groups = fetch_source_issues(project, latest_page).await?;
        latest_len = groups.len();
        latest_page += 1;
        all_groups.append(&mut groups);
    }
    Ok(all_groups)
}

async fn fetch_source_issues(
    project: &SourceProject,
    page: u32,
) -> Result<Vec<SourceIssue>, Box<dyn Error>> {
    let url = format!("{}/projects/{}/issues", *SOURCE_GITLAB_URL, project.id);
    let response = http::CLIENT
        .get(url)
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    let payload = &response.text().await?;
    let groups: Vec<SourceIssue> = serde_json::from_str(payload)?;
    Ok(groups)
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
    println!("Requesting export for project id {}...", project_id);
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

pub async fn archive_source_project(project: &SourceProject) -> Result<(), Box<dyn Error>> {
    println!("Requesting to archive project {}...", project.key());
    let url = format!("{}/projects/{}/archive", *SOURCE_GITLAB_URL, project.id);
    http::CLIENT
        .post(url)
        .header("PRIVATE-TOKEN", &*SOURCE_GITLAB_TOKEN)
        .send()
        .await?;
    println!("Archived project {}!", project.key());
    Ok(())
}
