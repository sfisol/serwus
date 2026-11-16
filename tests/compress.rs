//! `Serwus::compress`: the responses of the whole service compressed with what the client
//! accepts, the routes of the service and serwus's own.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

use serwus::{
    EmptyStats,
    server::{Serwus, default_cors},
    web,
};

#[derive(Clone, EmptyStats)]
struct AppData;

/// Big enough for compression to pay.
#[cfg_attr(feature = "swagger", paperclip::actix::api_v2_operation)]
async fn page() -> String {
    "Hello world\n".repeat(200)
}

/// Starts the service on a free port, compressing or not, and answers the `Content-Encoding` of
/// the response to `path`, asked with `Accept-Encoding: gzip`.
async fn content_encoding(compress: bool, path: &str) -> Option<String> {
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a free port")
        .port();
    let port: &'static str = port.to_string().leak();
    let serwus = Serwus::default()
        .set_bind_address("127.0.0.1")
        .set_app_port(port);
    let serwus = if compress { serwus.compress() } else { serwus };
    actix_web::rt::spawn(serwus.start(
        || AppData,
        |app| {
            app.route("/page", web::get().to(page));
        },
        default_cors,
    ));

    for _ in 0..100 {
        if let Ok(head) = response_head(port, path) {
            assert!(head.starts_with("HTTP/1.1 200"), "{path}: {head}");
            return head.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-encoding")
                    .then(|| value.trim().to_string())
            });
        }
        actix_web::rt::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the service didn't answer on port {port}");
}

/// The status line and the headers of the response to `GET path`, asked over plain HTTP with
/// `Accept-Encoding: gzip`. The service answers from its own threads.
fn response_head(port: &str, path: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = String::from_utf8_lossy(&response);
    let head = response.split("\r\n\r\n").next().unwrap_or_default();
    Ok(head.to_string())
}

#[actix_web::test]
async fn compress_compresses_the_routes_of_the_service() {
    assert_eq!(
        content_encoding(true, "/page").await.as_deref(),
        Some("gzip")
    );
}

#[actix_web::test]
async fn compress_compresses_the_routes_of_serwus_too() {
    assert_eq!(
        content_encoding(true, "/_stats").await.as_deref(),
        Some("gzip")
    );
}

#[actix_web::test]
async fn without_compress_nothing_is_compressed() {
    assert_eq!(content_encoding(false, "/page").await, None);
}
