use std::{
    eprintln,
    net::TcpStream,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{job, request::Request};

pub fn handle_client(mut stream: TcpStream) {
    let mut job_manager = job::new_manager();
    let id: u16 = str::parse(std::thread::current().name().unwrap()).unwrap();

    loop {
        let request = match Request::from_stream(&mut stream) {
            Ok(request) => request,
            Err(error) => {
                if error.to_string() == "The client disconnected" {
                    break;
                }

                eprintln!("{}", error);
                continue;
            }
        };
        println!("{}", request.to_json_string(id));

        let response = request.execute(&mut job_manager);
        let response_json = response.to_json_string(id);
        match response.send(&mut stream) {
            Ok(()) => (),
            Err(error) => {
                eprintln!("{}", error);
                continue;
            }
        }
        let time_stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pattern = r#""time":"#;
        let start_index = response_json.find(pattern).unwrap() + pattern.len();
        let end_index = start_index + response_json[start_index..].find(',').unwrap();
        let response_json = format!(
            "{}{}{}",
            &response_json[..start_index],
            time_stamp,
            &response_json[end_index..]
        );
        println!("{}", response_json);
    }
}
