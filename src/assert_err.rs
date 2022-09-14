#[allow(unused_macros)]
macro_rules! assert_err {
    ($result:expr, $message:literal) => {
        assert!($result.is_err());
        assert_eq!($message, $result.unwrap_err().to_string());
    };
}

#[allow(unused_imports)]
pub(crate) use assert_err;
