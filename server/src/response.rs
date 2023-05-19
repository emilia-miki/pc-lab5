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
    pub fn dump(self, stream: &mut TcpStream, buffer: &mut [u8]) -> Result<(), String> {
        let mut vec_buffer: Vec<u8>;
        let buffer = match self {
            Response::Error { error } => {
                println!("Sending an ErrorResponse with error {error}");

                let error = error.as_bytes();

                buffer[0] = 1;
                buffer[1..1 + error.len()].copy_from_slice(error);
                &buffer[..1 + error.len()]
            }
            Response::StartCalculation => {
                println!("Sending a StartCalculationResponse");

                buffer[..1].copy_from_slice(&[0u8]);
                &buffer[..1]
            }
            Response::SendData { index } => {
                println!("Sending a SendDataResponse with index {}", index);

                buffer[..2].copy_from_slice(&[0u8, index]);
                &buffer[..2]
            }
            Response::GetStatus {
                status,
                matrix_buffer,
            } => match (status, matrix_buffer) {
                (status, None) => match status {
                    Status::NoData => {
                        println!("Sending a GetStatusResponse with status NoData");

                        buffer[..2].copy_from_slice(&[0u8, 0u8]);
                        &buffer[..2]
                    }
                    Status::Ready => {
                        println!("Sending a GetStatusResponse with status Ready");

                        buffer[..2].copy_from_slice(&[0u8, 1u8]);
                        &buffer[..2]
                    }
                    Status::Running => {
                        println!("Sending a GetStatusResponse with status Running");

                        buffer[..2].copy_from_slice(&[0u8, 2u8]);
                        &buffer[..2]
                    }
                    Status::Completed => {
                        let error = "The job is completed, but no buffer was provided";

                        Err(error)?
                    }
                },
                (status, Some(matrix_buffer)) => match status {
                    Status::Completed => {
                        println!(
                            "Sending a GetStatusResponse with status Completed and a matrix buffer of length {}",
                            matrix_buffer.len()
                        );

                        vec_buffer = vec![0u8; 2 + matrix_buffer.len()];
                        let buffer = vec_buffer.as_mut_slice();
                        buffer[..2].copy_from_slice(&[0, 3]);
                        buffer[2..].copy_from_slice(matrix_buffer.as_slice());
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
            println!("{} bytes written to TcpStream", written_count);
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
