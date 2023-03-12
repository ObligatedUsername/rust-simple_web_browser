extern crate pancurses;

use std::{
    io::{prelude::*, BufReader, Result as IoResult},
    net::TcpStream,
    collections::HashMap
};
use pancurses::{
    initscr,
    endwin,
    noecho,
    Input
};

fn read_n<R>(reader: R, bytes_to_read: u64) -> Vec<u8>
where
    R: Read,
{
    let mut buf = vec![];
    reader.take(bytes_to_read).read_to_end(&mut buf).unwrap();
    buf
}

const PACKET_MAX_BYTES: usize = 512;

fn main() -> IoResult<()> {
    let (mut uri, mut port, mut urn) = (String::from(""), String::from("3000"), String::from(""));

    // User Interface -- in Terminal using pancurses
    let window = initscr();

    window.printw("Welcome to \"Simple\" Web Browser, press 'DEL' to quit\n");
    window.printw("Available Commands:\n");
    window.printw("\topen [URI:PORT/URN]\n");
    window.printw("> ");
    window.refresh();
    window.keypad(true);
    noecho();

    let mut command_line = String::new();
    let (mut command, mut args): (String, Vec<Vec<String>>) = (String::new(), vec![]);
    loop {
        // Command Line Input & Processing
        match window.getch() {
            Some(Input::Character(c)) => {
                window.addch(c);
                if c == '\n' {
                    (command, args) = command_line
                        .trim()
                        .split_once(' ')
                        .map(|t| (String::from(t.0), t.1
                                  .splitn(2, '/')
                                  .map(|s1| String::from(s1)
                                       .split(':')
                                       .map(|s2| String::from(s2))
                                       .collect())
                                  .collect()))
                        .unwrap_or((String::new(), vec![]));
                    println!("{command} {:?}", args);
                    if command.is_empty() {
                        command_line = String::new();
                        window.printw(&format!("Error: Please enter a valid command and its corresponding arguments.\n"));
                        window.printw("> ");
                    }
                } else {
                    command_line.push(c);
                    continue;
                }
            },
            Some(Input::KeyBackspace) => {
                if window.get_cur_x() > 2 {
                    window.mv(window.get_cur_y(), window.get_cur_x()-1);
                    window.delch();
                    command_line.pop();
                    continue;
                }
            },
            Some(Input::KeyDC) => { break; },
            Some(input) => { window.addstr(&format!("{:?}", input)); },
            None => ()
        }

        window.refresh();

        if !command.is_empty() {
            println!("{command} {args:?}");
            command_line = String::new();
            if command == "open" {
                uri = args.get(0).cloned().unwrap_or(vec![String::from("localhost")]).get(0).cloned().unwrap();
                port = args.get(0).cloned().unwrap_or(vec![String::new(), String::from("3000")]).get(1).cloned().unwrap_or(String::from("3000"));
                urn = args.get(1).cloned().unwrap_or(vec![String::from("")]).get(0).cloned().unwrap_or(String::from(""));
            } else { continue; }
            (command, args) = (String::new(), vec![]);
            // Request Handling
            loop {
                let mut stream = TcpStream::connect(format!("{uri}:{port}"))?;
                let request = format!("GET /{urn} HTTP/1.1\r\nHost: {uri}\r\n\r\n");
                stream.write(&request.into_bytes())?;
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

                window.printw(&format!("Status Line: {:?}\n", proc_status_line));
                window.refresh();
                
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

                window.printw(&format!("Header: {:?}\n", proc_header));
                window.refresh();

                let proc_body = body;

                window.printw(&format!("Body: {}\n", proc_body));
                window.refresh();

                break;
            }

            window.printw("> ");
            window.refresh();
        }
    }

    endwin();

    Ok(())
}
