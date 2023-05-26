use serde::Serialize;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use crate::job::JobManager;
use crate::{matrix_type::MatrixType, response::Response};

#[derive(Serialize)]
pub enum Request {
    Reserve {
        matrix_type: MatrixType,
        matrix_dimension: u32,
    },
    Calc {
        id: usize,
    },
    Poll {
        id: usize,
    },
}

impl std::convert::From<&Request> for String {
    fn from(value: &Request) -> Self {
        match value {
            Request::Reserve { .. } => String::from("reserve"),
            Request::Calc { .. } => String::from("calc"),
            Request::Poll { .. } => String::from("poll"),
        }
    }
}

impl Request {
    pub async fn from_stream(stream: &mut TcpStream) -> Result<Request, Box<dyn Error>> {
        let request_code = {
            let mut buffer = [0u8; 1];
            stream.read_exact(&mut buffer).await?;
            buffer[0]
        };

        match request_code {
            0 => {
                let matrix_type = {
                    let mut buffer = [0u8; 1];
                    stream.read_exact(&mut buffer).await?;
                    MatrixType::try_from(buffer[0])?
                };

                let matrix_dimension = {
                    let mut buffer = [0u8; 4];
                    stream.read_exact(&mut buffer).await?;
                    u32::from_le_bytes(buffer)
                };

                Ok(Request::Reserve {
                    matrix_type,
                    matrix_dimension,
                })
            }
            1 => {
                let id = {
                    let mut buffer = [0u8; 8];
                    stream.read_exact(&mut buffer).await?;
                    usize::from_le_bytes(buffer)
                };

                Ok(Request::Calc { id })
            }
            2 => {
                let id = {
                    let mut buffer = [0u8; 8];
                    stream.read_exact(&mut buffer).await?;
                    usize::from_le_bytes(buffer)
                };

                Ok(Request::Poll { id })
            }
            code => Err(format!("unknown request code: {code}"))?,
        }
    }

    pub async fn execute(self, job_manager: &mut JobManager, stream: &mut TcpStream) -> Response {
        match self {
            Request::Reserve {
                matrix_type,
                matrix_dimension,
            } => match job_manager.reserve(matrix_type, matrix_dimension).await {
                Ok(id) => Response::Reserve { id },
                Err(error) => Response::Error { error },
            },
            Request::Calc { id } => match job_manager.calc(id, stream).await {
                Ok(()) => Response::Calc,
                Err(error) => Response::Error { error },
            },
            Request::Poll { id } => Response::Poll {
                status: job_manager.poll(id).await,
            },
        }
    }

    pub fn to_json_string(&self, client_id: u16) -> String {
        format!(
            r#"{{"client":"{}","time":"{}","kind":"{}",{}}}"#,
            client_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            "request",
            {
                let mut json = format!(r#""type":"{}""#, String::from(self));
                match self {
                    Request::Reserve {
                        matrix_type,
                        matrix_dimension,
                    } => {
                        json = format!(
                            r#"{},"matrixType":"{}","matrixDimension":"{}""#,
                            json,
                            String::from(*matrix_type),
                            matrix_dimension
                        )
                    }
                    Request::Calc { id } => json = format!(r#"{},"id":"{}""#, json, id),
                    Request::Poll { id } => json = format!(r#"{},"id":"{}""#, json, id),
                }
                json
            }
        )
    }
}
