use std::{
    collections::HashMap,
    slice,
    sync::atomic::{AtomicPtr, Ordering},
    sync::{atomic::AtomicU8, Arc},
};

pub struct Matrix {
    pub type_size: u8,
    pub dimension: u32,
    pub bytes: Vec<u8>,
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
    bytes_ptr: AtomicPtr<u8>,
}

pub struct JobManager {
    job_iterator: u8,
    jobs: HashMap<u8, Arc<Job>>,
}

const DEFAULT_THREAD_COUNT: u8 = 4;

impl JobManager {
    pub fn add_job(&mut self, mut matrix: Matrix) -> u8 {
        let ptr = matrix.bytes.as_mut_ptr();
        let job = Arc::new(Job {
            status: AtomicStatus::new(Status::Ready),
            thread_count: AtomicU8::new(DEFAULT_THREAD_COUNT),
            matrix,
            bytes_ptr: AtomicPtr::new(ptr),
        });

        let index = self.job_iterator;
        self.jobs.insert(index, job);
        self.job_iterator += 1;

        index
    }

    pub fn start_job(&self, index: u8, thread_count: u8) -> Result<(), String> {
        let job = self
            .jobs
            .get(&index)
            .ok_or("There is no data for this index")?;

        if thread_count > 0 {
            job.thread_count.store(thread_count, Ordering::Relaxed);
        }

        job.status.store(Status::Running);

        let job = Arc::clone(job);
        std::thread::spawn(move || {
            std::thread::scope(move |s| {
                let len = job.matrix.bytes.len();
                let vec = job.bytes_ptr.load(Ordering::Relaxed);
                let vec: &mut [u8] = unsafe { slice::from_raw_parts_mut(vec, len) };
                let mut splits = split_vec(
                    vec,
                    job.matrix.type_size,
                    job.matrix.dimension,
                    thread_count,
                );

                let mut threads: Vec<std::thread::ScopedJoinHandle<()>> = Vec::with_capacity(4);

                for _ in 0..4 {
                    let split = splits.pop().unwrap();
                    threads.push(s.spawn(move || {
                        let (mut vec1, mut vec2) = split;
                        for i in 0..vec1.len() {
                            vec1[i].swap_with_slice(vec2[i]);
                        }
                    }));
                }

                for thread in threads {
                    match thread.join() {
                        Ok(()) => (),
                        Err(_) => eprintln!("A thread returned an error"),
                    }
                }

                job.status.store(Status::Completed);
            });
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
            Err(_) => panic!("Arc value is more than one, but the job is marked as completed"),
        };
        Some(job.matrix.bytes)
    }
}

pub fn new_manager() -> JobManager {
    JobManager {
        job_iterator: 0,
        jobs: HashMap::<u8, Arc<Job>>::new(),
    }
}

fn split_vec(
    vec: &mut [u8],
    type_size: u8,
    dimension: u32,
    thread_count: u8,
) -> Vec<(Vec<&mut [u8]>, Vec<&mut [u8]>)> {
    let thread_count: u32 = thread_count.into();
    let type_size: usize = type_size.into();
    let dimension: u32 = dimension;

    let full_len = dimension * (dimension + 1) / 2;
    let part_len = full_len / thread_count;

    let mut result = Vec::<(Vec<&mut [u8]>, Vec<&mut [u8]>)>::with_capacity(4);

    let capacity = {
        let capacity = full_len - part_len * (thread_count - 1);
        capacity.try_into().unwrap()
    };

    for _ in 0..thread_count {
        result.push((
            Vec::<&mut [u8]>::with_capacity(capacity),
            Vec::<&mut [u8]>::with_capacity(capacity),
        ));
    }

    let mut i = 0;
    let mut j = 0;
    let mut taken = (0, 0);
    let mut thread_index = (0, 0);
    for slice in vec.chunks_exact_mut(type_size) {
        if i == j {
            j += 1;
            if j % dimension == 0 {
                j = 0;
                i += 1;
            }

            continue;
        }

        let (result, taken, thread_index) = if i < j {
            (
                &mut result[thread_index.0].0,
                &mut taken.0,
                &mut thread_index.0,
            )
        } else {
            (
                &mut result[thread_index.1].1,
                &mut taken.1,
                &mut thread_index.1,
            )
        };

        result.push(slice);
        *taken += 1;
        *thread_index += 1;

        if u32::try_from(*thread_index).unwrap() < thread_count - 1 && *taken % part_len == 0 {
            *taken = 0;
            *thread_index += 1;
        }

        j += 1;
        if j % dimension == 0 {
            j = 0;
            i += 1;
        }
    }

    result
}
