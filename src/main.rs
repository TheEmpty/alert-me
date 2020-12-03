use serde::Deserialize;
use reqwest::Error;
use std::time::Duration;
use std::process::Command;

/// How often to poll the reddit
const WAIT_INTERVAL_DURATION: Duration = Duration::from_secs(60 * 5);
// currently only uses first user
const USER: &str = "Fast-Wolverine";

#[derive(Deserialize, Debug)]
struct RedditCommentData {
    author: String,
    body: String,
    // reqwest is certain this is an f and not a usize?
    created_utc: f64,
}

#[derive(Deserialize, Debug)]
struct RedditComment {
    data: RedditCommentData
}

#[derive(Deserialize, Debug)]
struct RedditComments {
    children: Vec<RedditComment>
}

#[derive(Deserialize, Debug)]
struct RedditResponse {
    data: RedditComments
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut last_update = 0 as f64;
    let os_string = std::env::current_dir().unwrap();
    let trigger_executable = format!("{pwd}/{executable}", pwd = os_string.to_str().unwrap(), executable = "trigger");
    println!("Will call {} when a new comment is added.", trigger_executable);

    loop {
        println!("Refreshing");
        let request_url = format!("https://www.reddit.com/user/{user}/comments.json",
                                user = USER);
        let response: RedditResponse = reqwest::get(&request_url).await?.json().await?;

        // in asc order
        let comments = response.data.children.iter();

        if last_update == 0.0 {
            last_update = response.data.children.get(0).unwrap().data.created_utc;
        }

        for comment in comments.rev() {
            if comment.data.created_utc > last_update {
                last_update = comment.data.created_utc;
                let message = format!("{user}: {comment}", user = comment.data.author, comment = comment.data.body);
                println!("Triggering for {:?}", comment);
                let _ = Command::new(trigger_executable.clone()).arg(message).spawn();
            }
        }

        tokio::time::delay_for(WAIT_INTERVAL_DURATION.clone()).await;
    }
}
