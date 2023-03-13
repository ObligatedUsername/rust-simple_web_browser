use std::{
    io::{self, prelude::*, BufReader, Result as IoResult},
    net::TcpStream,
    collections::HashMap,
    fs::{File, DirBuilder}
};
use base64::{
    Engine as _,
    engine::general_purpose
};

// Read N amount of bytes from reader
// fn read_n<R>(reader: R, bytes_to_read: u64) -> Vec<u8>
// where
//     R: Read,
// {
//     let mut buf = vec![];
//     reader.take(bytes_to_read).read_to_end(&mut buf).unwrap();
//     buf
// }

// const PACKET_MAX_BYTES: usize = 512;
const COMMANDS: &str = "    open [URI]:[PORT]/[URN]\n    quit\n";
const SUPPORTED_FILE_TYPES: [&str; 1] = ["text/plain"];

fn main() -> IoResult<()> {
    let (mut url, mut port, mut urn);
    let mut auth = String::new();

    // User Interface -- Native CLI
    println!("==== \"Simple\" Web Browser! ====");
    println!("====  Available Commands:  ====");
    print!("{COMMANDS}");

    let mut command_line = String::new();
    loop {
        // Command Line Input & Processing
        print!("> ");
        match io::stdin().read_line(&mut command_line) {
            Err(e) => { println!("error: {e}"); },
            _ => {}
        }
        if command_line.starts_with("http://") { command_line = command_line["http://".len()..].to_string(); }

        let (command, args): (String, Vec<Vec<String>>) = command_line
            .trim()
            .split_once(' ')
            .map(|t| (String::from(t.0), t.1
                      .splitn(2, '/')
                      .map(|s1| String::from(s1)
                           .split(':')
                           .map(|s2| String::from(s2))
                           .collect())
                      .collect()))
            .unwrap_or((command_line.split(' ').next().unwrap().trim_end().to_string(), vec![]));

        if !command.is_empty() {
            println!("{command_line}");
            if command == "open" {
                url = args.get(0).cloned().unwrap_or(vec![String::from("localhost")]).get(0).cloned().unwrap();
                port = args.get(0).cloned().unwrap_or(vec![String::new(), String::from("3000")]).get(1).cloned().unwrap_or(String::from("3000"));
                urn = args.get(1).cloned().unwrap_or(vec![String::from("")]).get(0).cloned().unwrap_or(String::from(""));

                // Request Handling
                loop {
                    let mut stream = TcpStream::connect(format!("{url}:{port}"))?;
                    let request = format!("GET /{urn} HTTP/1.1\r\nHost: {url}{auth}\r\n\r\n");
                    
                    auth = String::new();

                    stream.write_all(&request.into_bytes())?;
                    stream.flush()?;

                    let mut stream_buf_reader = BufReader::new(&mut stream);

                    // Parser
                    let (mut status_line, mut header, mut body) = (String::new(), String::new(), String::new());
                    let mut http_response = vec![];
                    stream_buf_reader.read_to_end(&mut http_response)?;
                    let mut byte_counter;
                    let http_response = String::from_utf8_lossy(http_response.as_slice());

                    // Status
                    status_line.push_str(&http_response[0..http_response.find("\r\n").unwrap() + 2]);
                    byte_counter = status_line.len();

                    // Header
                    header.push_str(&http_response[byte_counter..http_response.find("\r\n\r\n").unwrap() + 4]);
                    byte_counter = http_response.find("\r\n\r\n").unwrap() + 4;

                    // Body (might only deal with HTML for now)
                    body.push_str(&http_response[byte_counter..http_response.len()]);

                    // Response Processing
                    // >> Status Line
                    let proc_status_line: Vec<String> = status_line
                        .splitn(3, ' ')
                        .map(|s| String::from(s.trim_end()))
                        .collect();

                    // println!("Status Line: {:?}", proc_status_line);
                    
                    // >> Header
                    let mut proc_header: HashMap<String, Vec<Vec<_>>> = HashMap::new();
                    for line in header.lines() {
                        if line.is_empty() { break; }
                         let parts = line
                             .split_once(' ')
                             .unwrap();
                         proc_header.insert(
                             String::from(parts.0.trim_end_matches(':')),
                             parts.1
                                .split(';')
                                .map(|s1| String::from(s1.trim())
                                    .split('=')
                                    .map(|s2| String::from(s2))
                                    .collect())
                                .collect()
                            );
                    }

                    println!("Header: {:?}", proc_header);
                    println!();

                    // >> Body
                    let proc_body = body
                        .trim_end()
                        .to_string();

                    println!("Body:\n{}", proc_body);

                    // Response Handling
                    // >> Non 2XX Response Code Handling
                    let (response_code, message) = (proc_status_line[1].clone().parse::<usize>().unwrap(), proc_status_line[2].clone());
                    if response_code == 401 { // HTTP Basic Auth
                        println!();
                        println!("NOTICE: Authorization is Needed, please enter your username and password below, separated by a space");
                        println!("(you may ENTER if you don't wish to input your credentials.):");
                        match io::stdin().read_line(&mut auth) {
                            Err(e) => { println!("error: {e}"); },
                            _ => {}
                        }

                        if auth == "\n" { break; }
                        auth = String::from("\r\nAuthorization: ") +
                            &proc_header.get(&String::from("WWW-Authenticate"))
                                .unwrap().get(0).unwrap().get(0).unwrap()
                                .split(' ').next().unwrap() +
                            " " +
                            &general_purpose::STANDARD.encode(auth.replace(" ", ":").trim_end().as_bytes());
                        continue;
                    }
                    else if response_code >= 400 {
                        println!();
                        println!("!!!!!!!!");
                        println!("ERROR: {response_code} {message}");
                        println!("!!!!!!!!");
                        println!();

                        break;
                    }

                    // >> Redirect Checks
                    let check_redirect = proc_header.get(&String::from("Refresh")).cloned().unwrap_or(vec![]);
                    if !check_redirect.is_empty() {
                        urn = check_redirect
                            .get(1).cloned().unwrap()
                            .get(1).cloned().unwrap()
                            .clone();
                        urn = urn.splitn(4, '/').skip(3).next().unwrap().to_string();

                        println!();
                        println!("!!!!!!!!");
                        println!("NOTICE: Redirecting to {urn}");
                        println!("!!!!!!!!");
                        println!();

                        continue;
                    }

                    // >> File Downloads
                    if proc_header.get(&String::from("Content-Type")).is_some() {
                        let filename = proc_header.get(&String::from("Content-Disposition")).unwrap_or(&vec![vec![]])
                        .get(1).unwrap_or(&vec![]).get(1).unwrap_or(&String::from("unnamed")).trim_matches('\"').to_string();

                        let filetype = proc_header.get(&String::from("Content-Type")).unwrap()
                            .get(0).unwrap().get(0).unwrap();
                        if SUPPORTED_FILE_TYPES.iter().any(|s| s == filetype) {
                            let downloaded_file_path = String::from("./downloads");
                            DirBuilder::new()
                                .recursive(true)
                                .create(downloaded_file_path.clone())
                                .unwrap();

                            let mut f = File::create(downloaded_file_path + "/" + &filename)?;
                            f.write_all(proc_body.as_bytes())?;

                            println!();
                            println!("!!!!!!!!");
                            println!("NOTICE: Finished downloading {} with the size of {}", filename, proc_body.as_bytes().len());
                            println!("!!!!!!!!");
                            println!();

                            break;
                        }
                    }

                    // TODO: HTML Parsing and Simple Display

                    println!();
                    println!("!!!!!!!!");
                    println!("NOTICE: Finished reading {url}:{port}/{urn}");
                    println!("!!!!!!!!");
                    println!();

                    break;
                }

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
