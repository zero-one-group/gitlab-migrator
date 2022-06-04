use futures::future::{try_join_all, TryFuture};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

pub type Client = ClientWithMiddleware;

lazy_static::lazy_static! {
    pub static ref CLIENT: Client = create_shared_client();
}

fn create_shared_client() -> Client {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

pub async fn politely_try_join_all<I>(
    futures: I,
    chunk_size: usize,
    throttle_ms: u64,
) -> Result<Vec<<I::Item as TryFuture>::Ok>, <I::Item as TryFuture>::Error>
where
    I: IntoIterator,
    I::Item: TryFuture,
{
    let chunk_size = if chunk_size < 1 { 1 } else { chunk_size };
    let mut acc_items = vec![];
    let mut futures = futures.into_iter().peekable();
    while futures.peek().is_some() {
        let chunk: Vec<_> = futures.by_ref().take(chunk_size).collect();
        let mut chunk_items = try_join_all(chunk).await?;
        acc_items.append(&mut chunk_items);
        throttle_for(throttle_ms);
    }
    Ok(acc_items)
}

pub fn throttle_for(ms: u64) {
    println!("Throttling for {}ms...", ms);
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
