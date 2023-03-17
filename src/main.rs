use ncurses::*;
use base64::{engine::general_purpose, Engine as _};
use html_parser::{Dom, Element as RealElement, Node::*};
use std::{
    collections::HashMap,
    fs::{self, DirBuilder, File},
    io::{prelude::*, BufReader, Result as IoResult},
    net::TcpStream,
    thread,
    time::Duration,
    sync::mpsc,
};

// find_subsequence by Francis Gagné on StackOverflow
// Find the starting index of the byte subset "needle" in "haystack"
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// recursive_elem_vec_fill
// Recursively fill a vector with formatted string of elements from top to bottom
// Notes for certain elements:
// ---- only lists are indented, everything else follows their current depth,
// ---- TODO: open links (Setup a better data structure for these elements) (just use a huge
//            string)
fn recursive_elem_vec_fill(curr_elem: &RealElement, indent: &str, indent_depth: usize, extras: &str)
    -> Vec<String> {
    let mut elem_vec: Vec<String> = vec![];
    if !curr_elem.children.is_empty() {
        for child_elem in curr_elem.children.iter() {
            match child_elem {
                Element(elem) => match elem.name.as_str() {
                    "ol" | "ul" => {
                        elem_vec.append(&mut recursive_elem_vec_fill(elem, indent, indent_depth + 1, format!("{}|{}",
                                            elem.name,
                                            elem.attributes
                                            .iter()
                                            .map(|(key, value)| format!("{}:{}", key, value.as_ref().unwrap()))
                                            .collect::<Vec<String>>()
                                            .join(";"))
                                    .as_str()));
                    }
                    "a" => {
                        elem_vec.append(&mut recursive_elem_vec_fill(elem, indent, indent_depth, format!("{}|{}",
                                            elem.name,
                                            elem.attributes
                                            .iter()
                                            .map(|(key, value)| format!("{}:{}", key, value.as_ref().unwrap()))
                                            .collect::<Vec<String>>()
                                            .join(";"))
                                    .as_str()));
                    }
                    "script" => {}
                    _ => {
                        elem_vec.append(&mut recursive_elem_vec_fill(elem, indent, indent_depth, format!("{}|{}",
                                            elem.name,
                                            elem.attributes
                                            .iter()
                                            .map(|(key, value)| format!("{}:{}", key, value.as_ref().unwrap()))
                                            .collect::<Vec<String>>()
                                            .join(";"))
                                    .as_str()));
                    }
                },
                Text(text) => {
                    elem_vec.push(format!("{}{} >> {}", indent.repeat(indent_depth), text, extras));
                },
                _ => {}
            }
        }
    }
    elem_vec
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
    command_help.push_str("FYI, URL and PORT defaults to 'localhost' and '80' respectively.\nPress tab to switch between web page and command line view.\n");

    let (mut url, mut port, mut urn);
    let mut auth = String::new();

    // User Interface -- ncurses
    let screen = initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    // Web Page and View
    let mut page_view = false;
    let mut page_title = String::new();
    let mut elem_vec: Vec<String> = vec![];

    // Fetch and Download Web Pages and Files
    addstr(&command_help);

    addstr("> ");
    let cmd_line_curr_y = getcury(screen);
    
    let mut command_line = String::new();
    'cmd_line: loop {
        refresh();

        'cmd_line_input: loop {
            let ch = getch();

            match ch as u8 {
                10 if !page_view => {
                    break 'cmd_line_input;
                },
                9 => {
                    page_view = !page_view;
                    if page_view {
                        erase();
                        if elem_vec.is_empty() { addstr("You haven't loaded any site.\nLoad a website through the command line!"); }
                        else {
                            addstr(page_title.as_str());
                            for elem in &elem_vec {
                                addstr(format!("{elem}\n").as_str());
                            }
                        }
                    }
                    else {
                        erase();
                        addstr(&command_help);
                        addstr("> ");
                        addstr(&command_line);
                    }
                    refresh();
                },
                127 if !page_view => {
                    if getcurx(screen) < 3 { continue; }
                    mvdelch(getcury(screen), getcurx(screen)-1);
                    command_line.pop();
                },
                _ => {
                    if page_view { continue; }
                    addch(ch as u32);
                    command_line.push_str(&(ch as u8 as char).to_string());
                }
            }
        }
        
        // Clear feedback from previous input
        clrtobot();

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
                'webpage_load: loop {
                    // Loading indicator starts here
                    let (tx, rx) = mpsc::channel::<Option<&str>>();

                    mv(cmd_line_curr_y + 4, 0);
                    addstr("Loading");
                    refresh();
                    let cmd_line_curr_x = getcurx(screen);
                    thread::spawn(move || 'loading: loop {
                        for step in 0..=3 {
                            thread::sleep(Duration::from_secs_f64(0.25));
                            let (stop, stop_message) = match rx.try_recv() {
                                Ok(stop_message) => (true, stop_message),
                                Err(mpsc::TryRecvError::Disconnected) => (true, None),
                                Err(mpsc::TryRecvError::Empty) => (false, None),
                            };

                            if step == 3 {
                                mv(cmd_line_curr_y + 4, cmd_line_curr_x);
                                clrtoeol();
                            } else {
                                addstr(match stop_message {
                                    Some(message) => {
                                        mv(cmd_line_curr_y + 4, 0);
                                        clrtoeol();
                                        message
                                    },
                                    None => "."
                                });
                            }
                            refresh();

                            if stop {
                                mv(cmd_line_curr_y , 2);
                                break 'loading;
                            }
                        }
                    });

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

                    // Stop loading indicator here
                    tx.send(Some("Loading finished!")).unwrap();

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
                        mv(cmd_line_curr_y + 2, 0);
                        addstr("INFO: Authorization is needed, please enter your username and password, separated by a space\n(you may ENTER if you don't wish to input your credentials.):");
                        mv(cmd_line_curr_y, 2);
                        clrtoeol();

                        'auth_input: loop {
                            let ch = getch();

                            match ch as u8 {
                                10 => {
                                    break 'auth_input;
                                },
                                9 => {},
                                127 => {
                                    if getcurx(screen) < 3 { continue; }
                                    mvdelch(cmd_line_curr_y, getcurx(screen)-1);
                                    auth.pop();
                                },
                                _ => {
                                    addch(ch as u32);
                                    auth.push_str(&(ch as u8 as char).to_string());
                                }
                            }
                        }
                        mvdelch(cmd_line_curr_y, 2);
                        clrtobot();

                        if !auth.contains(' ') {
                            auth = String::new();
                            break 'webpage_load;
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
                        mv(cmd_line_curr_y + 2, 0);
                        addstr(format!("ERROR: {response_code} {message}").as_str());
                        mv(cmd_line_curr_y, 2);
                        clrtoeol();

                        break 'webpage_load;
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

                        mv(cmd_line_curr_y + 2, 0);
                        addstr(format!("INFO: Redirecting to {urn}").as_str());
                        mv(cmd_line_curr_y, 2);
                        clrtoeol();

                        continue;
                    }

                    if proc_header.get(&String::from("Content-Type")).is_none() {
                        mv(cmd_line_curr_y + 2, 0);
                        addstr(format!("ERROR: Content type is not known").as_str());
                        mv(cmd_line_curr_y, 2);
                        clrtoeol();
                        break 'webpage_load;
                    }

                    let mime_type = &proc_header.get(&String::from("Content-Type")).unwrap().clone()[0][0];

                    if command == "download" {
                        // >> File Downloads
                        let download_file_path = "./downloads";

                        DirBuilder::new()
                            .recursive(true)
                            .create(download_file_path)
                            .unwrap();

                        let unnamed_counts = fs::read_dir(download_file_path)?
                            .map(|res| {
                                res.unwrap()
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
                                    .unwrap()
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

                            mv(cmd_line_curr_y + 2, 0);
                            addstr(format!(
                                    "INFO: Finished downloading {} with the size of {:.1} {}",
                                filename, size, metric)
                                .as_str());
                            mv(cmd_line_curr_y, 2);
                            clrtoeol();
                        }
                    } else {
                        // Clear saved previous web page
                        elem_vec = vec![];
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
                            page_title = format!("Title: {}\n\n", title);
                            elem_vec.append(&mut recursive_elem_vec_fill(&body, "  ", 0, ""));
                        }

                        mv(cmd_line_curr_y + 2, 0);
                        addstr(&format!("INFO: Finished reading {url}:{port}/{urn}"));
                        mv(cmd_line_curr_y, 2);
                        clrtoeol();
                    }

                    break 'webpage_load;
                }
            } else if command == "quit" {
                break 'cmd_line;
            } else {
                mv(cmd_line_curr_y + 2, 0);
                addstr(&format!("ERROR: Command '{command}' not recognized"));
                mv(cmd_line_curr_y, 2);
                clrtoeol();
            }
        } else {
            mv(cmd_line_curr_y + 2, 0);
            addstr("ERROR: Please enter something");
            mv(cmd_line_curr_y, 2);
            clrtoeol();
        }
        command_line = String::new();
    }
    
    endwin();

    Ok(())
}
