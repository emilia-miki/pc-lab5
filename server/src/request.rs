use std::io::Read;
use std::net::TcpStream;
use std::sync::Mutex;

use crate::job::Matrix;

pub enum Request {
    SendData { matrix: Matrix },
    StartCalculation { index: u8, thread_count: u8 },
    GetStatus { index: u8 },
}

impl Request {
    pub fn from_stream(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<Request, String> {
        let read_count = match stream.read(buffer) {
            Ok(size) => size,
            Err(error) => Err(format!("{}", error))?,
        };

        let buffer = &buffer[0..read_count];

        match buffer[0] {
            0 => {
                let type_size = buffer[1];
                let dimension = {
                    let slice = &buffer[2..=5];
                    let mut array = [0u8; 4];
                    array.copy_from_slice(slice);
                    u32::from_ne_bytes(array)
                };

                let expected_len = {
                    let capacity = u32::from(type_size) * dimension * dimension;
                    usize::try_from(capacity).unwrap()
                };

                let mut matrix_buffer = vec![0u8; expected_len];

                let matrix_part = &buffer[6..];
                let vec_part = &mut matrix_buffer[0..matrix_part.len()];
                vec_part.copy_from_slice(matrix_part);
                let mut written_count = matrix_part.len();

                while written_count < expected_len {
                    let read_count = match stream.read(&mut matrix_buffer[written_count..]) {
                        Ok(size) => size,
                        Err(error) => Err(format!("{}", error))?,
                    };

                    written_count += read_count;
                }

                Ok(Request::SendData {
                    matrix: Matrix {
                        type_size,
                        dimension,
                        bytes: Mutex::new(Some(matrix_buffer)),
                    },
                })
            }
            1 => Ok(Request::StartCalculation {
                index: buffer[1],
                thread_count: buffer[2],
            }),
            2 => Ok(Request::GetStatus { index: buffer[1] }),
            code => Err(format!("Unknown request code: {code}")),
        }
    }
}
