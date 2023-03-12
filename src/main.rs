use std::{
    io::{self, prelude::*, BufReader, Result as IoResult},
    net::TcpStream,
    collections::HashMap
};

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
    let (mut uri, mut port, mut urn);

    // User Interface -- Native CLI
    println!("==== \"Simple\" Web Browser! ====");
    println!("====  Available Commands:  ====");
    print!("{COMMANDS}");

    let mut command_line = String::new();
    loop {
        // Command Line Input & Processing
        print!("> ");
        match io::stdin().read_line(&mut command_line) {
            Err(e) => { println!("ERROR: {e}"); },
            _ => {}
        }

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
        print!("{command_line}");

        if !command.is_empty() {
            command_line = String::new();
            if command == "open" {
                uri = args.get(0).cloned().unwrap_or(vec![String::from("localhost")]).get(0).cloned().unwrap();
                port = args.get(0).cloned().unwrap_or(vec![String::new(), String::from("3000")]).get(1).cloned().unwrap_or(String::from("3000"));
                urn = args.get(1).cloned().unwrap_or(vec![String::from("")]).get(0).cloned().unwrap_or(String::from(""));

                // Request Handling
                loop {
                    let mut stream = TcpStream::connect(format!("{uri}:{port}"))?;
                    let request = format!("GET /{urn} HTTP/1.1\r\nHost: {uri}\r\n\r\n");
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
                    //
                    let proc_status_line: Vec<String> = status_line
                        .split(' ')
                        .map(|s| String::from(s.trim_end()))
                        .collect();

                    println!("Status Line: {:?}", proc_status_line);
                    
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

                    let check_redirect = proc_header.get(&String::from("Refresh")).cloned().unwrap_or(vec![]);
                    if !check_redirect.is_empty() {
                        urn = check_redirect
                            .get(1).cloned().unwrap()
                            .get(1).cloned().unwrap()
                            .clone();
                        urn = urn.splitn(4, '/').skip(3).next().unwrap().to_string();
                        continue;
                    }

                    println!("Header: {:?}", proc_header);

                    let proc_body = body;

                    println!("Body: {}", proc_body);

                    break;
                }

                println!("\nNOTICE: Finished opening web page");
            } else if command == "quit" {
                break;
            } else {
                println!("ERROR: command '{command}' not recognized");
            }
        }
    }

    Ok(())
}
