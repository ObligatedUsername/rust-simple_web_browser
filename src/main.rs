extern crate base64;
extern crate html_parser;
extern crate spinners;

use base64::{engine::general_purpose, Engine as _};
use html_parser::{Dom, Element as RealElement, Node::*};
use spinners::{Spinner, Spinners};
use std::{
    collections::HashMap,
    fs::{self, DirBuilder, File},
    io::{self, prelude::*, stdout, BufReader, Result as IoResult},
    net::TcpStream,
};

// find_subsequence by Francis Gagné on StackOverflow
// Find the starting index of the byte subset "needle" in "haystack"
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// recursive_elem_print
// Recursively does indented & formatted prints of elements from top to bottom
// Notes for certain elements:
// ---- only lists are indented, everything else follows their current depth,
// ---- TODO: open links
fn recursive_elem_print(curr_elem: &RealElement, indent: &str, indent_depth: usize, extras: &str) {
    if !curr_elem.children.is_empty() {
        for child_elem in curr_elem.children.iter() {
            match child_elem {
                Element(elem) => match elem.name.as_str() {
                    "ol" | "ul" => {
                        recursive_elem_print(elem, indent, indent_depth + 1, extras);
                    }
                    "a" => {
                        recursive_elem_print(
                            elem,
                            indent,
                            indent_depth,
                            format!(
                                " -> {}",
                                elem.attributes
                                    .get(&String::from("href"))
                                    .cloned()
                                    .unwrap()
                                    .unwrap()
                            )
                            .as_str(),
                        );
                    }
                    "script" => {}
                    _ => {
                        recursive_elem_print(elem, indent, indent_depth, extras);
                    }
                },
                Text(text) => {
                    println!("{}{}{}", indent.repeat(indent_depth), text, extras);
                }
                Comment(_) => {}
            }
        }
    }
}

// read_n by Shepmaster on StackOverflow
// Read N amount of bytes from reader
// fn read_n<R>(reader: R, bytes_to_read: u64) -> Vec<u8>
// where
//     R: Read,
// {
//     let mut buf = vec![];
//     reader.take(bytes_to_read).read_to_end(&mut buf).unwrap();
//     buf
// }

// const PACKET_MAX_BYTES: usize = 4096;

fn main() -> IoResult<()> {
    // commands -> <command, arguments>
    let commands: HashMap<&str, [&str; 2]> = HashMap::from([
        ("open", ["[URI]:[PORT]/[URN]", "\"Opens a web page from the given URL.\""]),
        ("download", ["[URI]:[PORT]/[URN]", "\"Downloads file from the given URL. (Supported file types so far are: html, txt, pdf)\""]),
        ("help", ["", "\"Shows this message.\""]),
        ("quit", ["", "\"Exit from this program.\""]),
    ]);
    // supported_download_file_types -> <mime_type, MIME type>
    let supported_download_file_types: HashMap<&str, &str> = HashMap::from([
        // Text-only types
        ("text/plain", "txt"),
        ("text/csv", "csv"),
        ("text/css", "css"),
        ("text/html", "html"),
        ("text/javascript", "js"),
        // Default binary type
        ("application/octet-stream", "bin"),
        // Image types
        ("image/apng", "apng"),
        ("image/png", "png"),
        ("image/avif", "avif"),
        ("image/gif", "gif"),
        ("image/jpeg", "jpg"),
        ("image/svg+xml", "svg"),
        ("image/webp", "webp"),
        ("image/bmp", "bmp"),
        ("image/tiff", "tiff"),
        ("image/vnd.microsoft.icon", "ico"),
        // Audio types
        ("audio/wav", "wav"),
        ("audio/webm", "webm"),
        ("audio/ogg", "ogg"),
        ("audio/aac", "aac"),
        ("audio/wav", "wav"),
        ("audio/mpeg", "mp3"),
        ("audio/mp4", "m4a"),
        ("audio/opus", "opus"),
        ("audio/midi", "midi"),
        // Video types
        ("video/webm", "webm"),
        ("video/ogg", "ogg"),
        ("video/mp4", "mp4"),
        ("video/mpeg", "mpeg"),
        // Font types
        ("font/otf", "otf"),
        ("font/ttf", "ttf"),
        ("font/woff", "woff"),
        ("font/woff2", "woff2"),
        // Application types
        ("application/pdf", "pdf"),
        ("application/ogg", "ogg"),
        ("application/vnd.rar", "rar"),
        ("application/zip", "zip"),
        ("application/x-7z-compressed", "7z"),
        ("application/x-bzip", "bz"),
        ("application/x-bzip2", "bz2"),
        ("application/gzip", "gz"),
        ("application/x-tar", "tar"),
        ("application/json", "json"),
        ("application/x-httpd-php", "php"),
        ("application/x-sh", "sh"),
        ("application/xhtml+xml", "xhtml"),
        ("application/xml", "xml"),
        ("application/msword", "doc"),
        (
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "docx",
        ),
        ("application/vnd.ms-powerpoint", "ppt"),
        (
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "pptx",
        ),
        ("application/vnd.ms-excel", "xls"),
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "xlsx",
        ),
    ]);

    // Command Configuration
    let mut command_help =
        String::from("==== \"Simple\" Web Browser! ====\n====  Available Commands:  ====\n");
    for (c_command, c_args) in &commands {
        command_help.push_str(
            format!("    {} {}\n        {}\n\n", c_command, c_args[0], c_args[1]).as_str(),
        );
    }
    command_help.push_str("INFO: URL and PORT defaults to 'localhost' and '80' respectively.\n");

    let (mut url, mut port, mut urn);
    let mut auth = String::new();

    // User Interface -- Native CLI
    println!("{command_help}");

    let mut command_line = String::new();
    loop {
        // Command Line Input & Processing
        print!("> ");
        stdout().flush().unwrap();

        if let Err(e) = io::stdin().read_line(&mut command_line) {
            println!("ERROR: {e}");
        }
        println!();

        let (command, args): (String, Vec<Vec<String>>) = command_line
            .trim()
            .split_once(' ')
            .map(|t| {
                (
                    String::from(t.0),
                    t.1.trim_start_matches("http://")
                        .splitn(2, '/')
                        .map(|s1| String::from(s1).split(':').map(String::from).collect())
                        .collect(),
                )
            })
            .unwrap_or((
                command_line
                    .split(' ')
                    .next()
                    .unwrap()
                    .trim_end()
                    .to_string(),
                vec![],
            ));

        if !command.is_empty() {
            if ["open", "download"].contains(&command.as_str()) {
                url = if args.is_empty() {
                    String::from("localhost")
                } else {
                    args[0][0].clone()
                };
                port = if args.is_empty() || args[0].len() == 1 {
                    String::from("80")
                } else {
                    args[0][1].clone()
                };
                urn = if args.len() < 2 {
                    String::from("")
                } else {
                    args[1][0].clone()
                };

                // Request Handling
                loop {
                    let mut sp =
                        Spinner::new(Spinners::Aesthetic, "Please wait for a bit...".into());

                    let mut stream = TcpStream::connect(format!("{url}:{port}"))?;
                    let request = format!("GET /{urn} HTTP/1.0\r\nHost: {url}{auth}\r\n\r\n");

                    auth = String::new();

                    stream.write_all(&request.into_bytes())?;
                    stream.flush()?;

                    let mut stream_buf_reader = BufReader::new(&mut stream);

                    // Parser
                    let (mut status_line, mut header, mut body) =
                        (String::new(), String::new(), vec![]);
                    let mut http_response = vec![];
                    stream_buf_reader.read_to_end(&mut http_response)?;
                    let mut byte_counter;
                    let http_response = http_response.as_slice();

                    // Status
                    status_line.push_str(&String::from_utf8_lossy(
                        &http_response[..find_subsequence(http_response, b"\r\n").unwrap() + 2],
                    ));
                    byte_counter = status_line.len();

                    // Header
                    header.push_str(&String::from_utf8_lossy(
                        &http_response[byte_counter
                            ..find_subsequence(http_response, b"\r\n\r\n").unwrap() + 4],
                    ));
                    byte_counter = find_subsequence(http_response, b"\r\n\r\n").unwrap() + 4;

                    // Body (might only deal with HTML for now)
                    body.append(&mut http_response[byte_counter..http_response.len()].to_owned());

                    sp.stop_and_persist("✔", "Finished loading web page!".into());

                    // Response Processing
                    // >> Status Line
                    let proc_status_line: Vec<String> = status_line
                        .splitn(3, ' ')
                        .map(|s| String::from(s.trim_end()))
                        .collect();

                    // >> Header
                    let mut proc_header: HashMap<String, Vec<Vec<_>>> = HashMap::new();
                    for line in header.lines() {
                        if line.is_empty() {
                            break;
                        }
                        let parts = line.split_once(' ').unwrap();
                        proc_header.insert(
                            String::from(parts.0.trim_end_matches(':')),
                            parts
                                .1
                                .split(';')
                                .map(|s1| {
                                    String::from(s1.trim())
                                        .split('=')
                                        .map(String::from)
                                        .collect()
                                })
                                .collect(),
                        );
                    }

                    // >> Body
                    let proc_body = if body.ends_with(b"\n") {
                        &body[..body.len() - 1]
                    } else {
                        &body
                    };

                    // Response Handling
                    // >> Non 2XX Response Code Handling
                    let (response_code, message) = (
                        proc_status_line[1].clone().parse::<usize>().unwrap(),
                        proc_status_line[2].clone(),
                    );
                    if response_code == 401 {
                        // HTTP Basic Auth
                        println!();
                        println!("NOTICE: Authorization is Needed, please enter your username and password below, separated by a space");
                        println!("(you may ENTER if you don't wish to input your credentials.):");

                        if let Err(e) = io::stdin().read_line(&mut auth) {
                            println!("ERROR: {e}");
                        }

                        // TODO: clean up auth if the user inputs a new line, and
                        //       handle invalid auth
                        if auth == "\n" {
                            break;
                        }
                        auth = String::from("\r\nAuthorization: ")
                            + proc_header.get(&String::from("WWW-Authenticate")).unwrap()[0][0]
                                .split(' ')
                                .next()
                                .unwrap()
                            + " "
                            + &general_purpose::STANDARD
                                .encode(auth.replace(' ', ":").trim_end().as_bytes());
                        continue;
                    } else if response_code >= 400 {
                        println!();
                        println!("ERROR: {response_code} {message}");
                        println!();

                        break;
                    }

                    // >> Redirect Checks
                    let check_redirect = proc_header
                        .get(&String::from("Refresh"))
                        .cloned()
                        .unwrap_or(vec![]);
                    if !check_redirect.is_empty() {
                        urn = check_redirect[1][1]
                            .splitn(4, '/')
                            .nth(3)
                            .unwrap()
                            .to_string();

                        println!();
                        println!("NOTICE: Redirecting to {urn}");
                        println!();

                        continue;
                    }

                    let mime_type = &proc_header.get(&String::from("Content-Type")).unwrap()[0][0];

                    if command == "download" {
                        // >> File Downloads
                        let download_file_path = "./downloads";

                        let unnamed_counts = fs::read_dir(download_file_path)?
                            .map(|res| {
                                res.expect("ERROR: failed reading from downloads")
                                    .file_name()
                                    .into_string()
                                    .unwrap()
                            })
                            .collect::<Vec<String>>();
                        let unnamed_counts = unnamed_counts
                            .iter()
                            .filter(|s| s.starts_with("unnamed_"))
                            .map(|s| {
                                s.split(&['_', '.'][..])
                                    .nth(1)
                                    .unwrap()
                                    .parse::<isize>()
                                    .expect("ERROR: wrong format for unnamed file")
                            })
                            .max()
                            .unwrap_or(-1)
                            + 1;

                        let filename =
                            if proc_header.contains_key(&String::from("Content-Disposition")) {
                                proc_header
                                    .get(&String::from("Content-Disposition"))
                                    .unwrap()[1][1]
                                    .trim_matches('\"')
                                    .to_string()
                            } else {
                                format!(
                                    "unnamed_{}.{}",
                                    unnamed_counts,
                                    supported_download_file_types
                                        .get(mime_type.as_str())
                                        .unwrap()
                                )
                            };

                        if supported_download_file_types
                            .keys()
                            .any(|s| s == &mime_type.as_str())
                        {
                            DirBuilder::new()
                                .recursive(true)
                                .create(download_file_path)
                                .unwrap();

                            let mut f = File::create(format!("{download_file_path}/{filename}"))?;
                            f.write_all(proc_body)?;

                            let content_length =
                                proc_header.get(&String::from("Content-Length")).unwrap()[0][0]
                                    .parse::<usize>()
                                    .unwrap();

                            let (size, metric) = match content_length {
                                0..=999 => (content_length as f64, "Bytes"),
                                1_000..=999_999 => (content_length as f64 / 1_000_f64, "KB"),
                                1_000_000..=999_999_999 => {
                                    (content_length as f64 / 1_000_000_f64, "MB")
                                }
                                _ => (content_length as f64 / 1_000_000_000_f64, "GB"),
                            };

                            println!();
                            println!(
                                "NOTICE: Finished downloading {} with the size of {:.1} {}",
                                filename, size, metric
                            );
                            println!();
                        }
                    } else {
                        // println!("Status Line: {:?}", proc_status_line);
                        // println!();

                        // println!("Header: {:?}", proc_header);
                        // println!();

                        // println!("Body:\n{}", String::from_utf8_lossy(proc_body));

                        // HTML Parsing and Simple Display
                        // WARNING: Uses a non-production html parsing library, not sure by how much
                        //          it affects performance so far.
                        if mime_type == "text/html" {
                            let dom = Dom::parse(&String::from_utf8_lossy(proc_body)).unwrap();
                            let html = &dom
                                .children
                                .iter()
                                .last()
                                .unwrap()
                                .element()
                                .unwrap()
                                .children;
                            let (head, body): (RealElement, RealElement) = (
                                html[0].element().unwrap().clone(),
                                html[1].element().unwrap().clone(),
                            );
                            let title = head
                                .children
                                .iter()
                                .find(|e| matches!(e.element(), Some(elem) if elem.name == "title"))
                                .unwrap()
                                .element()
                                .unwrap()
                                .children[0]
                                .text()
                                .unwrap();
                            println!("Title: {}\n", title);
                            recursive_elem_print(&body, "  ", 0, "");
                        }

                        println!();
                        println!("NOTICE: Finished reading {url}:{port}/{urn}");
                        println!();
                    }

                    break;
                }
            } else if command == "help" {
                println!("{command_help}");
            } else if command == "quit" {
                break;
            } else {
                println!("ERROR: Command '{command}' not recognized");
            }
        } else {
            println!("ERROR: Please enter something");
        }
        command_line = String::new();
    }

    Ok(())
}
