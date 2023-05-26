use std::{
    eprintln,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};

use crate::{
    job::{self, MatrixData, Task},
    request::Request,
};

pub async fn handle_client(
    mut stream: TcpStream,
    tx: UnboundedSender<(MatrixData, Arc<Mutex<Task>>)>,
) {
    let port = stream.peer_addr().unwrap().port();
    let mut job_manager = job::new_manager(tx);

    loop {
        let request = match Request::from_stream(&mut stream).await {
            Ok(request) => request,
            Err(error) => match error.downcast::<std::io::Error>() {
                Ok(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    eprintln!("The client {port} disconnected");
                    break;
                }
                err => {
                    eprintln!(
                        "Error parsing request: {}",
                        err.map_or_else(|e| e.to_string(), |e| e.to_string())
                    );
                    break;
                }
            },
        };
        println!("{}", request.to_json_string(port));

        let response = request.execute(&mut job_manager, &mut stream).await;
        let response_json = response.to_json_string(port);
        match response.send(&mut stream).await {
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
