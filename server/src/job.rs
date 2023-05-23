use core::slice;
use std::{
    collections::HashMap,
    error::Error,
    io::Read,
    net::TcpStream,
    sync::{
        atomic::{AtomicPtr, AtomicU8, Ordering},
        Arc, Mutex,
    },
};

use once_cell::sync::Lazy;
use sysinfo::{RefreshKind, System, SystemExt};

use crate::status::{AtomicStatus, StatusStripped};
use crate::{matrix_type::MatrixType, status::Status};

struct Job {
    status: AtomicStatus,
    thread_count: AtomicU8,
    matrix_type: MatrixType,
    matrix_dimension: u32,
    matrix_vec: Mutex<Option<Vec<u8>>>,
}

pub struct JobManager {
    job_iterator: u8,
    jobs: HashMap<u8, Arc<Job>>,
}

const DEFAULT_THREAD_COUNT: u8 = 4;

fn reserve_if_available(len: u64) -> Option<Vec<u8>> {
    const AVAILABLE_MEMORY_THRESHOLD: u64 = 500_000_000;

    static SYSTEM: once_cell::sync::Lazy<Mutex<sysinfo::System>> =
        Lazy::new(|| Mutex::new(System::new_with_specifics(RefreshKind::new().with_memory())));

    let lock = SYSTEM.lock().unwrap();

    if (lock.available_memory() as i128) - (AVAILABLE_MEMORY_THRESHOLD as i128) - (len as i128) < 0
    {
        return None;
    }

    Some(Vec::<u8>::with_capacity(len as usize))
}

impl JobManager {
    pub fn reserve(&mut self, matrix_type: MatrixType, matrix_dimension: u32) -> Option<u8> {
        let expected_len = {
            let dim = matrix_dimension as u64;
            let type_size = matrix_type.get_type_size() as u64;
            type_size * dim * dim
        };

        match reserve_if_available(expected_len) {
            Some(vec) => Some({
                let job = Arc::new(Job {
                    status: AtomicStatus::new(StatusStripped::Reserved),
                    thread_count: AtomicU8::new(DEFAULT_THREAD_COUNT),
                    matrix_type,
                    matrix_dimension,
                    matrix_vec: Mutex::new(Some(vec)),
                });

                let index = self.job_iterator;
                self.jobs.insert(index, job);
                self.job_iterator += 1;

                index
            }),
            None => None,
        }
    }

    pub fn calc(
        &mut self,
        index: u8,
        thread_count: u8,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn Error>> {
        let job = self
            .jobs
            .get(&index)
            .ok_or("There is no data for this index")?;

        if thread_count > 0 {
            job.thread_count.store(thread_count, Ordering::Relaxed);
        }
        job.status.store(StatusStripped::Running);

        let type_size = job.matrix_type.get_type_size() as usize;
        let dim = job.matrix_dimension as usize;
        let mut vec = {
            let mut lock = job.matrix_vec.lock().unwrap();
            lock.take().unwrap()
        };
        let len = vec.capacity();
        unsafe {
            vec.set_len(len);
        }

        stream.read_exact(vec.as_mut_slice())?;

        let job = Arc::clone(job);
        std::thread::spawn(move || {
            let vec_ptr = Arc::new(AtomicPtr::new(vec.as_mut_ptr()));
            let thread_count = job.thread_count.load(Ordering::Relaxed) as usize;
            std::thread::scope(move |s| {
                for i in 0..thread_count {
                    let vec_ptr = Arc::clone(&vec_ptr);
                    s.spawn(move || {
                        let matrix_ptr = vec_ptr.load(Ordering::Relaxed);
                        let index = i;

                        let mut taken = 0;
                        for i in 0..dim {
                            for j in 0..i {
                                taken += 1;
                                if taken % thread_count != index {
                                    continue;
                                }

                                let lower_index = (i * dim + j) * type_size;
                                let upper_index = (j * dim + i) * type_size;

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
                    });
                }
            });

            let mut lock = job.matrix_vec.lock().unwrap();
            job.status.store(StatusStripped::Completed);
            let _ = lock.insert(vec);
        });

        Ok(())
    }

    pub fn poll(&mut self, index: u8) -> Status {
        match self.jobs.get(&index) {
            Some(job) => match job.status.load() {
                StatusStripped::Reserved => Status::Reserved,
                StatusStripped::Running => Status::Running,
                StatusStripped::Completed => {
                    let job = self.jobs.remove(&index).unwrap();
                    let job = match Arc::try_unwrap(job) {
                        Ok(job) => job,
                        Err(_) => {
                            panic!("Arc value is more than one, even though the job is marked as completed")
                        }
                    };
                    let matrix_bytes = job.matrix_vec.into_inner().unwrap().unwrap();

                    Status::Completed { matrix_bytes }
                }
            },
            None => Status::NoData,
        }
    }
}

pub fn new_manager() -> JobManager {
    JobManager {
        job_iterator: 1,
        jobs: HashMap::<u8, Arc<Job>>::new(),
    }
}
