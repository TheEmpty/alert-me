use dashmap::DashMap;
use log::{debug, info};
use reqwest::Error;
use serde::Deserialize;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

/// How often to poll the reddit
const WAIT_INTERVAL_DURATION: Duration = Duration::from_secs(60 * 5);

#[derive(Deserialize, Debug)]
struct RedditCommentData {
    author: String,
    body: String,
    // reqwest is certain this is an f and not a usize?
    created_utc: f64,
}

#[derive(Deserialize, Debug)]
struct RedditComment {
    data: RedditCommentData,
}

#[derive(Deserialize, Debug)]
struct RedditComments {
    children: Vec<RedditComment>,
}

#[derive(Deserialize, Debug)]
struct RedditResponse {
    data: RedditComments,
}

fn get_trigger_executable_path() -> String {
    let os_string = std::env::current_dir().unwrap();
    format!(
        "{pwd}/{executable}",
        pwd = os_string.to_str().unwrap(),
        executable = "trigger"
    )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();
    let last_update_hash: Arc<DashMap<String, f64>> = Arc::new(DashMap::new());

    debug!(
        "Will call {} when a new comment is added.",
        get_trigger_executable_path(),
    );

    loop {
        info!("Refreshing");
        // TODO: config
        let _ = check_reddit_user_comments("Fast-Wolverine".to_string(), last_update_hash.clone()).await;

        let _ = check_amazon_stock(
            "PS5".to_string(),
            "com".to_string(),
            "B08FC5L3RG".to_string(),
            last_update_hash.clone(),
        )
        .await;

        let _ = check_target_stock(
            "PS5".to_string(),
            "A-81114595".to_string(),
            last_update_hash.clone(),
        ).await;

        tokio::time::delay_for(WAIT_INTERVAL_DURATION.clone()).await;
    }
}

async fn check_target_stock(
    name: String,
    id: String,
    last_update_hash: Arc<DashMap<String, f64>>,
) -> Result<(), ErrorCode> {
    let request_url = format!(
        "https://www.target.com/p/{}",
        id
    );
    let response = reqwest::get(&request_url).await?.text().await?;

    let stock = match response.contains("Out of stock in stores near you") {
        true => 0.0,
        false => 1.0,
    };

    let last_update_key = format!("target_{}", id);
    if last_update_hash.get(&last_update_key).is_none() {
        last_update_hash.insert(last_update_key.clone(), stock);
    } else {
        let last_value = match last_update_hash.get(&last_update_key) {
            Some(val) => val,
            None => return Err(ErrorCode::GeneralError),
        };

        if *last_value != stock {
            last_update_hash.insert(last_update_key.clone(), stock);
            info!(
                "Target stock for {name} changed to {stock}",
                name = name,
                stock = stock
            );
            if stock == 1.0 {
                let product = format!(
                    "{name} back in stock on Target",
                    name = name,
                );
                debug!("Triggering for {:?}", product);
                let _ = Command::new(get_trigger_executable_path())
                    .arg(product)
                    .spawn();
            }
        }
    }

    Ok(())
}

async fn check_amazon_stock(
    name: String,
    domain: String,
    asin: String,
    last_update_hash: Arc<DashMap<String, f64>>,
) -> Result<(), ErrorCode> {
    let request_url = format!(
        "https://www.amazon.{domain}/dp/{asin}",
        domain = domain,
        asin = asin
    );
    let response = reqwest::get(&request_url).await?.text().await?;

    let stock = match response.contains("type=\"submit\" value=\"Add to Cart\"") {
        true => 1.0,
        false => 0.0,
    };

    let last_update_key = format!("amazon_{asin}", asin = asin);
    if last_update_hash.get(&last_update_key).is_none() {
        last_update_hash.insert(last_update_key.clone(), stock);
    } else {
        let last_value = match last_update_hash.get(&last_update_key) {
            Some(val) => val,
            None => return Err(ErrorCode::GeneralError),
        };

        if *last_value != stock {
            last_update_hash.insert(last_update_key.clone(), stock);
            info!(
                "Amazon.{domain} stock for {name} changed to {stock}",
                domain = domain,
                name = name,
                stock = stock
            );
            if stock == 1.0 {
                let product = format!(
                    "{name} ({asin}) back in stock on Amazon.{domain}",
                    name = name,
                    asin = asin,
                    domain = domain
                );
                debug!("Triggering for {:?}", product);
                let _ = Command::new(get_trigger_executable_path())
                    .arg(product)
                    .spawn();
            }
        }
    }

    Ok(())
}

/// Triggers for new comments from the given reddit user
async fn check_reddit_user_comments(
    username: String,
    last_update_hash: Arc<DashMap<String, f64>>,
) -> Result<(), ErrorCode> {
    let request_url = format!(
        "https://www.reddit.com/user/{user}/comments.json",
        user = username,
    );
    let response: RedditResponse = reqwest::get(&request_url).await?.json().await?;

    let comments = response.data.children.iter();

    let last_update_key = format!("reddit_{user}", user = username);

    if last_update_hash.get(&last_update_key).is_none() {
        match response.data.children.get(0) {
            Some(val) => {
                last_update_hash.insert(last_update_key.clone(), val.data.created_utc);
            }
            None => return Err(ErrorCode::GeneralError),
        }
    }

    for comment in comments.rev() {
        let last_update = match last_update_hash.get(&last_update_key) {
            Some(val) => val,
            None => return Err(ErrorCode::GeneralError),
        };

        if comment.data.created_utc > *last_update {
            last_update_hash.insert(last_update_key.clone(), comment.data.created_utc);
            let message = format!(
                "{user}: {comment}",
                user = comment.data.author,
                comment = comment.data.body
            );

            debug!("Triggering for {:?}", comment);
            let _ = Command::new(get_trigger_executable_path())
                .arg(message)
                .spawn();
        }
    }

    Ok(())
}

enum ErrorCode {
    GeneralError,
}

impl From<reqwest::Error> for ErrorCode {
    fn from(_error: reqwest::Error) -> Self {
        ErrorCode::GeneralError
    }
}
