use std::error::Error;
use std::format;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::status::Status;

#[derive(Debug)]
pub enum Response {
    Reserve { id: usize },
    Calc,
    Poll { status: Status },
    Error { error: String },
}

impl std::convert::From<&Response> for u8 {
    fn from(value: &Response) -> Self {
        match value {
            Response::Reserve { .. } => 0,
            Response::Calc => 1,
            Response::Poll { .. } => 2,
            Response::Error { .. } => 3,
        }
    }
}

impl std::convert::From<&Response> for String {
    fn from(value: &Response) -> Self {
        match value {
            Response::Reserve { .. } => String::from("reserve"),
            Response::Calc => String::from("calc"),
            Response::Poll { .. } => String::from("poll"),
            Response::Error { .. } => String::from("error"),
        }
    }
}

impl Response {
    pub async fn send(self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        let response_code = u8::from(&self);
        stream.write_all(&[response_code]).await?;

        match self {
            Response::Reserve { id } => stream.write_all(&id.to_le_bytes()).await?,
            Response::Calc => (),
            Response::Poll { status } => {
                let status_code = u8::from(&status);
                stream.write_all(&[status_code]).await?;

                if let Status::Completed { matrix_bytes } = status {
                    stream.write_all(&matrix_bytes).await?;
                }
            }
            Response::Error { error } => {
                let error = error.as_bytes();
                stream.write_all(&[error.len() as u8]).await?;
                stream.write_all(error).await?;
            }
        };

        Ok(())
    }

    pub fn to_json_string(&self, client_id: u16) -> String {
        format!(
            r#"{{"client":"{}","time":"{}","kind":"{}",{}}}"#,
            client_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            "response",
            {
                let mut json = format!(r#""type":"{}""#, String::from(self));
                match self {
                    Response::Reserve { id } => json = format!(r#"{},"id":"{}""#, json, id),
                    Response::Calc => (),
                    Response::Poll { status } => {
                        json = format!(r#"{},"status":"{}""#, json, String::from(status));
                    }
                    Response::Error { error } => {
                        json = format!(r#"{},"message":"{}""#, json, error)
                    }
                }
                json
            }
        )
    }
}
