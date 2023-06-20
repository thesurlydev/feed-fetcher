use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Read;
use kuchiki::traits::*;
use kuchiki::{NodeRef};

pub(crate) async fn extract_text_from_str(content: String, ignore_whitespace: bool) -> Result<String, Box<dyn Error>> {
    let document = kuchiki::parse_html().from_utf8().read_from(&mut content.as_bytes())?;
    process_doc(document, ignore_whitespace).await
}

pub(crate) async fn extract_text_from_file(path: String, ignore_whitespace: bool) -> Result<String, Box<dyn Error>> {
    let mut input: Box<dyn io::Read> = Box::new(std::fs::File::open(path).expect("Failed to open file"));
    let document = kuchiki::parse_html().from_utf8().read_from(&mut input)?;
    process_doc(document, ignore_whitespace).await
}

pub(crate) async fn process_doc(document: NodeRef, ignore_whitespace: bool) -> Result<String, Box<dyn Error>> {
    let mut output_buffer = String::new();
    let select_result = document.select("body");
    match select_result {
        Ok(select) => {
            select
                .for_each(|matched_noderef| {
                    let node = matched_noderef.as_node();
                    let part = serialize_text(node, ignore_whitespace);
                    output_buffer.push_str(&part);
                });
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    Ok(output_buffer)
}


fn serialize_text(node: &NodeRef, ignore_whitespace: bool) -> String {
    let mut result = String::new();
    for text_node in node.inclusive_descendants().text_nodes() {
        if ignore_whitespace && text_node.borrow().trim().is_empty() {
            continue;
        }

        result.push_str(&text_node.borrow());

        if ignore_whitespace {
            result.push('\n');
        }
    }

    result
}

pub(crate) fn get_test_content(path: String) -> String {
    let mut file: File = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents
}