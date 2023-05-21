use core::slice;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU8, Arc},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Mutex,
    },
};

use crate::matrix_type::MatrixType;

pub struct Matrix {
    pub m_type: MatrixType,
    pub dimension: u32,
    pub bytes: Option<Vec<u8>>,
}

pub enum Status {
    NoData,
    Ready,
    Running,
    Completed { matrix: Matrix },
}

impl Status {
    fn strip(&self) -> StatusStripped {
        match self {
            Status::NoData => StatusStripped::NoData,
            Status::Ready => StatusStripped::Ready,
            Status::Running => StatusStripped::Running,
            Status::Completed { .. } => StatusStripped::Completed,
        }
    }

    pub fn encode(&self) -> u8 {
        self.strip().encode()
    }
}

impl std::convert::From<&Status> for String {
    fn from(value: &Status) -> Self {
        match value {
            Status::NoData => String::from("noData"),
            Status::Ready => String::from("ready"),
            Status::Running => String::from("running"),
            Status::Completed { .. } => String::from("completed"),
        }
    }
}

pub enum StatusStripped {
    NoData,
    Ready,
    Running,
    Completed,
}

impl StatusStripped {
    pub fn encode(self) -> u8 {
        match self {
            StatusStripped::NoData => 0,
            StatusStripped::Ready => 1,
            StatusStripped::Running => 2,
            StatusStripped::Completed => 3,
        }
    }

    pub fn decode(code: u8) -> Option<StatusStripped> {
        match code {
            0 => Some(StatusStripped::NoData),
            1 => Some(StatusStripped::Ready),
            2 => Some(StatusStripped::Running),
            3 => Some(StatusStripped::Completed),
            _ => None,
        }
    }
}

struct AtomicStatus {
    status: AtomicU8,
}

impl AtomicStatus {
    pub fn new(status: StatusStripped) -> AtomicStatus {
        AtomicStatus {
            status: AtomicU8::new(status.encode()),
        }
    }

    pub fn load(&self) -> StatusStripped {
        StatusStripped::decode(self.status.load(Ordering::Relaxed)).unwrap()
    }

    pub fn store(&self, status: StatusStripped) {
        self.status.store(status.encode(), Ordering::Relaxed)
    }
}

struct Job {
    status: AtomicStatus,
    thread_count: AtomicU8,
    matrix: Mutex<Matrix>,
}

pub struct JobManager {
    job_iterator: u8,
    jobs: HashMap<u8, Arc<Job>>,
}

const DEFAULT_THREAD_COUNT: u8 = 4;

impl JobManager {
    pub fn add_job(&mut self, matrix: Matrix) -> u8 {
        let job = Arc::new(Job {
            status: AtomicStatus::new(StatusStripped::Ready),
            thread_count: AtomicU8::new(DEFAULT_THREAD_COUNT),
            matrix: Mutex::new(matrix),
        });

        let index = self.job_iterator;
        self.jobs.insert(index, job);
        self.job_iterator += 1;

        index
    }

    pub fn start_job(&mut self, index: u8, mut thread_count: u8) -> Result<(), String> {
        let job = self
            .jobs
            .get(&index)
            .ok_or("There is no data for this index")?;

        if thread_count > 0 {
            job.thread_count.store(thread_count, Ordering::Relaxed);
        } else {
            thread_count = job.thread_count.load(Ordering::Relaxed);
        }

        job.status.store(StatusStripped::Running);

        let job_arc = Arc::clone(job);
        std::thread::spawn(move || {
            let m_type: MatrixType;
            let dimension: usize;
            let mut matrix_vec: Vec<u8>;
            {
                let mut matrix = job_arc.matrix.lock().unwrap();
                m_type = matrix.m_type;
                dimension = matrix.dimension as usize;
                matrix_vec = matrix.bytes.take().unwrap()
            };
            let type_size = m_type.get_type_size() as usize;
            let matrix_ptr = Arc::new(AtomicPtr::new(matrix_vec.as_mut_ptr()));

            let job_arc = &job_arc;
            std::thread::scope(move |s| {
                let thread_count = thread_count as usize;
                let mut threads: Vec<std::thread::ScopedJoinHandle<()>> =
                    Vec::with_capacity(thread_count);

                for i in 0..thread_count {
                    let matrix_ptr = Arc::clone(&matrix_ptr);
                    threads.push(s.spawn(move || {
                        let matrix_ptr = matrix_ptr.load(Ordering::Relaxed);
                        let index = i;

                        let mut taken = 0;
                        for i in 0..dimension {
                            for j in 0..i {
                                taken += 1;
                                if taken % thread_count != index {
                                    continue;
                                }

                                let lower_index = (i * dimension + j) * type_size;
                                let upper_index = (j * dimension + i) * type_size;

                                let lower = unsafe {
                                    slice::from_raw_parts_mut(
                                        matrix_ptr.add(lower_index),
                                        type_size,
                                    )
                                };
                                let upper = unsafe {
                                    slice::from_raw_parts_mut(
                                        matrix_ptr.add(upper_index),
                                        type_size,
                                    )
                                };

                                lower.swap_with_slice(upper);
                            }
                        }
                    }));
                }

                for thread in threads {
                    match thread.join() {
                        Ok(()) => (),
                        Err(_) => eprintln!("A thread returned an error"),
                    }
                }
            });

            {
                let _ = job_arc.matrix.lock().unwrap().bytes.insert(matrix_vec);
            }

            job_arc.status.store(StatusStripped::Completed);
        });

        Ok(())
    }

    pub fn get_status(&mut self, index: u8) -> Status {
        match self.jobs.get(&index) {
            Some(job) => match job.status.load() {
                StatusStripped::NoData => Status::NoData,
                StatusStripped::Ready => Status::Ready,
                StatusStripped::Running => Status::Running,
                StatusStripped::Completed => match self.get_matrix(index) {
                    Some(matrix) => Status::Completed { matrix },
                    None => Status::Running,
                },
            },
            None => Status::NoData,
        }
    }

    fn get_matrix(&mut self, index: u8) -> Option<Matrix> {
        let is_none = {
            self.jobs
                .get(&index)
                .unwrap()
                .matrix
                .lock()
                .unwrap()
                .bytes
                .is_none()
        };

        if is_none {
            return None;
        }

        let job = self.jobs.remove(&index).unwrap();
        let job = match Arc::try_unwrap(job) {
            Ok(job) => job,
            Err(_) => {
                panic!("Arc value is more than one, even though the job is marked as completed")
            }
        };

        Some(job.matrix.into_inner().unwrap())
    }
}

pub fn new_manager() -> JobManager {
    JobManager {
        job_iterator: 1,
        jobs: HashMap::<u8, Arc<Job>>::new(),
    }
}
