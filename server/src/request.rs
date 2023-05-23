use std::error::Error;
use std::io::Read;
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::job::JobManager;
use crate::{matrix_type::MatrixType, response::Response};

pub enum Request {
    Reserve {
        matrix_type: MatrixType,
        matrix_dimension: u32,
    },
    Calc {
        index: u8,
        thread_count: u8,
    },
    Poll {
        index: u8,
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
    pub fn from_stream(stream: &mut TcpStream) -> Result<Request, Box<dyn Error>> {
        let mut buffer = [0u8; 5];

        stream.read_exact(&mut buffer[..1])?;
        let request_code = buffer[0];
        match request_code {
            0 => {
                stream.read_exact(&mut buffer[..5])?;

                let matrix_type = MatrixType::try_from(buffer[0])?;
                let matrix_dimension = {
                    let mut array = [0u8; 4];
                    array.copy_from_slice(&buffer[1..5]);
                    u32::from_le_bytes(array)
                };

                Ok(Request::Reserve {
                    matrix_type,
                    matrix_dimension,
                })
            }
            1 => {
                stream.read_exact(&mut buffer[..2])?;
                let index = buffer[0];
                let thread_count = buffer[1];

                Ok(Request::Calc {
                    index,
                    thread_count,
                })
            }
            2 => {
                stream.read_exact(&mut buffer[..1])?;
                let index = buffer[0];

                Ok(Request::Poll { index })
            }
            code => Err(format!("Unknown request code: {code}"))?,
        }
    }

    pub fn execute(self, job_manager: &mut JobManager, stream: &mut TcpStream) -> Response {
        match self {
            Request::Reserve {
                matrix_type,
                matrix_dimension,
            } => Response::Reserve {
                index: job_manager
                    .reserve(matrix_type, matrix_dimension)
                    .unwrap_or(0),
            },
            Request::Calc {
                index,
                thread_count,
            } => match job_manager.calc(index, thread_count, stream) {
                Ok(()) => Response::Calc,
                Err(error) => Response::Error {
                    error: error.to_string(),
                },
            },
            Request::Poll { index } => Response::Poll {
                status: job_manager.poll(index),
            },
        }
    }

    pub fn to_json_string(&self) -> String {
        format!(
            r#"{{"client":"{}","time":"{}","kind":"{}",{}}}"#,
            std::thread::current().name().unwrap(),
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
                    Request::Calc {
                        index,
                        thread_count,
                    } => {
                        json = format!(
                            r#"{},"index":"{}","threadCount":"{}""#,
                            json, index, thread_count,
                        )
                    }
                    Request::Poll { index } => json = format!(r#"{},"index":"{}""#, json, index),
                }
                json
            }
        )
    }
}
