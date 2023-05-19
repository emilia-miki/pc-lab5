use core::slice;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU8, Arc},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Mutex,
    },
    time,
};

pub struct Matrix {
    pub type_size: u8,
    pub dimension: u32,
    pub bytes: Mutex<Option<Vec<u8>>>,
}

#[derive(Copy, Clone)]
pub enum Status {
    NoData,
    Ready,
    Running,
    Completed,
}

impl std::convert::TryFrom<u8> for Status {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Status::NoData),
            1 => Ok(Status::Ready),
            2 => Ok(Status::Running),
            3 => Ok(Status::Completed),
            _ => Err("Invalid value stored in AtomicStatus".into()),
        }
    }
}

impl std::convert::From<Status> for u8 {
    fn from(value: Status) -> Self {
        match value {
            Status::NoData => 0,
            Status::Ready => 1,
            Status::Running => 2,
            Status::Completed => 3,
        }
    }
}

struct AtomicStatus {
    status: AtomicU8,
}

impl AtomicStatus {
    pub fn new(status: Status) -> AtomicStatus {
        AtomicStatus {
            status: AtomicU8::new(u8::from(status)),
        }
    }

    pub fn load(&self) -> Status {
        Status::try_from(self.status.load(Ordering::Relaxed)).unwrap()
    }

    pub fn store(&self, status: Status) {
        self.status.store(u8::from(status), Ordering::Relaxed)
    }
}

struct Job {
    status: AtomicStatus,
    thread_count: AtomicU8,
    matrix: Matrix,
}

pub struct JobManager {
    job_iterator: u8,
    jobs: HashMap<u8, Arc<Job>>,
}

const DEFAULT_THREAD_COUNT: u8 = 4;

impl JobManager {
    pub fn add_job(&mut self, matrix: Matrix) -> u8 {
        let job = Arc::new(Job {
            status: AtomicStatus::new(Status::Ready),
            thread_count: AtomicU8::new(DEFAULT_THREAD_COUNT),
            matrix,
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

        let job_start_time = time::Instant::now();

        job.status.store(Status::Running);

        let job_arc = Arc::clone(job);
        let spawning_thread_name = String::from(std::thread::current().name().unwrap());
        std::thread::spawn(move || {
            let spawning_thread_name = &spawning_thread_name;

            let mut matrix_vec = {
                let mut matrix_buffer = job_arc.matrix.bytes.lock().unwrap();
                matrix_buffer.take().unwrap()
            };
            let matrix_ptr = Arc::new(AtomicPtr::new(matrix_vec.as_mut_ptr()));

            let job_arc = &job_arc;
            std::thread::scope(move |s| {
                let type_size = job_arc.matrix.type_size as usize;
                let thread_count = thread_count as usize;
                let mut threads: Vec<std::thread::ScopedJoinHandle<()>> =
                    Vec::with_capacity(thread_count);

                for i in 0..thread_count {
                    let matrix_ptr = Arc::clone(&matrix_ptr);
                    threads.push(s.spawn(move || {
                        let matrix_ptr = matrix_ptr.load(Ordering::Relaxed);

                        let index = i;
                        let dimension = job_arc.matrix.dimension as usize;

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
                let mut matrix_buffer_opt = job_arc.matrix.bytes.lock().unwrap();
                let _ = matrix_buffer_opt.insert(matrix_vec);
            }

            job_arc.status.store(Status::Completed);

            let job_end_time = time::Instant::now();
            println!(
                "{}: It took {} ms to complete the calculation",
                spawning_thread_name,
                (job_end_time - job_start_time).as_millis()
            );
        });

        Ok(())
    }

    pub fn get_status(&self, index: u8) -> Status {
        match self.jobs.get(&index) {
            Some(job) => job.status.load(),
            None => Status::NoData,
        }
    }

    pub fn get_result(&mut self, index: u8) -> Option<Vec<u8>> {
        let job = self.jobs.remove(&index)?;
        let job = match Arc::try_unwrap(job) {
            Ok(job) => job,
            Err(_) => {
                eprintln!("Arc value is more than one, but the job is marked as completed");

                return None;
            }
        };

        let mut matrix_buffer = job.matrix.bytes.lock().unwrap();
        if matrix_buffer.is_none() {
            return None;
        };

        matrix_buffer.take()
    }
}

pub fn new_manager() -> JobManager {
    JobManager {
        job_iterator: 0,
        jobs: HashMap::<u8, Arc<Job>>::new(),
    }
}
