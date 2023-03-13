use std::{
    io::{self, prelude::*, BufReader, Result as IoResult},
    net::TcpStream,
    collections::HashMap
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
            print!("{command_line}");
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
                    let mut byte_counter = 0;
                    let http_response = String::from_utf8(http_response).unwrap();

                    // Status
                    if !status_line.ends_with("\r\n") {
                        status_line.push_str(&http_response[0..http_response.find('\n').unwrap() + 1]);
                        byte_counter = status_line.len();
                    }

                    // Header
                    if !header.ends_with("\r\n\r\n") {
                        header.push_str(&http_response[byte_counter..http_response.find('<').unwrap_or(http_response.len())]);
                        byte_counter = http_response.find('<').unwrap_or(0);
                    }

                    // Body (might only deal with HTML for now)
                    body.push_str(&http_response[byte_counter..http_response.len()]);

                    // Response Processing
                    // >> Status Line
                    let proc_status_line: Vec<String> = status_line
                        .splitn(3, ' ')
                        .map(|s| String::from(s.trim_end()))
                        .collect();

                    println!("Status Line: {:?}", proc_status_line);
                    
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

                    // >> Body
                    let proc_body = body
                        .trim_end()
                        .to_string();

                    println!("Body: {}", proc_body);

                    //  Response Handling
                    // >> Non 2XX Response Code Handling
                    let (response_code, message) = (proc_status_line[1].clone().parse::<usize>().unwrap(), proc_status_line[2].clone());
                    if response_code == 401 { // HTTP Basic Auth
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
                        println!("ERROR: {response_code} {message}");
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
                        continue;
                    }

                    break;
                }

                println!("\nNOTICE: Finished web page fetching attempt");
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
