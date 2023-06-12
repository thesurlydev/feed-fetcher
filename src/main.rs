use std::fs::{self};
use std::io::{Error, Write};

use rss::Channel;
use serde::Serialize;
use webpage::{Webpage, WebpageOptions};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let url = "https://columbiabasinherald.com/";

    // Generate timestamped directory and slug
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let url_simplified = url.replace("https://", "").replace("http://", "").replace("www.", "");
    let slug = slug::slugify(url_simplified);
    let dir_path = format!("downloads/{}_{}", timestamp, slug);
    fs::create_dir_all(&dir_path)?;

    let html_options = WebpageOptions { allow_insecure: true, ..Default::default() };
    let webpage = Webpage::from_url(&url, html_options);
    let webpage = match webpage {
        Ok(v) => v,
        Err(e) => {
            println!("Error fetching html webpage: {}", e);
            return Ok(());
        }
    };

    let content = &webpage.http.body;
    write_file(dir_path.clone(), "content.html", &content).await?;
    write_json_file(dir_path.clone(), "html-info.json", &webpage).await?;

    // If there's a feed available, write it to a file
    if webpage.html.feed.is_some() {
        let feed_url = webpage.html.feed.unwrap();
        handle_feed(&feed_url, &dir_path).await.expect("Feed error");
    }

    Ok(())
}

async fn write_file(dir_path: String, file_name: &str, content: &String) -> Result<String, Error>
{
    let content_path = format!("{}/{}", dir_path, file_name);
    let mut file: std::fs::File = std::fs::File::create(&content_path)?;
    file.write_all(content.as_bytes())?;
    Ok(content_path)
}

async fn write_json_file<T>(dir_path: String, file_name: &str, content: &T) -> Result<String, Error>
    where T: ?Sized + Serialize
{
    let info_path = format!("{}/{}", dir_path, file_name);
    let mut file: std::fs::File = std::fs::File::create(&info_path)?;
    let info_json = serde_json::to_string_pretty(&content)?;
    file.write_all(&info_json.as_bytes())?;
    Ok(info_path)
}

async fn handle_feed(feed_url: &str, dir_path: &str) -> Result<(), Error> {
    let feed_options = WebpageOptions { allow_insecure: true, ..Default::default() };
    let feed_webpage = Webpage::from_url(&feed_url, feed_options);
    let webpage = match feed_webpage {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error fetching feed webpage: {}", e);
            return Ok(());
        }
    };

    // Write the feed body to a file
    let feed_content = &webpage.http.body;
    let feed_content_path = format!("{}/{}", dir_path, "feed.txt");
    let mut file: std::fs::File = std::fs::File::create(&feed_content_path)?;
    file.write_all(feed_content.as_bytes())?;

    // Write the feed info to a file
    let feed_info_path = format!("{}/{}", dir_path, "feed-info.json");
    let mut feed_info_file: std::fs::File = std::fs::File::create(&feed_info_path)?;
    let feed_info_json = serde_json::to_string_pretty(&webpage)?;
    feed_info_file.write_all(&feed_info_json.as_bytes())?;

    let feed_parsed = Channel::read_from(feed_content.as_bytes());
    match feed_parsed {
        Ok(channel) => {
            let feed_parsed_json = serde_json::to_string_pretty(&channel)?;
            let feed_parsed_path = format!("{}/{}", dir_path, "feed-parsed.json");
            let mut feed_parsed_file: std::fs::File = std::fs::File::create(&feed_parsed_path)?;
            feed_parsed_file.write_all(&feed_parsed_json.as_bytes())?;
        }
        Err(err) => eprintln!("Error parsing feed: {}", err),
    }

    Ok(())
}

/*
/// Using playwright, fetch the content of the URL
async fn playwright_fetch(url: &str, user_agent: &String) -> String {
    let playwright = Playwright::initialize().await.expect("Unable to initialize playwright");
    playwright.prepare().expect("Error installing browsers");
    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.expect("Unable to launch browser");
    let context = browser.context_builder().user_agent(user_agent).build().await.expect("Unable to build context");
    let page = context.new_page().await.expect("Unable to create page");
    page.goto_builder(url).goto().await.expect("Unable to navigate to page");
    page.content().await.expect("Unable to get page content")
}*/
