use std::{eprintln, net::TcpStream};

use crate::{job, request::Request, response::Response};

trait Executable<'a> {
    fn execute(self, job_manager: &mut job::JobManager) -> Response;
}

impl<'a> Executable<'a> for Request {
    fn execute(self, job_manager: &mut job::JobManager) -> Response {
        match self {
            Request::SendData { matrix } => Response::SendData {
                index: job_manager.add_job(matrix),
            },
            Request::StartCalculation {
                index,
                thread_count,
            } => match job_manager.start_job(index, thread_count) {
                Ok(()) => Response::StartCalculation,
                Err(error) => Response::Error { error },
            },
            Request::GetStatus { index } => {
                let status = job_manager.get_status(index);
                let matrix_buffer = match status {
                    job::Status::Completed => job_manager.get_result(index),
                    _ => None,
                };

                if matrix_buffer.is_some() {
                    Response::GetStatus {
                        status,
                        matrix_buffer,
                    }
                } else {
                    Response::GetStatus {
                        status: job::Status::Running,
                        matrix_buffer: None,
                    }
                }
            }
        }
    }
}

pub fn handle_client(mut stream: TcpStream) {
    const BUFFER_SIZE: usize = 1500;

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut job_manager = job::new_manager();

    println!(
        "Serving for {} on a new thread",
        std::thread::current().name().unwrap()
    );

    loop {
        let request = match Request::from_stream(&mut stream, &mut buffer) {
            Ok(request) => request,
            Err(error) => {
                if error == "The client disconnected" {
                    break;
                }

                eprintln!("{}", error);
                continue;
            }
        };

        let response = request.execute(&mut job_manager);
        match response.dump(&mut stream, &mut buffer) {
            Ok(()) => (),
            Err(error) => {
                eprintln!("{}", error);
                continue;
            }
        }
    }
}
