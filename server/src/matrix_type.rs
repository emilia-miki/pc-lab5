use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub enum MatrixType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

impl MatrixType {
    pub fn get_type_size(&self) -> u8 {
        match self {
            MatrixType::U8 => 1,
            MatrixType::U16 => 2,
            MatrixType::U32 => 4,
            MatrixType::U64 => 8,
            MatrixType::I8 => 1,
            MatrixType::I16 => 2,
            MatrixType::I32 => 4,
            MatrixType::I64 => 8,
            MatrixType::F32 => 4,
            MatrixType::F64 => 8,
        }
    }
}

impl std::convert::TryFrom<u8> for MatrixType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MatrixType::U8),
            1 => Ok(MatrixType::U16),
            2 => Ok(MatrixType::U32),
            3 => Ok(MatrixType::U64),
            4 => Ok(MatrixType::U8),
            5 => Ok(MatrixType::U16),
            6 => Ok(MatrixType::U32),
            7 => Ok(MatrixType::U64),
            8 => Ok(MatrixType::F32),
            9 => Ok(MatrixType::F64),
            _ => Err(format!("Invalid matrix type code: {}", value)),
        }
    }
}

impl std::convert::From<MatrixType> for u8 {
    fn from(value: MatrixType) -> Self {
        match value {
            MatrixType::U8 => 0,
            MatrixType::U16 => 1,
            MatrixType::U32 => 2,
            MatrixType::U64 => 3,
            MatrixType::I8 => 4,
            MatrixType::I16 => 5,
            MatrixType::I32 => 6,
            MatrixType::I64 => 7,
            MatrixType::F32 => 8,
            MatrixType::F64 => 9,
        }
    }
}

impl std::convert::From<MatrixType> for String {
    fn from(m_type: MatrixType) -> Self {
        match m_type {
            MatrixType::U8 => String::from("u8"),
            MatrixType::U16 => String::from("u16"),
            MatrixType::U32 => String::from("u32"),
            MatrixType::U64 => String::from("u64"),
            MatrixType::I8 => String::from("i8"),
            MatrixType::I16 => String::from("i16"),
            MatrixType::I32 => String::from("i32"),
            MatrixType::I64 => String::from("i64"),
            MatrixType::F32 => String::from("f32"),
            MatrixType::F64 => String::from("f64"),
        }
    }
}
