use gitlab_migrator::apps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    apps::fetch_and_save_all_ci_variables().await?;
    Ok(())
}
