use gitlab_migrator::apps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    apps::wait_and_save_all_project_zips().await?;
    Ok(())
}
