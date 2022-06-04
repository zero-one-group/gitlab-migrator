pub fn load_env(key: &str) -> String {
    let result = std::env::var(key);
    result.unwrap_or_default()
}
