use gitlab_migrator::apps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    let input_app = args.get(1).map(|x| x.as_str());
    match input_app {
        Some("download-source-memberships") => Ok(apps::download_source_memberships().await?),
        Some("download-source-projects") => Ok(apps::download_source_projects().await?),
        Some("download-source-ci-variables") => Ok(apps::download_source_ci_variables().await?),
        Some("download-source-issues") => Ok(apps::download_source_issues().await?),
        Some("download-source-project-metadata") => {
            Ok(apps::download_source_project_metadata().await?)
        }
        Some("create-target-users") => Ok(apps::create_target_users().await?),
        Some("delete-target-users") => Ok(apps::delete_target_users().await?),
        Some("import-target-projects") => Ok(apps::import_target_projects().await?),
        Some("delete-target-projects") => Ok(apps::delete_target_projects().await?),
        Some("add-target-users-to-groups") => Ok(apps::add_target_users_to_groups().await?),
        Some(_) => Err("Unrecognised application name!".into()),
        None => Err("Must specify an application name!".into()),
    }
}
