mod job;
mod matrix_type;
mod request;
mod response;
mod thread;

use std::{error::Error, net::TcpListener};

fn main() -> Result<(), Box<dyn Error>> {
    const ADDR: &str = "127.0.0.1";
    let mut port = "3333";

    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 => (),
        2 => port = &args[1],
        _ => return Err("Invalid number of arguments".into()),
    }

    let listener = TcpListener::bind(format!("{ADDR}:{port}"))?;
    println!(r#"{{"kind":"listen","port":{port}}}"#);

    for stream in listener.incoming() {
        let stream = stream?;
        let addr = stream.peer_addr().unwrap();
        let port = addr.port();
        println!(r#"{{"kind":"accept","port":{port}}}"#);

        match std::thread::Builder::new()
            .name(format!("{port}"))
            .spawn(move || thread::handle_client(stream))
        {
            Ok(_) => (),
            Err(error) => eprintln!("{}", error),
        };
    }

    Ok(())
}
