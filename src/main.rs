use std::env;
use std::fs::{self, File};
use std::io::{Error, Write};

use atom_syndication::{Entry, Feed, Link};
use chrono::{DateTime, NaiveDate, Utc};
use opml::Outline;
use playwright::Playwright;
use rss::{Channel, Item};
use serde::Serialize;
use sqlx::{Pool, Postgres};
use url::Url;
use webpage::{Webpage, WebpageOptions};
use log::{error, info, warn, LevelFilter};
use simplelog::{ColorChoice, CombinedLogger, Config, TerminalMode, TermLogger, WriteLogger};

use crate::models::Source;

mod db;
mod models;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    info!("{:?}", args);
    if args.len() != 2 {
        info!("Usage: {} <url>", args[0]);
        return Ok(());
    }

    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Info, Config::default(), File::create("feed-fetcher.log").unwrap()),
        ]
    ).unwrap();

    let url = &args[1];

    let dir_path = create_timestamped_dir(url).await;

    let pool = db::get_pool().await;

    if url.starts_with("http") {
        info!("Handling url: {}", url);
        // let dir = dir_path.clone();
        handle_url(&dir_path, url, &pool).await.expect("Error handling url");
    } else if url.starts_with("feed!") {
        info!("Handling Feed url: {}", url);
        let orig_feed_url = url.replace("feed!", "");
        let feed_url = get_feed_url(&url, orig_feed_url).await;
        // TODO create a new source record
        let source_id = uuid::Uuid::try_from("5f4c7adf-2236-428b-9db6-7fbab59b4507").unwrap();
        handle_feed(source_id, &feed_url, &dir_path, &pool).await.expect("Feed error");
    } else if url.starts_with("opml!") {
        info!("Handling OPML url: {}", url);

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
                handle_opml_outline(&dir, &outline, &pool).await;
            }

        }
    } else {
        error!("Unknown url type: {}", url);
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

async fn handle_opml_outline(dir_path: &str, outline: &Outline, pool: &Pool<Postgres>) {
    info!("processing: {:?}", outline);

    // save source
    if outline.html_url.is_some() {
        let html_url = outline.html_url.clone().unwrap();
        let source = Source::new(outline.text.clone(), html_url, 5);
        let source_id = source.save(&pool).await.expect(save_error("source", source.url.as_str()).as_str());
        // save feed
        if outline.xml_url.is_some() {
            let feed_url = outline.xml_url.clone().unwrap();

            // create feed slug
            let url_simplified = feed_url.replace("https://", "").replace("http://", "").replace("www.", "");
            let feed_slug = slug::slugify(url_simplified);

            // create directory for feed
            let safe_feed_slug = safe_filename(&feed_slug).await;
            let feed_dir = format!("{}/{}", dir_path, safe_feed_slug);
            fs::create_dir_all(&feed_dir).expect("Unable to create directory");

            handle_feed(source_id, &feed_url, &feed_dir, pool).await.expect(save_error("feed", &feed_url).as_str());
        }
    }
}

fn save_error(thing: &str, id: &str) -> String {
    format!("Error saving {}: {}", thing, id)
}

async fn create_timestamped_dir(url: &str) -> String {
    // Generate timestamped directory and slug
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let url_simplified = url.replace("https://", "").replace("http://", "").replace("www.", "");
    let slug = slug::slugify(url_simplified);
    let dir_path = format!("downloads/{}_{}", timestamp, slug);
    fs::create_dir_all(&dir_path).expect("Unable to create directory");
    dir_path
}

async fn handle_url(dir_path: &str, url: &str, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let html_options = WebpageOptions { allow_insecure: true, ..Default::default() };
    let webpage_result = Webpage::from_url(&url, html_options);
    let webpage = match webpage_result {
        Ok(v) => v,
        Err(e) => {
            error!("Error fetching html webpage: {}", e);
            return Ok(());
        }
    };

    // save source to db
    let source = webpage_to_source(&webpage);
    source.save(pool).await.expect("Error saving source");

    info!("source: {:?}", source);

    let content = &webpage.http.body;
    write_file(dir_path, "content.html", &content).await?;
    write_json_file(dir_path, "html-info.json", &webpage).await?;

    // If there's a feed available, write it to a file
    if webpage.html.feed.is_some() {
        let orig_feed_url = webpage.html.feed.unwrap();
        let feed_url = get_feed_url(&url, orig_feed_url).await;
        handle_feed(source.id, &feed_url, &dir_path, pool).await.expect("Feed error");
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
    info!("Orig feed URL: {}", feed_url);

    if feed_url.starts_with("http") {
        return feed_url;
    }

    if feed_url.chars().next().map_or(false, |c| c.is_alphabetic()) {
        feed_url = format!("{}/{}", &url, &feed_url);
    } else if feed_url.starts_with('/') {
        feed_url = format!("{}{}", &url, &feed_url);
    } else {
        error!("Feed URL is not valid: {}", feed_url);
    }

    info!("Feed URL: {}", feed_url);

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

async fn handle_feed(source_id: uuid::Uuid, feed_url: &str, dir_path: &str, pool: &Pool<Postgres>) -> Result<(), Error> {
    let feed_options = WebpageOptions { allow_insecure: true, ..Default::default() };
    let feed_webpage_result = Webpage::from_url(&feed_url, feed_options);
    let feed_webpage = match feed_webpage_result {
        Ok(v) => v,
        Err(e) => {
            error!("Error fetching feed webpage: {}; {}", feed_url, e);
            return Ok(());
        }
    };

    // Write the feed body to a file
    let feed_content = &feed_webpage.http.body;

    write_file(dir_path, "feed.txt", &feed_content).await?;

    // Write the feed info to a file
    write_json_file(dir_path, "feed-info.json", &feed_webpage).await?;

    let rss_parse_result = handle_rss_feed(dir_path, feed_content.to_string(), true).await;
    if rss_parse_result.is_err() {
        info!("Trying to parse as Atom feed...");
        let atom_parse_result = handle_atom_feed(dir_path, feed_content, true).await;
        if atom_parse_result.is_err() {
            info!("Error parsing Atom feed: {}", atom_parse_result.err().unwrap());
        } else {
            info!("Atom feed parsed successfully");

            let atom = atom_parse_result.unwrap();
            let title = Option::from(atom.title.value);
            let feed_type = Option::from("Atom".to_string());

            // save feed to db
            let feed: models::Feed = feed_webpage_to_feed(source_id, title, feed_type, &feed_webpage);
            feed.save(pool).await.expect("Error saving feed");

            let entries: Vec<Entry> = atom.entries;
            if entries.len() == 0 {
                error!("No entries found in Atom feed");
            } else {
                for entry in entries {
                    let news_item = entry_to_news_item(feed.id, &entry);
                    let result = news_item.save(pool).await;
                    match result {
                        Ok(_) => {}
                        Err(e) => info!("Error saving news item: {}", e)
                    }
                }
            }
        }
    } else {
        info!("RSS feed parsed successfully");

        let channel: Channel = rss_parse_result.unwrap();
        let title = Option::from(channel.clone().title);
        let feed_type = Option::from("RSS".to_string());

        // save feed to db
        let feed: models::Feed = feed_webpage_to_feed(source_id, title, feed_type, &feed_webpage);
        let maybe_id = feed.save(pool).await;

        match maybe_id {
            Ok(id) => info!("Feed saved successfully: {}", id),
            Err(e) => info!("Feed not saved (possibly duplicate): {}", e)
        }

        let items: Vec<Item> = channel.clone().items;
        if items.len() == 0 {
            error!("No items found in RSS feed: {:?}", channel);
        } else {
            for item in items {
                let news_item = item_to_news_item(feed.id, &item);
                let maybe_id = news_item.save(pool).await;
                match maybe_id {
                    Ok(id) => info!("News item saved successfully: {}", id),
                    Err(e) => info!("News item not saved (possibly duplicate): {}", e)
                }
            }
        }
    }

    Ok(())
}

/// Convert an RSS item to a NewsItem
fn item_to_news_item(feed_id: uuid::Uuid, item: &Item) -> models::NewsItem {
    let title = item.title.clone().or(Some("n/a".to_string())).unwrap();
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
                    error!("{}", date_parse_error(&dt));
                    Utc::now()
                }
            }
        }
        None => Utc::now()
    };
    models::NewsItem::new(feed_id, guid, title, pub_date, url)
}

// write a test for the following function
fn parse_date(dt: &str) -> Option<DateTime<Utc>> {

    let dt = &dt.replace(" GMT", " +0000");

    // ex. 'Tue, 1 Jul 2003 10:52:37 +0200'
    let pr1 = DateTime::parse_from_rfc2822(dt);
    match pr1 {
        Ok(dt) => return Some(dt.with_timezone(&Utc)),
        Err(_) => {}
    }

    // ex. '1996-12-19T16:39:57-08:00'
    let pr2 = DateTime::parse_from_rfc3339(dt);
    match pr2 {
        Ok(dt) => return Some(dt.with_timezone(&Utc)),
        Err(_) => {}
    }

    // see: https://docs.rs/chrono/latest/chrono/format/strftime/index.html
    let datetime_formats = [
        "%a, %d %b %Y %H:%M:%S GMT",
        "%a, %d %b %Y %H:%M:%S %z",
        "%a, %d %b %Y %H:%M:%S %Z",
        "%a, %d %b %Y %H:%M:%S GMT",
        "%a, %e %b %Y %H:%M:%S %Z",
        "%a, %e %b %Y %H:%M:%S GMT",
    ];

    for format in &datetime_formats {
        let pr3 = DateTime::parse_from_str(dt, format);
        match pr3 {
            Ok(dt) => return Some(dt.with_timezone(&Utc)),
            Err(_) => continue
        }
    }

    let naivedate_formats = [
        "%a, %d %b %Y",
        "%a, %e %b %Y",
        "%Y-%m-%d",
        "%Y-%M-%d",
    ];

    for format in &naivedate_formats {
        let pr4 = NaiveDate::parse_from_str(dt, format);
        match pr4 {
            Ok(d) => {
                let naive_date_time = d.and_hms_opt(0, 0, 0).unwrap();
                let dt = DateTime::<Utc>::from_utc(naive_date_time, Utc);
                return Some(dt);
            },
            Err(_) => continue
        }
    }


    None
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn test_another() {
        let date_str = "Wed, 31 May 2023";
        let parsed_date = parse_date(date_str);
        assert!(parsed_date.is_some(), "Unable to parse");
    }

    #[test]
    fn test_parse_date_valid_no_time() {
        let date_str = "2023-06-19";
        let parsed_date = parse_date(date_str);
        assert!(parsed_date.is_some(), "Expected Some, got None.");
    }
    /*#[test]
    fn test_parse_date_valid() {
        let parsed_date = parsed_date.unwrap();
        let expected_date: DateTime<Utc> = "2023-06-19T00:00:00Z".parse().unwrap();
        assert_eq!(parsed_date, expected_date, "Dates do not match.");
    }*/

    #[test]
    fn test_parse_date_invalid() {
        let date_str = "Not a date";
        let parsed_date = parse_date(date_str);
        assert!(parsed_date.is_none(), "Expected None, got Some.");
    }

    #[test]
    fn test_parse_date_with_time() {
        let date_str = "Wed, 01 Jan 2020 12:34:56 GMT";
        let parsed_date = parse_date(date_str);
        assert!(parsed_date.is_some(), "Expected Some, got None.");

        let parsed_date = parsed_date.unwrap();
        let expected_date: DateTime<Utc> = "2020-01-01T12:34:56Z".parse().unwrap();
        assert_eq!(parsed_date, expected_date, "Dates do not match.");
    }
}

fn date_parse_error(date: &str) -> String {
    format!("Failed to parse date and time: '{}'", date)
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

async fn handle_atom_feed(dir_path: &str, feed_content: &str, save_content_files: bool) -> Result<Feed, atom_syndication::Error> {
    let feed_parsed = Feed::read_from(feed_content.as_bytes());
    let parsed_file_path = format!("{}/{}", dir_path, "feed-parsed.json");
    match &feed_parsed {
        Ok(feed) => {
            let feed_parsed_json = serde_json::to_string_pretty(&feed).expect("Unable to serialize Atom feed");
            let mut feed_parsed_file: fs::File = fs::File::create(&parsed_file_path).expect("Unable to create feed parsed file");
            feed_parsed_file.write_all(&feed_parsed_json.as_bytes()).expect("Unable to write feed parsed file");

            // for each feed entry save the html content to a file
            if save_content_files {
                save_atom_content(dir_path, &feed).await.expect("Unable to save Atom content");
            }
        }
        Err(err) => info!("Not a RSS feed: {}", err),
    }

    return feed_parsed;
}

async fn save_atom_content(dir_path: &str, feed: &Feed) -> Result<(), Error> {
    // create "content" directory under dir_path
    let content_dir_path = format!("{}/{}", dir_path, "content");
    fs::create_dir_all(&content_dir_path).expect("Unable to create content directory");

    let entries: Vec<Entry> = feed.clone().entries;
    if entries.len() == 0 {
        error!("No entries found in Atom feed");
    } else {
        for entry in entries {
            let title = entry.title.clone().value;
            let title_slug = slug::slugify(title);
            let maybe_content_url = get_atom_content_url(&entry.links).await;
            download_content(&content_dir_path, &title_slug, maybe_content_url).await;
        }
    }

    Ok(())
}

async fn get_atom_content_url(links: &Vec<Link>) -> Option<String> {
    if links.len() == 1 {
        return Some(links.first().unwrap().href.clone());
    }
    // iterate over entry.links and find the one with mime type "text/html"
    return links.into_iter().find(|link| {
        if link.href.ends_with(".html") || link.href.ends_with(".htm") {
            return true;
        }
        let maybe_mime_type = link.mime_type.clone();
        match maybe_mime_type {
            Some(mime_type) => mime_type == "text/html",
            None => false
        }
    }).map(|lnk| lnk.clone().href);
}

async fn download_content(content_dir: &String,
                          title_slug: &String,
                          maybe_content_url: Option<String>) {
    match maybe_content_url {
        None => {
            error!("No content URL found for item: {}", title_slug.clone());
            return;
        }
        Some(url) => {
            let valid_url = validate_url(&url).await;
            if !valid_url {
                error!("Attempted download with invalid URL: {}", url);
                return;
            }
            let safe_title_slug = safe_filename(&title_slug).await;
            let content_file_path = format!("{}/{}.html", content_dir, safe_title_slug);
            let mut item_content_file: fs::File = fs::File::create(&content_file_path).expect("Unable to create content file");
            if !validate_url(&url).await {
                return;
            }

            let maybe_content = playwright_fetch(&url).await;
            match maybe_content {
                None => {
                    error!("Unable to fetch content for item: {}", title_slug.clone());
                    return;
                },
                Some(content) => {
                    let _ = item_content_file.write_all(content.as_bytes());
                }
            }
        }
    }
}

async fn safe_filename(orig: &String) -> &str {
    if orig.len() <= 100 {
        orig
    } else {
        &orig[..100]
    }
}

async fn handle_rss_feed(dir_path: &str, feed_content: String, save_content_files: bool) -> Result<Channel, rss::Error> {
    let feed_parsed = Channel::read_from(feed_content.as_bytes());
    let parsed_file_path = format!("{}/{}", dir_path, "feed-parsed.json");
    match &feed_parsed {
        Ok(channel) => {
            let feed_parsed_json = serde_json::to_string_pretty(&channel).expect("Unable to serialize RSS channel");
            let mut feed_parsed_file: fs::File = fs::File::create(&parsed_file_path).expect("Unable to create feed parsed file");
            feed_parsed_file.write_all(&feed_parsed_json.as_bytes()).expect("Unable to write feed parsed file");

            if save_content_files {
                save_rss_content(dir_path, &channel).await.expect("Unable to save RSS content");
            }
        }
        Err(err) => warn!("Error parsing RSS feed: {}", err),
    }

    return feed_parsed;
}

async fn save_rss_content(dir_path: &str, channel: &Channel) -> Result<(), Error> {
    // create "content" directory under dir_path
    let content_dir_path = format!("{}/{}", dir_path, "content");
    fs::create_dir_all(&content_dir_path).expect("Unable to create content directory");

    let items: Vec<Item> = channel.clone().items;
    if items.len() == 0 {
        error!("No items found in RSS channel: {:?}", channel);
    } else {
        for item in items {
            match item.title {
                None => {
                    error!("No title found for item: {:?}", item);
                    continue;
                }
                Some(title) => {
                    let title_slug = slug::slugify(title.clone());
                    let maybe_content_url = item.link.clone();
                    download_content(&content_dir_path, &title_slug, maybe_content_url).await;
                }
            }
        }
    }
    Ok(())
}

/// Using playwright, fetch the content of the URL
#[allow(dead_code)]
async fn playwright_fetch(url: &str) -> Option<String> {
    info!("Fetching URL: {}", url);
    let playwright = Playwright::initialize().await.expect("Unable to initialize playwright");
    playwright.prepare().expect("Error installing browsers");
    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.expect("Unable to launch browser");
    let context = browser.context_builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")
        .build().await.expect("Unable to build context");
    let page = context.new_page().await.expect("Unable to create page");
    page.goto_builder(url).goto().await.expect("Unable to navigate to page");
    page.content().await.ok()
}

async fn validate_url(url: &str) -> bool {
    let url_parsed = Url::parse(url);
    match url_parsed {
        Ok(u) => {
            // Check if the URL has a scheme (e.g., http, https)
            if u.scheme().is_empty() {
                error!("Invalid URL (no scheme): {}", u);
                return false;
            }

            // Check if the URL has a host
            if u.host().is_none() {
                error!("Invalid URL (no host): {}", u);
                return false;
            }
            true
        },
        Err(e) => {
            error!("Invalid URL: {}; {}", url, e);
            false
        }
    }
}
