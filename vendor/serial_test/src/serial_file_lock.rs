use crate::file_lock::make_lock_for_name_and_path;

#[doc(hidden)]
pub fn fs_serial_core(name: &str, path: Option<&str>, function: fn()) {
    let mut lock = make_lock_for_name_and_path(name, path);
    lock.start_serial();
    function();
    lock.end_serial();
}

#[doc(hidden)]
pub fn fs_serial_core_with_return<E>(
    name: &str,
    path: Option<&str>,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    let mut lock = make_lock_for_name_and_path(name, path);
    lock.start_serial();
    let ret = function();
    lock.end_serial();
    ret
}

#[doc(hidden)]
#[cfg(feature = "async")]
pub async fn fs_async_serial_core_with_return<E>(
    name: &str,
    path: Option<&str>,
    fut: impl std::future::Future<Output = Result<(), E>>,
) -> Result<(), E> {
    let mut lock = make_lock_for_name_and_path(name, path);
    lock.start_serial();
    let ret = fut.await;
    lock.end_serial();
    ret
}

#[doc(hidden)]
#[cfg(feature = "async")]
pub async fn fs_async_serial_core(
    name: &str,
    path: Option<&str>,
    fut: impl std::future::Future<Output = ()>,
) {
    let mut lock = make_lock_for_name_and_path(name, path);
    lock.start_serial();
    fut.await;
    lock.end_serial();
}

#[cfg(test)]
mod tests {
    use std::panic;

    use fslock::LockFile;

    use super::fs_serial_core;
    use crate::file_lock::path_for_name;

    #[test]
    fn test_serial() {
        fs_serial_core("test", None, || {});
    }

    #[test]
    fn unlock_on_assert_sync_without_return() {
        let lock_path = path_for_name("unlock_on_assert_sync_without_return");
        let _ = panic::catch_unwind(|| {
            fs_serial_core("foo", Some(&lock_path), || {
                assert!(false);
            })
        });
        let mut lockfile = LockFile::open(&lock_path).unwrap();
        assert!(lockfile.try_lock().unwrap());
    }
}
