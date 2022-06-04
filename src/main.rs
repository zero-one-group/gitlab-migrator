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
        Some("create-target-users") => Ok(apps::create_target_users().await?),
        Some(_) => Err("Unrecognised application name!".into()),
        None => Err("Must specify an application name!".into()),
    }
}
