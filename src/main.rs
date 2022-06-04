use gitlab_migrator::apps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let memberships = apps::fetch_all_memberships().await?;
    println!("{:#?}", memberships);
    Ok(())
}
