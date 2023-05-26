use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Status {
    NoData,
    Reserved,
    Running,
    Completed { matrix_bytes: Vec<u8> },
}

impl std::convert::From<&Status> for String {
    fn from(value: &Status) -> Self {
        match value {
            Status::NoData => String::from("no data"),
            Status::Reserved => String::from("reserved"),
            Status::Running => String::from("running"),
            Status::Completed { .. } => String::from("completed"),
        }
    }
}

impl std::convert::From<&Status> for u8 {
    fn from(value: &Status) -> Self {
        match value {
            Status::NoData => 0,
            Status::Reserved => 1,
            Status::Running => 2,
            Status::Completed { .. } => 3,
        }
    }
}
