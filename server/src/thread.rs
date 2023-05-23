use std::{
    eprintln, io,
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
                if error.is::<io::Error>() {
                    break;
                }

                eprintln!("Error parsing request: {}", error);
                continue;
            }
        };
        println!("{}", request.to_json_string());

        let response = request.execute(&mut job_manager, &mut stream);
        let response_json = response.to_json_string(id);
        match response.send(&mut stream) {
            Ok(()) => (),
            Err(error) => {
                eprintln!("Error sending response: {}", error);
                continue;
            }
        }
        let time_on_send = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let response_json = json_replace(&response_json, "time", &format!("{time_on_send}"));
        println!("{}", response_json);
    }
}

fn json_replace(json: &str, key: &str, new_value: &str) -> String {
    let pattern = format!(r#""{key}":""#);
    let begin = json.find(&pattern).unwrap() + pattern.len();
    let end = begin + json[begin..].find('"').unwrap();
    format!("{}{}{}", &json[..begin], new_value, &json[end..])
}
