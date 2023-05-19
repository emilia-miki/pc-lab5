mod job;
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

    println!("Listening on port {port}.");

    let mut id = 0;
    for stream in listener.incoming() {
        let stream = stream?;
        let addr = stream.peer_addr().unwrap();
        println!("Got a new connection: {}", &addr);
        match std::thread::Builder::new()
            .name(format!("Client {id} ({})", addr))
            .spawn(move || thread::handle_client(stream))
        {
            Ok(_) => {
                id += 1;
            }
            Err(error) => eprintln!("{}", error),
        };
    }

    Ok(())
}
