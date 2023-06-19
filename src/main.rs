use std::env;
use std::fs::{self};
use std::io::{Error, Write};

use atom_syndication::{Entry, Feed};
use chrono::{DateTime, Utc};
use opml::Outline;
use rss::{Channel, Item};
use serde::Serialize;
use webpage::{Webpage, WebpageOptions};

use crate::models::Source;

mod db;
mod models;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    if args.len() != 2 {
        println!("Usage: {} <url>", args[0]);
        return Ok(());
    }

    let url = &args[1];

    let dir_path = create_timestamped_dir(url).await;

    if url.starts_with("http") {
        println!("Handling url: {}", url);
        // let dir = dir_path.clone();
        handle_url(&dir_path, url).await.expect("Error handling url");
    } else if url.starts_with("feed!") {
        println!("Handling Feed url: {}", url);
        let orig_feed_url = url.replace("feed!", "");
        let feed_url = get_feed_url(&url, orig_feed_url).await;
        let source_id = uuid::Uuid::try_from("5f4c7adf-2236-428b-9db6-7fbab59b4507").unwrap();
        handle_feed(source_id, &feed_url, &dir_path).await.expect("Feed error");
    } else if url.starts_with("opml!") {
        println!("Handling OPML url: {}", url);

        // strip opml! from url
        let opml_url = url.replace("opml!", "");

        // determine if url is local or remote
        if opml_url.starts_with("http") {} else {
            // assume local file
            let opml_file = fs::read_to_string(opml_url).expect("Unable to read file");
            let opml = opml::OPML::from_str(&opml_file).expect("Unable to parse OPML");

            // first, get all the rss outlines from opml
            let mut outlines = Vec::new();
            for outline in opml.body.outlines {
                collect_outlines(&outline, &mut outlines);
            }

            // then, handle each outline
            for outline in outlines {
                let dir = dir_path.clone();
                handle_opml_outline(&dir, &outline).await;
            }

        }
    } else {
        println!("Unknown url type: {}", url);
    }

    Ok(())
}

fn collect_outlines(outline: &Outline, outlines: &mut Vec<Outline>) {
    // Add the current outline to the list
    if outline.r#type.is_some() && outline.r#type.clone().unwrap() == "rss" {
        outlines.push(outline.clone());
    }

    // Collect all child outlines recursively
    for child in &outline.outlines {
        collect_outlines(child, outlines);
    }
}

async fn handle_opml_outline(dir_path: &str, outline: &Outline) {
    println!("processing: {:?}", outline);

    // save source
    if outline.html_url.is_some() {
        let html_url = outline.html_url.clone().unwrap();
        let source = Source::new(outline.text.clone(), html_url, 5);
        let source_id = source.save().await.expect(save_error("source", source.url.as_str()).as_str());
        // save feed
        if outline.xml_url.is_some() {
            let feed_url = outline.xml_url.clone().unwrap();
            println!("feed_url: {}", feed_url);

            // create feed slug
            let url_simplified = feed_url.replace("https://", "").replace("http://", "").replace("www.", "");
            let feed_slug = slug::slugify(url_simplified);

            // create directory for feed
            let feed_dir = format!("{}/{}", dir_path, feed_slug);
            fs::create_dir_all(&feed_dir).expect("Unable to create directory");

            handle_feed(source_id, &feed_url, &feed_dir).await.expect(save_error("feed", &feed_url).as_str());
        }
    }
}

fn save_error(thing: &str, id: &str) -> String {
    format!("Error saving {}: {}", thing, id)
}

async fn create_timestamped_dir(url: &str) -> String {
    // Generate timestamped directory and slug
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let url_simplified = url.replace("https://", "").replace("http://", "").replace("www.", "");
    let slug = slug::slugify(url_simplified);
    let dir_path = format!("downloads/{}_{}", timestamp, slug);
    fs::create_dir_all(&dir_path).expect("Unable to create directory");
    dir_path
}

async fn handle_url(dir_path: &str, url: &str) -> anyhow::Result<()> {
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
    write_file(dir_path, "content.html", &content).await?;
    write_json_file(dir_path, "html-info.json", &webpage).await?;

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

            let entries: Vec<Entry> = atom.entries;
            if entries.len() == 0 {
                eprintln!("No entries found in Atom feed");
            } else {
                for entry in entries {
                    let news_item = entry_to_news_item(feed.id, &entry);
                    let result = news_item.save().await;
                    match result {
                        Ok(_) => {}
                        Err(e) => println!("Error saving news item: {}", e)
                    }
                }
            }
        }
    } else {
        println!("RSS feed parsed successfully");

        let channel: Channel = rss_parse_result.unwrap();
        let title = Option::from(channel.title);
        let feed_type = Option::from("RSS".to_string());

        // save feed to db
        let feed: models::Feed = feed_webpage_to_feed(source_id, title, feed_type, &feed_webpage);
        let maybe_id = feed.save().await;
        match maybe_id {
            Ok(id) => println!("Feed saved successfully: {}", id),
            Err(e) => println!("Feed not saved (possibly duplicate): {}", e)
        }

        let items: Vec<Item> = channel.items;
        if items.len() == 0 {
            eprintln!("No items found in RSS feed");
        } else {
            for item in items {
                let news_item = item_to_news_item(feed.id, &item);
                let maybe_id = news_item.save().await;
                match maybe_id {
                    Ok(id) => println!("News item saved successfully: {}", id),
                    Err(e) => println!("News item not saved (possibly duplicate): {}", e)
                }
            }
        }
    }

    Ok(())
}

/// Convert an RSS item to a NewsItem
fn item_to_news_item(feed_id: uuid::Uuid, item: &Item) -> models::NewsItem {
    let title = item.title.clone().expect("Unable to get title");
    // set guid to either guid or link
    let guid = match item.guid.clone() {
        Some(guid) => guid.value,
        None => item.link.clone().expect("Unable to get link for guid")
    };
    let url = item.link.clone().expect("Unable to get link");
    let maybe_pub_date = item.pub_date.clone();
    let pub_date: DateTime<Utc> = match maybe_pub_date {
        Some(dt) => {
            match parse_date(&dt) {
                Some(dt) => dt,
                None => {
                    eprintln!("{}", date_parse_error(&dt));
                    Utc::now()
                }
            }
        }
        None => Utc::now()
    };
    models::NewsItem::new(feed_id, guid, title, pub_date, url)
}

fn parse_date(dt: &str) -> Option<DateTime<Utc>> {
    let formats = [
        "%a, %d %b %Y %H:%M:%S GMT",
        "%a, %d %b %Y %H:%M:%S %z"
    ];
    for format in &formats {
        if let Ok(dt) = DateTime::parse_from_str(dt, format) {
            return Some(dt.with_timezone(&Utc));
        }
    }
    None
}

fn date_parse_error(date: &str) -> String {
    format!("Failed to parse date and time: {}", date)
}

/// Convert an Atom entry to a NewsItem
fn entry_to_news_item(feed_id: uuid::Uuid, entry: &Entry) -> models::NewsItem {
    let title = entry.title.clone().value;
    let guid = entry.id.clone();
    let url = entry.links[0].href.clone();
    let published = match entry.published {
        Some(p) => p,
        None => entry.updated.clone()
    };
    models::NewsItem::new(feed_id, guid, title, DateTime::from(published), url)
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
    match &feed_parsed {
        Ok(channel) => {
            let feed_parsed_json = serde_json::to_string_pretty(&channel).expect("Unable to serialize RSS channel");
            let mut feed_parsed_file: fs::File = fs::File::create(&parsed_file_path).expect("Unable to create feed parsed file");
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
