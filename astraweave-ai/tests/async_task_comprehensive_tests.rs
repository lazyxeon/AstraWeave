//! Comprehensive test suite for AsyncTask
//!
//! This module extends the basic AsyncTask tests with comprehensive coverage
//! of edge cases, error handling, and production scenarios.
//!
//! Coverage goals:
//! - Multiple try_recv() calls (consumption semantics)
//! - Concurrent task spawning
//! - Error propagation paths
//! - Timeout edge cases (zero, very large)
//! - Task abortion scenarios
//! - is_finished() state checks

#![cfg(feature = "llm_orchestrator")]
use astraweave_ai::AsyncTask;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test that try_recv() consumes the result on first call
#[tokio::test]
async fn test_async_task_multiple_try_recv_consumes_result() {
    let handle = tokio::spawn(async { 42 });

    // Wait for completion
    sleep(Duration::from_millis(10)).await;

    let mut task = AsyncTask::new(handle);

    // First call should succeed
    let result1 = task.try_recv();
    assert!(result1.is_some());
    assert_eq!(result1.unwrap().unwrap(), 42);

    // Second call should return None (result consumed)
    let result2 = task.try_recv();
    assert!(result2.is_none());

    // Third call should still return None
    let result3 = task.try_recv();
    assert!(result3.is_none());
}

/// Test concurrent spawning of many AsyncTask instances
#[tokio::test]
async fn test_async_task_concurrent_spawns() {
    let tasks: Vec<_> = (0..100)
        .map(|i| {
            let handle = tokio::spawn(async move {
                // Very short delays to avoid timing issues
                if i % 10 == 0 {
                    sleep(Duration::from_micros(100)).await;
                }
                i as u64
            });
            AsyncTask::new(handle)
        })
        .collect();

    // Wait generously for all to complete
    sleep(Duration::from_millis(100)).await;

    // Verify all tasks complete with correct values
    for (i, mut task) in tasks.into_iter().enumerate() {
        let result = task.try_recv();
        assert!(result.is_some(), "Task {} should be complete", i);
        assert_eq!(result.unwrap().unwrap(), i as u64);
    }
}

/// Test that is_finished() returns correct status throughout lifecycle.
///
/// The task waits on a oneshot instead of a wall-clock sleep. On a loaded
/// Windows runner a 50 ms sleep can finish before the test's 20 ms sleep
/// returns, or still be running after the following 40 ms sleep.
#[tokio::test]
async fn test_async_task_is_finished_lifecycle() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn(async move { rx.await.expect("lifecycle signal") });

    let mut task = AsyncTask::new(handle);

    assert!(!task.is_finished());

    tx.send(42).expect("send lifecycle signal");
    for _ in 0..64 {
        if task.is_finished() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(task.is_finished());

    let result = task.try_recv();
    assert_eq!(result.expect("task result").expect("task value"), 42);
    assert!(task.is_finished());
}

/// Test timeout with zero duration (immediate timeout)
#[tokio::test]
async fn test_async_task_zero_timeout() {
    let handle = tokio::spawn(async {
        sleep(Duration::from_millis(10)).await;
        42
    });

    let mut task = AsyncTask::with_timeout(handle, Duration::ZERO);

    // Should timeout immediately (even without sleep)
    match task.try_recv() {
        Some(Err(e)) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("timed out"),
                "Expected timeout error, got: {}",
                err_msg
            );
        }
        other => panic!("Expected immediate timeout, got {:?}", other),
    }
}

/// Test timeout with very large duration (effectively no timeout)
#[tokio::test]
async fn test_async_task_large_timeout() {
    let handle = tokio::spawn(async {
        sleep(Duration::from_millis(10)).await;
        42
    });

    let mut task = AsyncTask::with_timeout(handle, Duration::from_secs(3600)); // 1 hour

    // Wait for task completion (with extra buffer for CI)
    sleep(Duration::from_millis(50)).await;

    // Should complete successfully (not timeout)
    match task.try_recv() {
        Some(Ok(value)) => assert_eq!(value, 42),
        other => panic!("Expected success with large timeout, got {:?}", other),
    }
}

/// Test that task abortion actually stops execution
#[tokio::test]
async fn test_async_task_abort_stops_execution() {
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let handle = tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        executed_clone.store(true, Ordering::SeqCst);
        42
    });

    let task = AsyncTask::new(handle);

    // Drop immediately (should abort)
    drop(task);

    // Wait longer than task duration
    sleep(Duration::from_millis(100)).await;

    // Task should NOT have executed (aborted before setting flag)
    assert!(
        !executed.load(Ordering::SeqCst),
        "Task should have been aborted"
    );
}

/// Test error propagation from task panic
#[tokio::test]
async fn test_async_task_panic_propagation() {
    let handle = tokio::spawn(async {
        panic!("Intentional test panic");
        #[allow(unreachable_code)]
        42
    });

    // Wait for panic to occur
    sleep(Duration::from_millis(10)).await;

    let mut task = AsyncTask::new(handle);

    // Should return error, not panic main thread
    match task.try_recv() {
        Some(Err(e)) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("join error"),
                "Expected join error from panic, got: {}",
                err_msg
            );
        }
        other => panic!("Expected panic error, got {:?}", other),
    }
}

/// Test that await_result() consumes the task
#[tokio::test]
async fn test_async_task_await_result_consumes() {
    let handle = tokio::spawn(async {
        sleep(Duration::from_millis(10)).await;
        42
    });

    let task = AsyncTask::new(handle);

    // await_result consumes self
    let result = task.await_result().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);

    // task is now consumed (can't use it again)
    // This is compile-time enforced, no runtime check needed
}

/// Test elapsed() precision during task execution
#[tokio::test]
async fn test_async_task_elapsed_precision() {
    let handle = tokio::spawn(async { 42 });
    let task = AsyncTask::new(handle);

    // Check multiple times with very generous timing windows for CI
    let elapsed1 = task.elapsed();
    sleep(Duration::from_millis(30)).await;
    let elapsed2 = task.elapsed();
    sleep(Duration::from_millis(30)).await;
    let elapsed3 = task.elapsed();

    // Elapsed should monotonically increase (this is the important property)
    assert!(
        elapsed2 > elapsed1,
        "elapsed2 ({:?}) should be > elapsed1 ({:?})",
        elapsed2,
        elapsed1
    );
    assert!(
        elapsed3 > elapsed2,
        "elapsed3 ({:?}) should be > elapsed2 ({:?})",
        elapsed3,
        elapsed2
    );

    // Very generous sanity checks (20ms tolerance for CI timing variance)
    assert!(
        elapsed1 < Duration::from_millis(20),
        "elapsed1 should be ~0ms, got {:?}",
        elapsed1
    );
    assert!(
        elapsed2 >= Duration::from_millis(20) && elapsed2 <= Duration::from_millis(60),
        "elapsed2 should be ~30ms, got {:?}",
        elapsed2
    );
    assert!(
        elapsed3 >= Duration::from_millis(50) && elapsed3 <= Duration::from_millis(90),
        "elapsed3 should be ~60ms, got {:?}",
        elapsed3
    );
}

/// Test try_recv() on task that completes with error (not panic)
#[tokio::test]
async fn test_async_task_error_result() {
    let handle = tokio::spawn(async {
        // Simulate async operation that returns error
        sleep(Duration::from_millis(10)).await;
        Err::<i32, &str>("intentional error")
    });

    sleep(Duration::from_millis(50)).await; // Extra buffer for CI

    let mut task = AsyncTask::new(handle);

    // Task should complete successfully (join succeeds)
    match task.try_recv() {
        Some(Ok(result)) => {
            // The Result<i32, &str> is the value, not a join error
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "intentional error");
        }
        other => panic!("Expected Ok(Err(...)), got {:?}", other),
    }
}

/// Test concurrent try_recv() calls (should be safe)
#[tokio::test]
async fn test_async_task_concurrent_try_recv() {
    let handle = tokio::spawn(async {
        sleep(Duration::from_millis(200)).await; // Long enough to guarantee pending state
        42
    });

    let mut task = AsyncTask::new(handle);

    // Poll several times - should all return None (pending)
    for i in 0..5 {
        let result = task.try_recv();
        assert!(
            result.is_none(),
            "Task should still be running at poll {}",
            i
        );
        sleep(Duration::from_millis(20)).await; // Total: 100ms, well under 200ms
    }

    // Wait for completion
    sleep(Duration::from_millis(120)).await; // 100ms + 120ms = 220ms total

    // Now should succeed
    let result = task.try_recv();
    assert!(result.is_some());
    assert_eq!(result.unwrap().unwrap(), 42);
}

/// Test timeout behavior when task completes just before timeout
#[tokio::test]
async fn test_async_task_timeout_race_condition() {
    // Task completes in 30ms, timeout is 100ms (more margin)
    let handle = tokio::spawn(async {
        sleep(Duration::from_millis(30)).await;
        42
    });

    let mut task = AsyncTask::with_timeout(handle, Duration::from_millis(100));

    // Wait for task completion (before timeout)
    sleep(Duration::from_millis(60)).await;

    // Should complete successfully (no timeout)
    match task.try_recv() {
        Some(Ok(value)) => assert_eq!(value, 42),
        Some(Err(e)) => panic!("Expected success, got timeout: {}", e),
        None => panic!("Expected completion, task still running"),
    }
}

/// Test is_finished() after timeout
#[tokio::test]
async fn test_async_task_is_finished_after_timeout() {
    let handle = tokio::spawn(async {
        sleep(Duration::from_secs(10)).await;
        42
    });

    let mut task = AsyncTask::with_timeout(handle, Duration::from_millis(50));

    // Not finished yet
    assert!(!task.is_finished());

    // Wait for timeout
    sleep(Duration::from_millis(60)).await;

    // Trigger timeout by calling try_recv
    let result = task.try_recv();
    assert!(result.is_some());
    assert!(result.unwrap().is_err()); // Timeout error

    // Should now report finished (task was aborted)
    assert!(task.is_finished());
}

/// Test await_result() on already-completed task
#[tokio::test]
async fn test_async_task_await_result_on_completed() {
    let handle = tokio::spawn(async { 42 });

    // Wait for completion
    sleep(Duration::from_millis(10)).await;

    let task = AsyncTask::new(handle);
    assert!(task.is_finished());

    // await_result should still work
    let result = task.await_result().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

/// Test that Drop aborts in-flight tasks correctly
#[tokio::test]
async fn test_async_task_drop_aborts_inflight() {
    let started = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));

    let started_clone = started.clone();
    let completed_clone = completed.clone();

    let handle = tokio::spawn(async move {
        started_clone.store(true, Ordering::SeqCst);
        sleep(Duration::from_millis(100)).await;
        completed_clone.store(true, Ordering::SeqCst);
        42
    });

    let task = AsyncTask::new(handle);

    // Give task time to start
    sleep(Duration::from_millis(20)).await;
    assert!(started.load(Ordering::SeqCst), "Task should have started");
    assert!(!task.is_finished());

    // Drop while in-flight
    drop(task);

    // Wait past completion time
    sleep(Duration::from_millis(100)).await;

    // Task should NOT have completed (aborted)
    assert!(
        !completed.load(Ordering::SeqCst),
        "Task should have been aborted before completion"
    );
}
