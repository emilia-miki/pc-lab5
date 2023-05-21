use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{error::Error, io::Read};

use crate::{
    job::{self, Matrix},
    matrix_type::MatrixType,
    response::Response,
};

pub enum Request {
    SendData { matrix: Matrix },
    StartCalculation { index: u8, thread_count: u8 },
    GetStatus { index: u8 },
}

impl Request {
    pub fn execute(self, job_manager: &mut job::JobManager) -> Response {
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
            Request::GetStatus { index } => Response::GetStatus {
                status: job_manager.get_status(index),
            },
        }
    }

    fn read_chunk(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<(), Box<dyn Error>> {
        let read_count = stream.read(buffer)?;

        if read_count == 0 {
            Err("The client disconnected")?;
        }

        if read_count < buffer.len() {
            Err(format!(
                "Expected to read {} bytes, but got only {}",
                buffer.len(),
                read_count
            ))?;
        }

        Ok(())
    }

    pub fn from_stream(stream: &mut TcpStream) -> Result<Request, Box<dyn Error>> {
        let mut buffer = [0u8; 6];
        Request::read_chunk(stream, &mut buffer[..1])?;

        match buffer[0] {
            0 => {
                Request::read_chunk(stream, &mut buffer[1..6])?;

                let m_type = MatrixType::try_from(buffer[1])?;
                let type_size = m_type.get_type_size();
                let dimension = {
                    let slice = &buffer[2..6];
                    let mut array = [0u8; 4];
                    array.copy_from_slice(slice);
                    u32::from_le_bytes(array)
                };

                let expected_len = (u32::from(type_size) * dimension * dimension) as usize;
                let mut matrix_buffer = vec![0u8; expected_len];

                Request::read_chunk(stream, &mut matrix_buffer)?;
                matrix_buffer = dbg!(matrix_buffer);

                Ok(Request::SendData {
                    matrix: Matrix {
                        m_type,
                        dimension,
                        bytes: Some(matrix_buffer),
                    },
                })
            }
            1 => {
                Request::read_chunk(stream, &mut buffer[1..=2])?;

                Ok(Request::StartCalculation {
                    index: buffer[1],
                    thread_count: buffer[2],
                })
            }
            2 => {
                Request::read_chunk(stream, &mut buffer[1..=1])?;

                Ok(Request::GetStatus { index: buffer[1] })
            }
            code => Err(format!("Unknown request code: {code}"))?,
        }
    }

    pub fn to_json_string(&self, client_id: u16) -> String {
        format!(
            r#"{{"client":{},"time":{},"kind":"{}","type":"{}","payload":{}}}"#,
            client_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            "request",
            match self {
                Request::SendData { .. } => "sendData",
                Request::StartCalculation { .. } => "startCalculation",
                Request::GetStatus { .. } => "getStatus",
            },
            match self {
                Request::SendData { matrix } => format!(
                    r#"{{"type":"{}","dimension":{}}}"#,
                    String::from(matrix.m_type),
                    matrix.dimension
                ),
                Request::StartCalculation {
                    index,
                    thread_count,
                } => format!(r#"{{"index":{},"threadCount":{}}}"#, index, thread_count),
                Request::GetStatus { index } => format!(r#"{{"index":{}}}"#, index),
            }
        )
    }
}
