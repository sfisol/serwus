// FIXME: Futile formatting, but log macro requires static string
// FIXME: Try to dynamically execute proper log macro warn/error/...
#[macro_export]
macro_rules! return_logged {
    {warn, $error_type:expr, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        log::warn!("{}", msg);
        return Err($error_type(msg))
    }};
    {error, $error_type:expr, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        log::error!("{}", msg);
        return Err($error_type(msg))
    }};
    {warn, $error_type:expr, $message:expr} => {{
        log::warn!($message);
        return Err($error_type($message.to_string()))
    }};
    {error, $error_type:expr, $message:expr, $error:expr} => {{
        log::error!($message);
        return Err($error_type($message.to_string()))
    }};
}

#[macro_export]
macro_rules! error_logged {
    {warn, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        log::warn!("{}", msg);
        msg
    }};
    {error, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        log::error!("{}", msg);
        msg
    }};
    {warn, $message:expr} => {{
        log::warn!($message);
        $message.to_string()
    }};
    {error, $message:expr, $error:expr} => {{
        log::error!($message);
        $message.to_string()
    }};
}
