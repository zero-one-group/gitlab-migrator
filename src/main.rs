#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let url = load_env("SOURCE_GITLAB_URL");
    let token = load_env("SOURCE_GITLAB_TOKEN");
    let response = gitlab_migrator::http::CLIENT
        .get(format!("{}/groups/", url))
        .query(&[("per_page", "100")])
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?;
    println!("{:#?}", response.text().await?);
    Ok(())
}

pub fn load_env(key: &str) -> String {
    let result = std::env::var(key);
    result.unwrap_or_default()
}
