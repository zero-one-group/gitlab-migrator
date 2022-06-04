use gitlab_migrator::apps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let projects = apps::fetch_all_source_projects().await?;
    println!("{:#?}", projects);
    println!("{:#?}", projects.len());
    Ok(())
}
