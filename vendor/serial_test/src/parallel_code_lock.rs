#![allow(clippy::await_holding_lock)]

use crate::code_lock::{check_new_key, LOCKS};
#[cfg(feature = "async")]
use futures::FutureExt;
use std::panic;

#[doc(hidden)]
pub fn local_parallel_core_with_return<E>(
    name: &str,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    check_new_key(name);

    let lock = LOCKS.get(name).unwrap();
    lock.start_parallel();
    let res = panic::catch_unwind(function);
    lock.end_parallel();
    match res {
        Ok(ret) => ret,
        Err(err) => {
            panic::resume_unwind(err);
        }
    }
}

#[doc(hidden)]
pub fn local_parallel_core(name: &str, function: fn()) {
    check_new_key(name);

    let lock = LOCKS.get(name).unwrap();
    lock.start_parallel();
    let res = panic::catch_unwind(|| {
        function();
    });
    lock.end_parallel();
    if let Err(err) = res {
        panic::resume_unwind(err);
    }
}

#[doc(hidden)]
#[cfg(feature = "async")]
pub async fn local_async_parallel_core_with_return<E>(
    name: &str,
    fut: impl std::future::Future<Output = Result<(), E>> + panic::UnwindSafe,
) -> Result<(), E> {
    check_new_key(name);

    let lock = LOCKS.get(name).unwrap();
    lock.start_parallel();
    let res = fut.catch_unwind().await;
    lock.end_parallel();
    match res {
        Ok(ret) => ret,
        Err(err) => {
            panic::resume_unwind(err);
        }
    }
}

#[doc(hidden)]
#[cfg(feature = "async")]
pub async fn local_async_parallel_core(
    name: &str,
    fut: impl std::future::Future<Output = ()> + panic::UnwindSafe,
) {
    check_new_key(name);

    let lock = LOCKS.get(name).unwrap();
    lock.start_parallel();
    let res = fut.catch_unwind().await;
    lock.end_parallel();
    if let Err(err) = res {
        panic::resume_unwind(err);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "async")]
    use crate::{local_async_parallel_core, local_async_parallel_core_with_return};

    use crate::{code_lock::LOCKS, local_parallel_core, local_parallel_core_with_return};
    use std::{io::Error, panic};

    #[test]
    fn unlock_on_assert_sync_without_return() {
        let _ = panic::catch_unwind(|| {
            local_parallel_core("unlock_on_assert_sync_without_return", || {
                assert!(false);
            })
        });
        assert_eq!(
            LOCKS
                .get("unlock_on_assert_sync_without_return")
                .unwrap()
                .parallel_count(),
            0
        );
    }

    #[test]
    fn unlock_on_assert_sync_with_return() {
        let _ = panic::catch_unwind(|| {
            local_parallel_core_with_return(
                "unlock_on_assert_sync_with_return",
                || -> Result<(), Error> {
                    assert!(false);
                    Ok(())
                },
            )
        });
        assert_eq!(
            LOCKS
                .get("unlock_on_assert_sync_with_return")
                .unwrap()
                .parallel_count(),
            0
        );
    }

    #[tokio::test]
    #[cfg(feature = "async")]
    async fn unlock_on_assert_async_without_return() {
        async fn demo_assert() {
            assert!(false);
        }
        async fn call_serial_test_fn() {
            local_async_parallel_core("unlock_on_assert_async_without_return", demo_assert()).await
        }
        // as per https://stackoverflow.com/a/66529014/320546
        let _ = panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            let _enter_guard = handle.enter();
            futures::executor::block_on(call_serial_test_fn());
        });
        assert_eq!(
            LOCKS
                .get("unlock_on_assert_async_without_return")
                .unwrap()
                .parallel_count(),
            0
        );
    }

    #[tokio::test]
    #[cfg(feature = "async")]
    async fn unlock_on_assert_async_with_return() {
        async fn demo_assert() -> Result<(), Error> {
            assert!(false);
            Ok(())
        }

        #[allow(unused_must_use)]
        async fn call_serial_test_fn() {
            local_async_parallel_core_with_return(
                "unlock_on_assert_async_with_return",
                demo_assert(),
            )
            .await;
        }

        // as per https://stackoverflow.com/a/66529014/320546
        let _ = panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            let _enter_guard = handle.enter();
            futures::executor::block_on(call_serial_test_fn());
        });
        assert_eq!(
            LOCKS
                .get("unlock_on_assert_async_with_return")
                .unwrap()
                .parallel_count(),
            0
        );
    }
}
