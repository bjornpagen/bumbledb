use super::*;

#[test]
fn cloned_contexts_observe_cancellation() {
    let work = WorkContext::new();
    let clone = work.clone();
    assert_eq!(work.checkpoint(), Ok(()));
    assert_eq!(clone.checkpoint(), Ok(()));
    clone.cancel();
    assert_eq!(work.checkpoint(), Err(WorkError::Cancelled));
    assert_eq!(clone.checkpoint(), Err(WorkError::Cancelled));
    work.cancel();
    assert_eq!(clone.checkpoint(), Err(WorkError::Cancelled));
}

#[test]
fn cancellation_does_not_stop_independent_operations() {
    let stopped = WorkContext::new();
    let live = WorkContext::new();
    stopped.cancel();
    assert_eq!(live.checkpoint(), Ok(()));
    assert_eq!(stopped.checkpoint(), Err(WorkError::Cancelled));
}

#[test]
fn cancellation_crosses_threads_without_retaining_the_original_handle() {
    let work = WorkContext::new();
    let worker = work.clone();
    let task = std::thread::spawn(move || {
        while worker.checkpoint().is_ok() {
            std::thread::yield_now();
        }
        worker.checkpoint()
    });
    work.cancel();
    drop(work);
    assert_eq!(task.join().unwrap(), Err(WorkError::Cancelled));
}
