mod job;
mod matrix_type;
mod request;
mod response;
mod status;
mod thread;

use std::{error::Error, net::TcpListener};

fn main() -> Result<(), Box<dyn Error>> {
    const ADDR: &str = "127.0.0.1";

    let args: Vec<String> = std::env::args().collect();
    let port = match args.len() {
        1 => "0",
        2 => &args[1],
        _ => Err("there can be only one argument: the port number")?,
    };

    let listener = TcpListener::bind(format!("{ADDR}:{port}"))?;
    let port = listener.local_addr().unwrap().port();
    println!(r#"{{"kind":"listen","port":"{port}"}}"#);

    for stream in listener.incoming() {
        let stream = stream?;
        let port = stream.peer_addr().unwrap().port();
        println!(r#"{{"kind":"accept","port":"{port}"}}"#);

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
