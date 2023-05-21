use std::io::Write;
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::job::Status;

pub enum Response {
    SendData { index: u8 },
    StartCalculation,
    GetStatus { status: Status },
    Error { error: String },
}

impl Response {
    fn encode(&self) -> u8 {
        match self {
            Response::SendData { .. } => 0,
            Response::StartCalculation => 1,
            Response::GetStatus { .. } => 2,
            Response::Error { .. } => 3,
        }
    }

    pub fn send(self, stream: &mut TcpStream) -> Result<(), String> {
        let response_code = self.encode();

        let buffer = match self {
            Response::SendData { index } => vec![response_code, index],
            Response::StartCalculation => vec![response_code],
            Response::GetStatus { status } => {
                let status_code = status.encode();

                match status {
                    Status::Completed { matrix } => {
                        let matrix_buffer = matrix.bytes.unwrap();
                        let mut buffer = vec![0u8; 7 + matrix_buffer.len()];
                        buffer[..3].copy_from_slice(
                            &[
                                response_code,
                                status_code,
                                matrix.m_type.try_into().unwrap(),
                            ][..],
                        );
                        buffer[3..7].copy_from_slice(&matrix.dimension.to_le_bytes());
                        buffer[7..].copy_from_slice(&matrix_buffer);
                        buffer
                    }
                    _ => vec![response_code, status_code],
                }
            }
            Response::Error { error } => {
                println!("Sending an ErrorResponse with error {error}");

                let error = error.as_bytes();

                let mut buffer = vec![0u8; 2 + error.len()];
                buffer[..2].copy_from_slice(&[response_code, error.len() as u8][..]);
                buffer[2..].copy_from_slice(error);
                buffer
            }
        };

        let _ = match stream.write(&buffer) {
            Ok(size) => size,
            Err(error) => Err(format!("{}", error))?,
        };

        Ok(())
    }

    pub fn to_json_string(&self, client_id: u16) -> String {
        format!(
            r#"{{"client":{},"time":{},"kind":"{}","type":"{}","payload":{}}}"#,
            client_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            "response",
            match self {
                Response::SendData { .. } => "sendData",
                Response::StartCalculation => "startCalculation",
                Response::GetStatus { .. } => "getStatus",
                Response::Error { .. } => "error",
            },
            match self {
                Response::SendData { index } => format!(r#"{{"index":{}}}"#, index),
                Response::StartCalculation => format!(r#"{{}}"#),
                Response::GetStatus { status } => {
                    match status {
                        Status::Completed { matrix } => {
                            format!(
                                r#"{{"status":"{}","type":"{}","dimension":{}}}"#,
                                String::from(status),
                                String::from(matrix.m_type),
                                matrix.dimension
                            )
                        }
                        status => format!(r#"{{"status":"{}"}}"#, String::from(status)),
                    }
                }
                Response::Error { error } => format!(r#"{{"message":"{}"}}"#, error),
            }
        )
    }
}
