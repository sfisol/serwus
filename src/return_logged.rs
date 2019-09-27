// FIXME: Futile formatting, but log macro requires static string
// FIXME: Try to dynamically execute proper log macro warn/error/...
#[macro_export]
macro_rules! return_logged {
    {warn, $error_type:expr, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        warn!("{}", msg);
        return Err($error_type(msg))
    }};
    {error, $error_type:expr, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        warn!("{}", msg);
        return Err($error_type(msg))
    }};
    {warn, $error_type:expr, $message:expr} => {{
        warn!($message);
        return Err($error_type($message.to_string()))
    }};
    {error, $error_type:expr, $message:expr, $error:expr} => {{
        warn!($message);
        return Err($error_type($message.to_string()))
    }};
}

#[macro_export]
macro_rules! error_logged {
    {warn, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        warn!("{}", msg);
        msg
    }};
    {error, $message:expr, $error:expr} => {{
        let msg = format!($message, $error);
        warn!("{}", msg);
        msg
    }};
    {warn, $message:expr} => {{
        warn!($message);
        $message.to_string()
    }};
    {error, $message:expr, $error:expr} => {{
        warn!($message);
        $message.to_string()
    }};
}
