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
                Response::GetStatus {
                    status,
                    matrix_buffer,
                }
            }
        }
    }
}

pub fn handle_client(mut stream: TcpStream) {
    const BUFFER_SIZE: usize = 1500;

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut job_manager = job::new_manager();

    loop {
        let request = match Request::from_stream(&mut stream, &mut buffer) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("{}", error);
                continue;
            }
        };

        let response = request.execute(&mut job_manager);
        match response.to_stream(&mut stream, &mut buffer) {
            Ok(()) => (),
            Err(error) => {
                eprintln!("{}", error);
                continue;
            }
        }
    }
}
