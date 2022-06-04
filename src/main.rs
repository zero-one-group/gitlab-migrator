use gitlab_migrator::apps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let groups = apps::fetch_all_source_groups().await?;
    println!("{:#?}", groups);
    println!("{:#?}", groups.len());
    Ok(())
}
