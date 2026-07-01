use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::{sync::Arc, vec, vec::Vec};
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let detect = process_inner.deadlock_detection_enabled;
    drop(process_inner);
    drop(process);
    if detect && mutex.locked() {
        return -0xdead;
    }
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let detect = process_inner.deadlock_detection_enabled;
    if sem_id != 0 && detect {
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        task_inner.sem_need = sem_id as isize;
        while task_inner.sem_allocated.len() <= sem_id {
            task_inner.sem_allocated.push(0);
        }
        drop(task_inner);
        //
        let cnt = process_inner.tasks.len();
        let sem_cnt = process_inner.semaphore_list.len();
        let mut available: Vec<isize> = vec![0; sem_cnt]; // work
        let mut finish: Vec<bool> = vec![false; cnt];
        let mut updated = true;
        //
        for (j, sem) in process_inner.semaphore_list.iter().enumerate() {
            if let Some(sem) = sem.as_ref() {
                let sem_inner = sem.inner.exclusive_access();
                available[j] = sem_inner.count;
            }
        }
        // print available
        if false {
            print!(
                "(semaphore_down available) [{} - sem_id{}]available: ",
                tid, sem_id
            );
            for i in 0..sem_cnt {
                print!("{} ,", available[i]);
            }
            println!("");
            if false {
                for j in 0..cnt {
                    let Some(task) = process_inner.tasks[j].as_ref() else {
                        continue;
                    };
                    let task_inner = task.inner_exclusive_access();
                    print!("(--------------) [{} {}] need: ", tid, j);
                    for i in 0..task_inner.sem_allocated.len() {
                        print!("{} ", task_inner.sem_allocated[i]); // >= 0
                    }
                    println!(
                        " {} for {};",
                        task_inner.sem_wait, //judge_vec_a_greater_equ_than_b(&available, &task_inner.sem_allocated),
                        task_inner.sem_need
                    );
                    drop(task_inner);
                }
            }
        }

        while updated {
            updated = false;
            for i in 0..cnt {
                // find a task whose Finish[i] == false;&&Need[i,j] <= Work[j];
                let Some(task) = process_inner.tasks[i].as_ref() else {
                    continue;
                };
                let task_inner = task.inner_exclusive_access();
                if !finish[i] {
                    // until all tasks are finished
                    if false && task_inner.sem_wait {
                        println!(
                            "WWW --- available[task_inner.sem_need {}] = {}",
                            task_inner.sem_need, available[task_inner.sem_need as usize]
                        );
                    }
                    if (task_inner.sem_need != 0 && available[task_inner.sem_need as usize] > 0)
                        || task_inner.sem_need == 0
                    {
                        //if (task_inner.sem_wait && available[task_inner.sem_need as usize] > 0) || !task_inner.sem_wait {
                        //if judge_vec_a_greater_equ_than_b(&available, &task_inner.sem_allocated) {
                        finish[i] = true;
                        updated = true;
                        for j in 1..task_inner.sem_allocated.len() {
                            available[j] -= task_inner.sem_allocated[j];
                        }
                        break;
                    }
                }
                drop(task_inner);
            }
        }
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        //task_inner.sem_allocated[sem_id] += 1;
        drop(task_inner);
        // check if all true
        for ii in 0..cnt {
            if !finish[ii] {
                {
                    // print out available array
                    print!("(semaphore_down fail) [{} - finish{}]available: ", tid, ii);
                    for i in 0..sem_cnt {
                        print!("{} ,", available[i]);
                    }
                    println!("");
                    if true {
                        for j in 0..cnt {
                            let Some(task) = process_inner.tasks[j].as_ref() else {
                                continue;
                            };
                            let task_inner = task.inner_exclusive_access();
                            print!("(--------------) [{} {}] need: ", tid, j);
                            for i in 0..task_inner.sem_allocated.len() {
                                print!("{} ", task_inner.sem_allocated[i]); // >= 0
                            }
                            println!(
                                " {} for {};",
                                task_inner.sem_wait, //judge_vec_a_greater_equ_than_b(&available, &task_inner.sem_allocated),
                                task_inner.sem_need
                            );
                            drop(task_inner);
                        }
                    }
                }
                return -0xdead;
            }
        }
    }
    drop(process_inner);
    //{TODE} sem.will_deadlock
    let task = current_task().unwrap();
    //let mut task_inner = task.inner_exclusive_access();
    //task_inner.sem_wait = true;
    //drop(task_inner);
    sem.down();
    let mut task_inner = task.inner_exclusive_access();
    //task_inner.sem_wait = false;
    task_inner.sem_need = 0;
    /*if sem_id != 0*/
    {
        while task_inner.sem_allocated.len() /*e.g. should-to 2*/ <= sem_id
        /*e.g. 1*/
        {
            task_inner.sem_allocated.push(0);
        }
        task_inner.sem_allocated[sem_id] -= 1; //{}{}
    }
    drop(task_inner);
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.deadlock_detection_enabled = _enabled != 0;
    if process_inner.deadlock_detection_enabled {
        println!("QwQ kernel: deadlock detection enabled.");
    }
    drop(process_inner);
    drop(process);
    0
}
