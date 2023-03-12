use std::{
    io::{prelude::*, BufReader, Result as IoResult},
    net::TcpStream,
    collections::HashMap
};

fn read_n<R>(reader: R, bytes_to_read: u64) -> Vec<u8>
where
    R: Read,
{
    let mut buf = vec![];
    reader.take(bytes_to_read).read_to_end(&mut buf).unwrap();
    buf
}

const PACKET_MAX_BYTES: usize = 256;

fn main() -> IoResult<()> {
    let (uri, port) = ("monta.if.its.ac.id", "80");
    let mut stream = TcpStream::connect(format!("{uri}:{port}"))?;

    let mut urn = String::from("");
    // Request Handling
    'request: loop {
        let request = format!("GET /{urn} HTTP/1.1\r\nHost: {uri}\r\n\r\n");
        stream.write(&request.into_bytes())?;
        stream.flush()?;

        let mut stream_buf_reader = BufReader::new(&mut stream);
        let (mut status_line, mut header, mut body): (String, String, String) = (String::new(), String::new(), String::new());

        // Parser
        loop {
            let chunked_http_response = read_n(&mut stream_buf_reader, PACKET_MAX_BYTES as u64);
            let mut byte_counter = 0;
            let chunked_http_response = String::from_utf8(chunked_http_response).unwrap();

            // Status
            if !status_line.ends_with("\r\n") {
                status_line.push_str(&chunked_http_response[0..chunked_http_response.find('\n').unwrap() + 1]);
                if status_line.len() == PACKET_MAX_BYTES { continue; }
                byte_counter = status_line.len();
                println!("Status Line: {}", status_line);
            }

            // Header
            if !header.ends_with("\r\n\r\n") {
                header.push_str(&chunked_http_response[byte_counter..chunked_http_response.find('<').unwrap_or(chunked_http_response.len())]);
                if !header.ends_with("\r\n\r\n") { continue; }
                byte_counter = chunked_http_response.find('<').unwrap_or(0);
                println!("Header:");
                print!("{}", header);
            }

            if chunked_http_response.len() < PACKET_MAX_BYTES { break; }

            // Body (might only deal with HTML for now)
            body.push_str(&chunked_http_response[byte_counter..chunked_http_response.len()]);
            if chunked_http_response.len() == 256 { continue; }
            println!("Body:");
            print!("{}", body);

            if chunked_http_response.len() < PACKET_MAX_BYTES { break; }
        }
        
        // Response Processing
        //
        let proc_status_line: Vec<String> = status_line
            .split(' ')
            .map(|s| String::from(s.trim_end()))
            .collect();

        println!("{:?}", proc_status_line);
        
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
            println!("{:?}", urn);
            continue;
        }

        println!("{:?}", proc_header);

        let proc_body: Vec<String> = body
            .lines()
            .map(|s| String::from(s))
            .collect();

        println!("{:?}", proc_body);

        break 'request;
    }

    Ok(())
}
