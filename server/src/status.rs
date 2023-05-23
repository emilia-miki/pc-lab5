use std::sync::atomic::{AtomicU8, Ordering};

pub enum Status {
    NoData,
    Reserved,
    Running,
    Completed { matrix_bytes: Vec<u8> },
}

impl std::convert::From<&Status> for String {
    fn from(value: &Status) -> Self {
        match value {
            Status::NoData => String::from("noData"),
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

pub enum StatusStripped {
    Reserved,
    Running,
    Completed,
}

impl std::convert::From<StatusStripped> for u8 {
    fn from(value: StatusStripped) -> u8 {
        match value {
            StatusStripped::Reserved => 1,
            StatusStripped::Running => 2,
            StatusStripped::Completed => 3,
        }
    }
}

impl std::convert::TryFrom<u8> for StatusStripped {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(StatusStripped::Reserved),
            2 => Ok(StatusStripped::Running),
            3 => Ok(StatusStripped::Completed),
            _ => Err(String::from("Invalid status code")),
        }
    }
}

pub struct AtomicStatus {
    status: AtomicU8,
}

impl AtomicStatus {
    pub fn new(status: StatusStripped) -> AtomicStatus {
        AtomicStatus {
            status: AtomicU8::new(u8::from(status)),
        }
    }

    pub fn load(&self) -> StatusStripped {
        StatusStripped::try_from(self.status.load(Ordering::Relaxed)).unwrap()
    }

    pub fn store(&self, status: StatusStripped) {
        self.status.store(u8::from(status), Ordering::Relaxed)
    }
}
