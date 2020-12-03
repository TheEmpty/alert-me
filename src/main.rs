use reqwest::Error;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;
use std::sync::Arc;
use std::collections::HashMap; // Not threadsafe

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
    let last_update_hash: Arc<HashMap<String, f64>> = Arc::new(HashMap::new());

    println!(
        "Will call {} when a new comment is added.",
        get_trigger_executable_path(),
    );

    loop {
        println!("Refreshing");
        // TODO: config
        check_reddit_user_comments("Fast-Wolverine".to_string(), last_update_hash.clone()).await;
        tokio::time::delay_for(WAIT_INTERVAL_DURATION.clone()).await;
    }
}

/// Triggers for new comments from the given reddit user
async fn check_reddit_user_comments(username: String, mut last_update_hash: Arc<HashMap<String, f64>>) {
    let request_url = format!(
        "https://www.reddit.com/user/{user}/comments.json",
        user = username, 
    );
    // A unwrap() a monster
    let response: RedditResponse = reqwest::get(&request_url).await.unwrap().json().await.unwrap();

    let comments = response.data.children.iter();

    let last_update_key = format!("reddit_{user}", user = username);

    if last_update_hash.get(&last_update_key).is_none() {
        Arc::get_mut(&mut last_update_hash).unwrap().insert(last_update_key.clone(), response.data.children.get(0).unwrap().data.created_utc);
    }

    for comment in comments.rev() {
        let last_update = last_update_hash.get(&last_update_key).unwrap();
        if comment.data.created_utc > *last_update {
            Arc::get_mut(&mut last_update_hash).unwrap().insert(last_update_key.clone(), comment.data.created_utc);
            let message = format!(
                "{user}: {comment}",
                user = comment.data.author,
                comment = comment.data.body
            );

            println!("Triggering for {:?}", comment);
            let _ = Command::new(get_trigger_executable_path())
                .arg(message)
                .spawn();
        }
    }
}