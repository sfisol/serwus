// FIXME: Futile formatting, but log macro requires static string
// FIXME: Try to dynamically execute proper log macro warn/error/...
#[macro_export]
macro_rules! return_logged {
    {warn, $error_type:expr, $($arg:tt)*} => {{
        let msg = format!($($arg)*);
        log::warn!("{}", msg);
        return Err($error_type(msg))
    }};
    {error, $error_type:expr, $($arg:tt)*} => {{
        let msg = format!($($arg)*);
        log::error!("{}", msg);
        return Err($error_type(msg))
    }};
}

#[macro_export]
macro_rules! error_logged {
    {warn, $($arg:tt)*} => {{
        let msg = format!($($arg:tt)*);
        log::warn!("{}", msg);
        msg
    }};
    {error, $($arg:tt)*} => {{
        let msg = format!($($arg:tt)*);
        log::error!("{}", msg);
        msg
    }};
}
