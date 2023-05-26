mod job;
mod matrix_type;
mod request;
mod response;
mod status;
mod thread;

use sysinfo::{CpuRefreshKind, RefreshKind, System, SystemExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), String> {
    const ADDR: &str = "127.0.0.1";

    let args: Vec<String> = std::env::args().collect();
    let port = match args.len() {
        1 => "0",
        2 => &args[1],
        _ => Err("there can be only one argument: the port number")?,
    };

    let listener = TcpListener::bind(format!("{ADDR}:{port}"))
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().unwrap().port();
    println!(r#"{{"kind":"listen","port":"{port}"}}"#);

    let cpu_count = System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::new()))
        .cpus()
        .len();

    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(cpu_count)
        .build()
        .unwrap();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::task::spawn(job::process_tasks(tp, rx));
    loop {
        let (stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
        tokio::spawn(thread::handle_client(stream, tx.clone()));
    }
}
