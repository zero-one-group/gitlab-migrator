#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = gitlab_migrator::http::CLIENT
        .get("https://ifconfig.me")
        .send()
        .await?;
    println!("{:#?}", response.text().await?);
    Ok(())
}
