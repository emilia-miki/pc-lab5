use std::future::Future;
use rayon::prelude::*;
use tokio::sync::oneshot::Receiver;
use std::pin::Pin;
use std::task::Poll;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use once_cell::sync::Lazy;
use sysinfo::{RefreshKind, System, SystemExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{matrix_type::MatrixType, status::Status};
use itertools::Itertools;

static SYSTEM: once_cell::sync::Lazy<tokio::sync::Mutex<sysinfo::System>> =
    Lazy::new(|| tokio::sync::Mutex::new(System::new_with_specifics(RefreshKind::new().with_memory())));

async fn reserve_if_available(len: usize) -> Result<Vec<u8>, String> {
    const AVAILABLE_MEMORY_THRESHOLD: u64 = 500_000_000;

    let lock = SYSTEM.lock().await;

    if (lock.available_memory() as i128) - (AVAILABLE_MEMORY_THRESHOLD as i128) - (len as i128) < 0
    {
        return Err(String::from("not enough memory"));
    }

    Ok(vec![0u8; len])
}

pub struct MatrixData {
    matrix_type_size: usize,
    matrix_dimensions: usize,
    matrix_vec: Vec<u8>,
}

pub enum Task {
    NoData,
    Reserved(MatrixData),
    Ready(MatrixData),
    Running,
    Completed(Vec<u8>),
}

impl Task {
    async fn reserve(
        self_arc: &Arc<tokio::sync::Mutex<Self>>,
        matrix_type: MatrixType,
        matrix_dimension: u32,
    ) -> Result<(), String> {
        let mut lock = self_arc.lock().await;
        match *lock {
            Task::NoData => (),
            _ => panic!("calling reserve on a task other than Task::NoData"),
        };

        let matrix_type_size = matrix_type.get_type_size() as usize;
        let matrix_dimension = matrix_dimension as usize;
        let matrix_vec =
            reserve_if_available(matrix_type_size * matrix_dimension * matrix_dimension).await?;

        let data = MatrixData {
            matrix_type_size,
            matrix_dimensions: matrix_dimension,
            matrix_vec,
        };

        *lock = Task::Reserved(data);
        Ok(())
    }

    async fn fill(self_arc: &Arc<tokio::sync::Mutex<Self>>, stream: &mut TcpStream) -> Result<(), String> {
        let mut lock = self_arc.lock().await;
        let task_reserved = std::mem::replace(&mut *lock, Task::NoData);

        let task_ready = match task_reserved {
            Task::Reserved(mut data) => {
                let n = stream
                    .read_exact(&mut data.matrix_vec)
                    .await
                    .map_err(|e| e.to_string())?;
                if n == 0 {
                    Err(String::from("the client disconnected"))?
                }

                Task::Ready(data)
            }
            _ => panic!("calling fill on a task other than Task::Reserved"),
        };

        *lock = task_ready;
        Ok(())
    }

    async fn run(
        arc_self: Arc<tokio::sync::Mutex<Self>>,
        thread_pool_tx: &UnboundedSender<(MatrixData, Arc<tokio::sync::Mutex<Self>>)>,
    ) {
        let task_ready = {
            let mut lock = arc_self.lock().await;
            std::mem::replace(&mut *lock, Task::Running)
        };
        match task_ready {
            Task::Ready(data) => thread_pool_tx
                .send((data, arc_self))
                .unwrap_or_else(|_| panic!("couldn't send the data to the thread pool manager")),
            _ => panic!("calling run on a task other than Task::Ready"),
        };
    }
}

struct PointerWrapper(*mut u8);
unsafe impl Sync for PointerWrapper {}

// pub async fn transpose(tp: &rayon::ThreadPool, data: MatrixData) -> Vec<u8> {
//     let type_size = data.matrix_type_size;
//     let dimension = data.matrix_dimensions;
//     let mut matrix_vec = data.matrix_vec;

//     let (tx, rx) = tokio::sync::oneshot::channel();
//     let closure = move || {
//         let vec_ptr = &PointerWrapper(matrix_vec.as_mut_ptr());
//         matrix_vec
//             .par_chunks_exact_mut(type_size)
//             .enumerate()
//             .map(|(i, ch)| (i / dimension, i % dimension, ch))
//             .filter(|(i, j, _)| i < j)
//             .map(|(i, j, lower_chunk)| {
//                 (lower_chunk, unsafe {
//                     std::slice::from_raw_parts_mut(
//                         vec_ptr.0.add((j * dimension + i) * type_size),
//                         type_size,
//                     )
//                 })
//             })
//             .for_each(|(l, u)| l.swap_with_slice(u));

//         tx.send(matrix_vec).unwrap();
//     };

//     tp.install(closure);

//     rx.await.unwrap()
// }

// actually translates to:
enum TransposeState<'a> {
    Initialized {
        tp: &'a rayon::ThreadPool,
        data: MatrixData,
    },
    AwaitingResult(Receiver<Vec<u8>>),
    Terminated,
}

struct Transpose<'a> {
    state: Arc<Mutex<TransposeState<'a>>>,

    // this would be needed if this task's work were long and didn't solely depend
    // on awaiting the oneshot receiver with the transposed matrix from the thread pool
    // waker: Option<Arc<Mutex<Waker>>>,
}

impl<'a> Future for Transpose<'a> {
    type Output = Vec<u8>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut state_lock = self.state.lock().unwrap();
        loop {
            let previous_state = std::mem::replace(&mut *state_lock, TransposeState::Terminated);
            match previous_state {
                TransposeState::Initialized { tp, data } => {
                    let type_size = data.matrix_type_size;
                    let dimension = data.matrix_dimensions;
                    let mut matrix_vec = data.matrix_vec;

                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let closure = move || {
                        let vec_ptr = &PointerWrapper(matrix_vec.as_mut_ptr());
                        matrix_vec
                            .par_chunks_exact_mut(type_size)
                            .enumerate()
                            .map(|(i, ch)| (i / dimension, i % dimension, ch))
                            .filter(|(i, j, _)| i < j)
                            .map(|(i, j, lower_chunk)| {
                                (lower_chunk, unsafe {
                                    std::slice::from_raw_parts_mut(
                                        vec_ptr.0.add((j * dimension + i) * type_size),
                                        type_size,
                                    )
                                })
                            })
                            .for_each(|(l, u)| l.swap_with_slice(u));

                        tx.send(matrix_vec).unwrap();
                    };

                    tp.install(closure);

                    *state_lock = TransposeState::AwaitingResult(rx);
                }
                TransposeState::AwaitingResult(mut rx) => {
                    match Pin::new(&mut rx).poll(cx) {
                        Poll::Ready(result) => {
                            *state_lock = TransposeState::Terminated;
                            return Poll::Ready(result.unwrap());
                        },
                        Poll::Pending => {
                            *state_lock = TransposeState::AwaitingResult(rx);
                            return Poll::Pending;
                        },
                    }
                }
                TransposeState::Terminated => {
                    panic!("polled after termination");
                }
            }
        }
    }
}

pub fn transpose(tp: &rayon::ThreadPool, data: MatrixData) -> impl Future<Output = Vec<u8>> + '_ {
    Transpose{ state: Arc::new(Mutex::new(TransposeState::Initialized { tp, data })) }
}

pub async fn process_tasks(
    tp: rayon::ThreadPool,
    mut process_tasks_channel_rx: UnboundedReceiver<(MatrixData, Arc<tokio::sync::Mutex<Task>>)>,
) {
    loop {
        let (data, task_arc) = process_tasks_channel_rx.recv().await.unwrap();

        let matrix_vec = transpose(&tp, data).await;

        let task = Task::Completed(matrix_vec);
        let mut lock = task_arc.lock().await;
        match *lock {
            Task::Running => *lock = task,
            _ => panic!(
                "trying to complete the task, but it's current state is other than Task::Running"
            ),
        };
    }
}

pub struct JobManager {
    task_iterator: usize,
    tasks: HashMap<usize, Arc<tokio::sync::Mutex<Task>>>,
    process_tasks_channel_tx: UnboundedSender<(MatrixData, Arc<tokio::sync::Mutex<Task>>)>,
}

impl JobManager {
    pub async fn reserve(
        &mut self,
        matrix_type: MatrixType,
        matrix_dimension: u32,
    ) -> Result<usize, String> {
        let task_arc = Arc::new(tokio::sync::Mutex::new(Task::NoData));
        Task::reserve(&task_arc, matrix_type, matrix_dimension).await?;

        let id = self.task_iterator;
        self.tasks.insert(id, task_arc);
        self.task_iterator += 1;
        Ok(id)
    }

    pub async fn calc(&mut self, id: usize, stream: &mut TcpStream) -> Result<(), String> {
        let task_arc = self.tasks.get(&id).ok_or("the id is not reserved")?;
        Task::fill(task_arc, stream).await?;
        Task::run(Arc::clone(task_arc), &self.process_tasks_channel_tx).await;
        Ok(())
    }

    pub async fn poll(&mut self, id: usize) -> Status {
        let task = self.tasks.get(&id);
        match task {
            Some(task_arc) => {
                let task = task_arc.lock().await;
                match *task {
                    Task::Reserved(_) => return Status::Reserved,
                    Task::Ready(_) => return Status::Running,
                    Task::Running => return Status::Running,
                    Task::NoData => {
                        panic!("found a Task::NoData in the hash map, which is not allowed")
                    }
                    Task::Completed(_) => (),
                };
            }
            None => return Status::NoData,
        };

        let task_arc = self.tasks.remove(&id).unwrap();
        let task_lock = Arc::try_unwrap(task_arc).unwrap_or_else(|_| {
            panic!("arc reference count is more then one, but the task is marked as completed")
        });
        let task = task_lock.into_inner();
        match task {
            Task::Completed(matrix_bytes) => Status::Completed { matrix_bytes },
            _ => unreachable!(),
        }
    }
}

pub fn new_manager(
    tx: tokio::sync::mpsc::UnboundedSender<(MatrixData, Arc<tokio::sync::Mutex<Task>>)>,
) -> JobManager {
    JobManager {
        task_iterator: 1,
        tasks: HashMap::<usize, Arc<tokio::sync::Mutex<Task>>>::new(),
        process_tasks_channel_tx: tx,
    }
}

trait FormatAsMatrix {
    fn format_as_matrix(&self, type_size: usize, dim: usize) -> String;
}

impl FormatAsMatrix for Vec<u8> {
    fn format_as_matrix(&self, type_size: usize, dim: usize) -> String {
        const MAX_CORNER_DIM: usize = 5;

        self.chunks_exact(type_size)
            .map(|num| format!("{:>16}", hex::encode(num)))
            .enumerate()
            .filter(|(i, _)| (i / dim <= MAX_CORNER_DIM) && (i % dim <= MAX_CORNER_DIM))
            .group_by(|(i, _)| i / MAX_CORNER_DIM)
            .into_iter()
            .map(|(_, g)| g.map(|(_, num_str)| num_str).join(" "))
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TestCase {
        num_threads: usize,
        matrix_type_size: usize,
        matrix_dimensions: usize,
        duration: std::time::Duration,
    }

    fn test_cases() -> Vec<TestCase> {
        [1, 2, 4, 8]
            .into_par_iter()
            .map(|matrix_type_size| {
                [10, 100, 1000, 10000, 12500]
                    .into_par_iter()
                    .map(move |matrix_dimensions| {
                        (1..10)
                            .into_par_iter()
                            .map(move |num_threads| TestCase {
                                num_threads,
                                matrix_type_size,
                                matrix_dimensions,
                                duration: std::time::Duration::default(),
                            })
                            .collect::<Vec<TestCase>>()
                    })
                    .flatten()
                    .collect::<Vec<TestCase>>()
            })
            .flatten()
            .collect()
    }

    #[test]
    fn transposition_verbose() {
        _transposition(true);
    }

    #[test]
    fn transposition() {
        _transposition(false);
    }

    fn _transposition(print_matrices: bool) {
        let mut test_cases = test_cases();

        for test_case in test_cases.iter_mut() {
            let type_size = test_case.matrix_type_size;
            let dim = test_case.matrix_dimensions;

            let line = format!(
                "transposing a {}x{} matrix of type_size {} on {} threads",
                dim, dim, type_size, test_case.num_threads
            );
            print!("{}", line);
            std::io::stdout().flush().unwrap();

            let tp = rayon::ThreadPoolBuilder::new()
                .num_threads(test_case.num_threads)
                .build()
                .unwrap();

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let orig_vec: Vec<u8> = (0..type_size * dim * dim)
                    .into_par_iter()
                    .map(|_| rand::random())
                    .collect();

                let matrix_data = MatrixData {
                    matrix_type_size: test_case.matrix_type_size,
                    matrix_dimensions: test_case.matrix_dimensions,
                    matrix_vec: orig_vec.clone(),
                };

                let begin_time = std::time::Instant::now();
                let transposed_vec = transpose(&tp, matrix_data).await;
                let end_time = std::time::Instant::now();
                test_case.duration = end_time - begin_time;

                print!("\r{}\r", " ".repeat(line.len()));
                if print_matrices {
                    println!();
                    println!("original matrix:");
                    println!("{}", orig_vec.format_as_matrix(type_size, dim));
                    println!();
                    println!("transposed matrix:");
                    println!("{}", transposed_vec.format_as_matrix(type_size, dim));
                    println!();
                }
                std::io::stdout().flush().unwrap();

                let transposed_vec_chunks = transposed_vec.par_chunks_exact(type_size).collect::<Vec<&[u8]>>();
                orig_vec
                    .par_chunks_exact(type_size)
                    .enumerate()
                    .map(|(i, ch)| (i / dim, i % dim, ch))
                    .for_each(|(i, j, ch)| assert_eq!(ch, transposed_vec_chunks[j * dim + i], 
                            "assertion failed for a {}x{} matrix of type_size {} bytes:\n\noriginal:\n{}\n\ntransposed:\n{}\n\n",
                            dim,
                            dim,
                            type_size,
                            orig_vec.format_as_matrix(type_size, dim),
                            transposed_vec.format_as_matrix(type_size, dim)
                ));
            });
        }
        println!();

        test_cases
            .iter()
            .group_by(|tc| (tc.matrix_type_size, tc.matrix_dimensions))
            .into_iter()
            .map(|(_, g)| g.collect_vec())
            .for_each(|vec| {
                let min_duration = vec.iter().map(|tc| tc.duration).min().unwrap().as_nanos();

                vec.into_iter().for_each(|tc| {
                    println!(
                        "type_size {} bytes - dim {:>6} - {} threads - {:>12} ns - {:.2}%",
                        tc.matrix_type_size,
                        tc.matrix_dimensions,
                        tc.num_threads,
                        tc.duration.as_nanos(),
                        (100.0 * tc.duration.as_nanos() as f64 / min_duration as f64)
                    );
                });
            });
    }
}
