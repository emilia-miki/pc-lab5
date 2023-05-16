use std::io::Write;
use std::net::TcpStream;

use crate::job::Status;

pub enum Response {
    Error {
        error: String,
    },
    SendData {
        index: u8,
    },
    StartCalculation,
    GetStatus {
        status: Status,
        matrix_buffer: Option<Vec<u8>>,
    },
}

impl Response {
    pub fn to_stream(self, stream: &mut TcpStream, buffer: &mut [u8]) -> Result<(), String> {
        let mut vec_buffer: Vec<u8>;
        let buffer = match self {
            Response::Error { error } => {
                let error = error.as_bytes();

                buffer[0] = 1;
                buffer[1..].copy_from_slice(error);
                &buffer[..1 + error.len()]
            }
            Response::StartCalculation => {
                buffer.copy_from_slice(&[0u8, 0]);
                &buffer[..1]
            }
            Response::SendData { index } => {
                buffer.copy_from_slice(&[0, index]);
                &buffer[..2]
            }
            Response::GetStatus {
                status,
                matrix_buffer,
            } => match (status, matrix_buffer) {
                (status, None) => match status {
                    Status::NoData => {
                        buffer.copy_from_slice(&[0, 0]);
                        &buffer[..2]
                    }
                    Status::Ready => {
                        buffer.copy_from_slice(&[0, 1]);
                        &buffer[..2]
                    }
                    Status::Running => {
                        buffer.copy_from_slice(&[0, 2]);
                        &buffer[..2]
                    }
                    Status::Completed => Err("The job is completed, but no buffer was provided")?,
                },
                (status, Some(matrix_buffer)) => match status {
                    Status::Completed => {
                        vec_buffer = vec![0u8; 2 + matrix_buffer.len()];
                        let buffer = vec_buffer.as_mut_slice();
                        buffer.copy_from_slice(&[0, 3]);
                        buffer.copy_from_slice(matrix_buffer.as_slice());
                        buffer
                    }
                    _ => Err("There is a buffer provided, but the job is not completed yet")?,
                },
            },
        };

        let written_count = match stream.write(buffer) {
            Ok(size) => size,
            Err(error) => Err(format!("{}", error))?,
        };

        if written_count == buffer.len() {
            Ok(())
        } else {
            Err(format!(
                "Only {} bytes written to TcpStream out of {}",
                written_count,
                buffer.len()
            ))
        }
    }
}
