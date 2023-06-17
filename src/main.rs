use std::env;
use std::fs::{self};
use std::io::{Error, Write};

use atom_syndication::Feed;
use rss::Channel;
use serde::Serialize;
use webpage::{Webpage, WebpageOptions};
use crate::models::{Source};

mod db;
mod models;

#[tokio::main]
async fn main() -> Result<(), Error> {

    /*let source_types = db::source_types().await.unwrap();
    println!("source_types: {:?}", source_types);

    let sources = db::sources().await.unwrap();
    println!("sources: {:?}", sources);

    let feeds = db::feeds().await.unwrap();
    println!("feeds: {:?}", feeds);

    let news = db::news().await.unwrap();
    println!("news: {:?}", news);*/

    /*let st = SourceType::new(5, "Website".to_string(), None);
    let id: anyhow::Result<i32> = st.save().await;
    if id.is_err() {
        println!("Error saving SourceType: {:?}", id.err().unwrap());
    } else {
        println!("Saved: {:?}", st);
    }*/

    /*let st = db::source_type_by_name("Website").await.expect("Error getting source type");

    let s = Source::new("This Week in Rust".to_owned(), "https://this-week-in-rust.org/".to_owned(), st.id);
    let id = s.save().await;
    if id.is_err() {
        println!("Error saving Source: {:?}", id.err().unwrap());
    } else {
        println!("Saved: {:?}", s);
    }*/



    /*let s = models::Source {
        id: uuid::Uuid::new_v4(),
        name: "name".to_string(),
        url: "url".to_string(),
        type_id: 1,
        paywall: None,
        feed_available: None,
        description: None,
        short_name: None,
        state: None,
        city: None,
        create_timestamp: chrono::Utc::now().into(),
    };

    let saved_s = db::upsert_source(s).await;
    println!("Saved source: {:?}", saved_s);*/

    /*let f = models::Feed {
        id: uuid::Uuid::new_v4(),
        source_id: Default::default(),
        url: "https://this-week-in-rust.org/atom.xml".to_string(),
        title: "This Week in Rust".to_string(),
        create_timestamp: None,
        feed_type: None,
        ttl: None,
    };
    let saved_f = db::upsert_feed(f).await;
    println!("Saved feed: {:?}", saved_f);*/


    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    if args.len() != 2 {
        println!("Usage: {} <url>", args[0]);
        return Ok(());
    }

    let url = &args[1];

    // Generate timestamped directory and slug
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let url_simplified = url.replace("https://", "").replace("http://", "").replace("www.", "");
    let slug = slug::slugify(url_simplified);
    let dir_path = format!("downloads/{}_{}", timestamp, slug);
    fs::create_dir_all(&dir_path)?;

    let html_options = WebpageOptions { allow_insecure: true, ..Default::default() };
    let webpage_result = Webpage::from_url(&url, html_options);
    let webpage = match webpage_result {
        Ok(v) => v,
        Err(e) => {
            println!("Error fetching html webpage: {}", e);
            return Ok(());
        }
    };

    // save source to db
    let source = webpage_to_source(&webpage);
    source.save().await.expect("Error saving source");

    println!("source: {:?}", source);

    let content = &webpage.http.body;
    write_file(dir_path.as_str(), "content.html", &content).await?;
    write_json_file(dir_path.as_str(), "html-info.json", &webpage).await?;

    // If there's a feed available, write it to a file
    if webpage.html.feed.is_some() {
        let orig_feed_url = webpage.html.feed.unwrap();
        let feed_url = get_feed_url(&url, orig_feed_url).await;
        handle_feed(source.id, &feed_url, &dir_path).await.expect("Feed error");
    }

    Ok(())
}

fn webpage_to_source(webpage: &Webpage) -> Source {
    let title = webpage.html.title.clone().unwrap();
    let url = webpage.http.url.clone();
    let type_id = 5; // Website
    Source::new(title, url, type_id)
}

async fn get_feed_url(url: &str, orig_feed_url: String) -> String {
    let mut feed_url = orig_feed_url.clone();
    println!("Orig feed URL: {}", feed_url);

    if feed_url.starts_with("http") {
        return feed_url;
    }

    if feed_url.chars().next().map_or(false, |c| c.is_alphabetic()) {
        feed_url = format!("{}/{}", &url, &feed_url);
    } else if feed_url.starts_with('/') {
        feed_url = format!("{}{}", &url, &feed_url);
    } else {
        eprintln!("Feed URL is not valid: {}", feed_url);
    }

    println!("Feed URL: {}", feed_url);

    return feed_url;
}

async fn write_file(dir_path: &str, file_name: &str, content: &String) -> Result<String, Error>
{
    let content_path = format!("{}/{}", dir_path, file_name);
    let mut file: fs::File = fs::File::create(&content_path)?;
    file.write_all(content.as_bytes())?;
    Ok(content_path)
}

async fn write_json_file<T>(dir_path: &str, file_name: &str, content: &T) -> Result<String, Error>
    where T: ?Sized + Serialize
{
    let info_path = format!("{}/{}", dir_path, file_name);
    let mut file: fs::File = fs::File::create(&info_path)?;
    let info_json = serde_json::to_string_pretty(&content)?;
    file.write_all(&info_json.as_bytes())?;
    Ok(info_path)
}

async fn handle_feed(source_id: uuid::Uuid, feed_url: &str, dir_path: &str) -> Result<(), Error> {
    let feed_options = WebpageOptions { allow_insecure: true, ..Default::default() };
    let feed_webpage_result = Webpage::from_url(&feed_url, feed_options);
    let feed_webpage = match feed_webpage_result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error fetching feed webpage: {}", e);
            return Ok(());
        }
    };

    // Write the feed body to a file
    let feed_content = &feed_webpage.http.body;

    write_file(dir_path, "feed.txt", &feed_content).await?;

    // Write the feed info to a file
    write_json_file(dir_path, "feed-info.json", &feed_webpage).await?;

    let rss_parse_result = handle_rss_feed(dir_path, feed_content.to_string()).await;
    if rss_parse_result.is_err() {
        println!("Trying to parse as Atom feed...");
        let atom_parse_result = handle_atom_feed(dir_path, feed_content).await;
        if atom_parse_result.is_err() {
            println!("Error parsing Atom feed: {}", atom_parse_result.err().unwrap());
        } else {
            println!("Atom feed parsed successfully");

            let atom = atom_parse_result.unwrap();
            let title = Option::from(atom.title.value);
            let feed_type = Option::from("Atom".to_string());

            // save feed to db
            let feed: models::Feed = feed_webpage_to_feed(source_id, title, feed_type, &feed_webpage);
            feed.save().await.expect("Error saving feed");

            // TODO save Atom feed stories to db
        }
    } else {
        println!("RSS feed parsed successfully");

        let rss = rss_parse_result.unwrap();
        let title = Option::from(rss.title);
        let feed_type = Option::from("RSS".to_string());

        // save feed to db
        let feed: models::Feed = feed_webpage_to_feed(source_id, title, feed_type, &feed_webpage);
        feed.save().await.expect("Error saving feed");

        // TODO save RSS feed stories to db
    }

    Ok(())
}

fn feed_webpage_to_feed(source_id: uuid::Uuid, title: Option<String>, feed_type: Option<String>, webpage: &Webpage) -> models::Feed {
    let url = webpage.http.url.clone();
    models::Feed::new(source_id, url, title, feed_type)
}

async fn handle_atom_feed(dir_path: &str, feed_content: &str) -> Result<Feed, atom_syndication::Error> {
    let feed_parsed = Feed::read_from(feed_content.as_bytes());
    let parsed_file_path = format!("{}/{}", dir_path, "feed-parsed.json");
    match &feed_parsed {
        Ok(feed) => {
            let feed_parsed_json = serde_json::to_string_pretty(&feed).expect("Unable to serialize Atom feed");
            let mut feed_parsed_file: fs::File = fs::File::create(&parsed_file_path).expect("Unable to create feed parsed file");
            feed_parsed_file.write_all(&feed_parsed_json.as_bytes()).expect("Unable to write feed parsed file");
        }
        Err(err) => println!("Error parsing RSS feed: {}", err),
    }

    return feed_parsed;
}

async fn handle_rss_feed(dir_path: &str, feed_content: String) -> Result<Channel, rss::Error> {
    let feed_parsed = Channel::read_from(feed_content.as_bytes());
    let parsed_file_path = format!("{}/{}", dir_path, "feed-parsed.json");
    println!("Parsed file path: {}", parsed_file_path);
    match &feed_parsed {
        Ok(channel) => {
            let feed_parsed_json = serde_json::to_string_pretty(&channel).expect("Unable to serialize RSS channel");
            let feed_parsed_path = format!("{}/{}", dir_path, parsed_file_path);
            let mut feed_parsed_file: fs::File = fs::File::create(&feed_parsed_path).expect("Unable to create feed parsed file");
            feed_parsed_file.write_all(&feed_parsed_json.as_bytes()).expect("Unable to write feed parsed file");
        }
        Err(err) => println!("Error parsing RSS feed: {}", err),
    }

    return feed_parsed;
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
