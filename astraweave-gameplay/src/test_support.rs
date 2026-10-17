//! Test-only failure path. This crate denies `unwrap`, `expect`, `panic`, and `println`.

use std::fmt::{self, Debug};
use std::io::Write;

pub fn fail(message: &str) -> ! {
    let _ = std::io::stderr().write_all(message.as_bytes());
    let _ = std::io::stderr().write_all(b"\n");
    std::process::abort();
}

pub fn test_panic(args: fmt::Arguments<'_>) -> ! {
    fail(&fmt::format(args));
}

pub trait TestValue<T> {
    fn test_unwrap(self) -> T;
    fn test_expect(self, message: &str) -> T;
}

impl<T> TestValue<T> for Option<T> {
    fn test_unwrap(self) -> T {
        match self {
            Some(value) => value,
            None => fail("expected Some"),
        }
    }

    fn test_expect(self, message: &str) -> T {
        match self {
            Some(value) => value,
            None => fail(message),
        }
    }
}

impl<T, E: Debug> TestValue<T> for Result<T, E> {
    fn test_unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => fail(&format!("{err:?}")),
        }
    }

    fn test_expect(self, message: &str) -> T {
        match self {
            Ok(value) => value,
            Err(err) => fail(&format!("{message}: {err:?}")),
        }
    }
}

pub trait TestErr<E> {
    fn test_unwrap_err(self) -> E;
}

impl<T: Debug, E> TestErr<E> for Result<T, E> {
    fn test_unwrap_err(self) -> E {
        match self {
            Err(err) => err,
            Ok(value) => fail(&format!("expected Err, got {value:?}")),
        }
    }
}

#[cfg(test)]
mod support_paths {
    use super::{test_panic, TestErr, TestValue};

    #[test]
    fn support_paths_return_the_value() {
        assert_eq!(Some(7).test_unwrap(), 7);
        assert_eq!(Some(7).test_expect("ok"), 7);
        let ok: Result<i32, &str> = Ok(7);
        assert_eq!(ok.test_expect("ok"), 7);
        let ok: Result<i32, &str> = Ok(8);
        assert_eq!(ok.test_unwrap(), 8);
        let err: Result<i32, &str> = Err("no");
        assert_eq!(err.test_unwrap_err(), "no");
        if std::env::var_os("AW_TEST_SUPPORT_PANIC").is_some() {
            test_panic(format_args!("support"));
        }
    }
}
