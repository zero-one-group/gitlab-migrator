use crate::types::SourceGroup;
use crate::{env, http};
use std::error::Error;

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

async fn fetch_source_groups(page: usize) -> Result<Vec<SourceGroup>, Box<dyn Error>> {
    let url = env::load_env("SOURCE_GITLAB_URL");
    let token = env::load_env("SOURCE_GITLAB_TOKEN");
    let response = http::CLIENT
        .get(format!("{}/groups/", url))
        .query(&[("per_page", "100"), ("page", &page.to_string())])
        .header("PRIVATE-TOKEN", token)
        .send()
        .await?;
    let payload = &response.text().await?;
    let groups: Vec<SourceGroup> = serde_json::from_str(payload)?;
    Ok(groups)
}
