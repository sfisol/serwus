//! Reading `multipart/form-data` into memory.
//!
//! Both functions check their limits while the payload streams in: a part that grows past the
//! limit stops being kept at once, so a client cannot make the server buffer more than the
//! limits allow, no matter how much it sends.
//!
//! The rest of such a payload is still read and thrown away, up to a limit of its own. Browsers
//! do not show a response that arrives while they are still sending the request — when the
//! server stops reading an upload half-way and closes the connection, the user gets a
//! connection error page instead of the message about the file being too large.

use std::{error::Error, fmt};

use actix_multipart::{Field, MultipartError};
use actix_web::error::PayloadError;
use bytes::{Bytes, BytesMut};
use futures::TryStreamExt;

pub use actix_multipart::Multipart;

/// How many times the size limit of a part is read and thrown away after the limit is crossed.
const DRAIN_FACTOR: usize = 16;

/// Helper for reading multipart/form-data, returning vector of read bytes.
///
/// Each non-empty part becomes `(bytes, position, Content-Disposition header)`. The position is
/// the part name parsed as an integer, or the index of the part when the name is not a number.
/// Reading stops after `max_parts` non-empty parts. A part bigger than `max_upload_size` bytes
/// ends with [`PayloadError::Overflow`] as soon as it crosses the limit; the rest of the payload,
/// up to 16 times `max_upload_size`, is then read and thrown away, so that the client gets
/// the response.
pub async fn read_bytes(
    mut payload: Multipart,
    max_parts: u64,
    max_upload_size: usize,
) -> Result<Vec<(Bytes, i32, Option<String>)>, MultipartError> {
    let mut files = Vec::new();
    // Used to set default position if there is no name or value is not integer
    let mut incr_default_position = 0;

    // Iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let Some(file) = read_field(&mut field, max_upload_size).await? else {
            let max_drain = max_upload_size.saturating_mul(DRAIN_FACTOR);
            drain(&mut payload, Some(field), max_drain).await;
            return Err(MultipartError::Payload(PayloadError::Overflow));
        };

        if !file.is_empty() {
            let position: i32 = field
                .content_disposition()
                .and_then(|content_disposition| {
                    content_disposition
                        .get_name()
                        .and_then(|r| r.parse::<i32>().ok())
                })
                .unwrap_or(incr_default_position);

            files.push((
                file,
                position,
                field
                    .headers()
                    .get("Content-Disposition")
                    .and_then(|v| v.to_str().ok().map(String::from)),
            ));
        }

        incr_default_position += 1; // Increment default position

        if files.len() as u64 >= max_parts {
            break;
        }
    }

    Ok(files)
}

/// One part of a `multipart/form-data` payload, read into memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// Field name — `name` from `Content-Disposition`.
    pub name: Option<String>,
    /// File name — `filename` from `Content-Disposition`. Browsers send it for every file
    /// input, as an empty string when no file was chosen.
    pub filename: Option<String>,
    /// `Content-Type` of the part as declared by the client — not checked against the content.
    pub content_type: Option<String>,
    pub bytes: Bytes,
}

impl Part {
    /// Content of a plain form field; `None` when it is not valid UTF-8.
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes).ok()
    }
}

/// Limits for [`read_parts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartLimits {
    /// Most parts in the payload.
    pub max_parts: usize,
    /// Most bytes of a single part.
    pub max_part_size: usize,
    /// Most bytes read and thrown away once a limit is crossed, so that the client finishes
    /// sending and gets the response. A longer payload is cut off — the client then sees
    /// a connection error.
    pub max_drain: usize,
}

impl PartLimits {
    /// Up to `max_parts` parts of up to `max_part_size` bytes each. Once a limit is crossed,
    /// up to 16 times `max_part_size` more bytes are read and thrown away.
    pub const fn new(max_parts: usize, max_part_size: usize) -> Self {
        Self {
            max_parts,
            max_part_size,
            max_drain: max_part_size.saturating_mul(DRAIN_FACTOR),
        }
    }
}

/// Why [`read_parts`] did not read the payload.
#[derive(Debug)]
pub enum ReadPartsError {
    /// Malformed payload, not `multipart/form-data`, or the connection broke.
    Multipart(MultipartError),
    /// More parts than allowed.
    TooManyParts,
    /// A part grew past the allowed size; `name` is its field name.
    PartTooLarge { name: Option<String> },
}

impl fmt::Display for ReadPartsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Multipart(error) => write!(f, "multipart payload: {error}"),
            Self::TooManyParts => f.write_str("too many parts in multipart payload"),
            Self::PartTooLarge { name: Some(name) } => {
                write!(f, "multipart part \"{name}\" is too large")
            }
            Self::PartTooLarge { name: None } => f.write_str("multipart part is too large"),
        }
    }
}

impl Error for ReadPartsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Multipart(error) => Some(error),
            Self::TooManyParts | Self::PartTooLarge { .. } => None,
        }
    }
}

impl From<MultipartError> for ReadPartsError {
    fn from(error: MultipartError) -> Self {
        Self::Multipart(error)
    }
}

/// Reads all parts of `payload` into memory, in order, with their field names, file names
/// and content types — files and plain fields alike, empty ones included.
///
/// A payload with more than [`PartLimits::max_parts`] parts, or a part bigger than
/// [`PartLimits::max_part_size`] bytes, stops being kept as soon as the limit is crossed, so at
/// most `max_parts * max_part_size` bytes are ever buffered. The rest of it is read and thrown
/// away up to [`PartLimits::max_drain`] bytes, so that a browser shows the response.
///
/// ```no_run
/// use actix_web::{HttpResponse, Responder};
/// use serwus::utils::read_bytes::{Multipart, PartLimits, ReadPartsError, read_parts};
///
/// async fn upload(payload: Multipart) -> impl Responder {
///     match read_parts(payload, PartLimits::new(4, 1024 * 1024)).await {
///         Ok(parts) => {
///             let file = parts.iter().find(|part| part.name.as_deref() == Some("file"));
///             HttpResponse::Ok().body(format!("{} bytes", file.map_or(0, |file| file.bytes.len())))
///         }
///         Err(ReadPartsError::PartTooLarge { .. }) => HttpResponse::PayloadTooLarge().finish(),
///         Err(_) => HttpResponse::BadRequest().finish(),
///     }
/// }
/// ```
pub async fn read_parts(
    mut payload: Multipart,
    limits: PartLimits,
) -> Result<Vec<Part>, ReadPartsError> {
    let mut parts = Vec::new();

    while let Some(mut field) = payload.try_next().await? {
        if parts.len() >= limits.max_parts {
            drain(&mut payload, Some(field), limits.max_drain).await;
            return Err(ReadPartsError::TooManyParts);
        }
        let name = field.name().map(String::from);
        let Some(bytes) = read_field(&mut field, limits.max_part_size).await? else {
            drain(&mut payload, Some(field), limits.max_drain).await;
            return Err(ReadPartsError::PartTooLarge { name });
        };
        let filename = field
            .content_disposition()
            .and_then(|content_disposition| content_disposition.get_filename())
            .map(String::from);
        let content_type = field.content_type().map(ToString::to_string);

        parts.push(Part {
            name,
            filename,
            content_type,
            bytes,
        });
    }

    Ok(parts)
}

/// Content of one part; `None` as soon as it grows past `max_size` bytes — the rest of it
/// stays unread.
async fn read_field(field: &mut Field, max_size: usize) -> Result<Option<Bytes>, MultipartError> {
    let mut bytes = BytesMut::new();

    while let Some(chunk) = field.try_next().await? {
        if bytes.len() + chunk.len() > max_size {
            return Ok(None);
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(Some(bytes.freeze()))
}

/// Reads and throws away the rest of `payload`, starting with the unfinished `field`, up to
/// `max_bytes` bytes. The rest of a field is counted here chunk by chunk — `Multipart` would
/// skip it without any limit when asked for the next field. Errors just end it: the response
/// goes out either way.
async fn drain(payload: &mut Multipart, mut field: Option<Field>, max_bytes: usize) {
    let mut left = max_bytes;

    loop {
        if let Some(mut current) = field.take() {
            while let Ok(Some(chunk)) = current.try_next().await {
                match left.checked_sub(chunk.len()) {
                    Some(rest) => left = rest,
                    None => return,
                }
            }
        }
        match payload.try_next().await {
            Ok(Some(next)) => field = Some(next),
            Ok(None) | Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use actix_multipart::test::{
        TestFormField, create_form_data_payload_and_headers_from_fields_with_boundary,
    };
    use actix_web::{http::header::HeaderMap, mime};
    use futures::{StreamExt, stream};

    use super::*;

    fn form(fields: Vec<TestFormField<'static>>) -> (Bytes, HeaderMap) {
        create_form_data_payload_and_headers_from_fields_with_boundary("granica", fields)
    }

    /// The payload in chunks of `chunk` bytes, counting how many of them were taken.
    fn chunked(body: Bytes, headers: &HeaderMap, chunk: usize) -> (Multipart, Rc<AtomicUsize>) {
        let taken = Rc::new(AtomicUsize::new(0));
        let counter = taken.clone();
        let chunks: Vec<Bytes> = body.chunks(chunk).map(Bytes::copy_from_slice).collect();
        let stream = stream::iter(chunks).map(move |chunk| {
            counter.fetch_add(1, Ordering::Relaxed);
            Ok::<_, PayloadError>(chunk)
        });
        (Multipart::new(headers, stream), taken)
    }

    fn payload(body: Bytes, headers: &HeaderMap) -> Multipart {
        chunked(body, headers, 64).0
    }

    /// A 1 MiB part in 1 KiB chunks over a 10 KiB limit: reading has to stop long before the
    /// part ends. `actix-multipart` buffers some chunks ahead of the field, so not right at
    /// the 10th.
    fn assert_read_early(taken: &AtomicUsize) {
        let taken = taken.load(Ordering::Relaxed);
        assert!(
            taken < 128,
            "{taken} of 1024 chunks taken for a 10 KiB limit"
        );
    }

    #[actix_web::test]
    async fn parts_with_names_file_names_and_types() {
        let (body, headers) = form(vec![
            TestFormField::new("title", "Zażółć gęślą jaźń"),
            TestFormField::new("file", &b"\x89PNG\r\n"[..])
                .filename("image.png")
                .content_type(mime::IMAGE_PNG),
            TestFormField::new("empty", "").filename(""),
        ]);

        let parts = read_parts(payload(body, &headers), PartLimits::new(3, 1024))
            .await
            .unwrap();

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].name.as_deref(), Some("title"));
        assert_eq!(parts[0].text(), Some("Zażółć gęślą jaźń"));
        assert_eq!(parts[0].filename, None);
        assert_eq!(parts[1].filename.as_deref(), Some("image.png"));
        assert_eq!(parts[1].content_type.as_deref(), Some("image/png"));
        assert_eq!(&parts[1].bytes[..], b"\x89PNG\r\n");
        // a file input with no file chosen
        assert_eq!(parts[2].filename.as_deref(), Some(""));
        assert!(parts[2].bytes.is_empty());
    }

    #[actix_web::test]
    async fn too_many_parts() {
        let (body, headers) = form(vec![
            TestFormField::new("a", "Ala"),
            TestFormField::new("b", "ma"),
            TestFormField::new("c", "kota"),
        ]);

        let error = read_parts(payload(body, &headers), PartLimits::new(2, 1024))
            .await
            .unwrap_err();

        assert!(matches!(error, ReadPartsError::TooManyParts), "{error:?}");
    }

    /// 1 KiB chunks of a form with a 1 MiB file.
    fn big_file() -> (Bytes, HeaderMap, usize) {
        let (body, headers) = form(vec![
            TestFormField::new("title", "Ala ma kota"),
            TestFormField::new("file", vec![b'x'; 1024 * 1024]).filename("big.bin"),
        ]);
        let chunks = body.len().div_ceil(1024);
        (body, headers, chunks)
    }

    /// A part over the limit stops being kept, but the rest of the payload is read and thrown
    /// away, so that the client gets the response.
    #[actix_web::test]
    async fn part_too_large_drains_the_rest() {
        let (body, headers, chunks) = big_file();
        let (multipart, taken) = chunked(body, &headers, 1024);
        let limits = PartLimits {
            max_drain: 2 * 1024 * 1024,
            ..PartLimits::new(4, 10 * 1024)
        };

        let error = read_parts(multipart, limits).await.unwrap_err();

        match error {
            ReadPartsError::PartTooLarge { name } => assert_eq!(name.as_deref(), Some("file")),
            error => panic!("{error:?}"),
        }
        assert_eq!(taken.load(Ordering::Relaxed), chunks);
    }

    /// Past `max_drain` the payload is cut off, not read to the end.
    #[actix_web::test]
    async fn drain_has_a_limit() {
        let (body, headers, _) = big_file();
        let (multipart, taken) = chunked(body, &headers, 1024);
        let limits = PartLimits {
            max_drain: 0,
            ..PartLimits::new(4, 10 * 1024)
        };

        let error = read_parts(multipart, limits).await.unwrap_err();

        assert!(
            matches!(error, ReadPartsError::PartTooLarge { .. }),
            "{error:?}"
        );
        assert_read_early(&taken);
    }

    #[actix_web::test]
    async fn default_drain_is_16_parts() {
        assert_eq!(PartLimits::new(4, 1024).max_drain, 16 * 1024);
    }

    #[actix_web::test]
    async fn part_exactly_at_the_limit() {
        let (body, headers) = form(vec![TestFormField::new("file", vec![b'x'; 100])]);

        let parts = read_parts(payload(body, &headers), PartLimits::new(1, 100))
            .await
            .unwrap();

        assert_eq!(parts[0].bytes.len(), 100);
    }

    #[actix_web::test]
    async fn not_multipart() {
        let headers = HeaderMap::new();
        let error = read_parts(
            payload(Bytes::from_static(b"a=1"), &headers),
            PartLimits::new(1, 100),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, ReadPartsError::Multipart(_)), "{error:?}");
    }

    #[actix_web::test]
    async fn read_bytes_positions_and_headers() {
        let (body, headers) = form(vec![
            TestFormField::new("2", "druga"),
            TestFormField::new("empty", ""),
            TestFormField::new("name", "trzecia").filename("c.txt"),
        ]);

        let files = read_bytes(payload(body, &headers), 10, 1024).await.unwrap();

        let positions: Vec<i32> = files.iter().map(|(_, position, _)| *position).collect();
        assert_eq!(positions, [2, 2]);
        assert_eq!(&files[0].0[..], b"druga");
        assert_eq!(
            files[1].2.as_deref(),
            Some("form-data; name=\"name\"; filename=\"c.txt\"")
        );
    }

    #[actix_web::test]
    async fn read_bytes_stops_after_max_parts() {
        let (body, headers) = form(vec![
            TestFormField::new("a", "Ala"),
            TestFormField::new("b", "ma"),
            TestFormField::new("c", "kota"),
        ]);

        let files = read_bytes(payload(body, &headers), 2, 1024).await.unwrap();

        assert_eq!(files.len(), 2);
    }

    /// Over the limit `read_bytes` throws away up to 16 times `max_upload_size` more and cuts
    /// the rest off.
    #[actix_web::test]
    async fn read_bytes_overflow_drains_up_to_a_limit() {
        let (body, headers, chunks) = big_file();
        let (multipart, taken) = chunked(body, &headers, 1024);

        let error = read_bytes(multipart, 4, 10 * 1024).await.unwrap_err();

        assert!(
            matches!(error, MultipartError::Payload(PayloadError::Overflow)),
            "{error:?}"
        );
        let taken = taken.load(Ordering::Relaxed);
        assert!(
            (170..chunks).contains(&taken),
            "{taken} of {chunks} chunks taken for a 10 KiB limit with 160 KiB to drain"
        );
    }

    #[actix_web::test]
    async fn read_bytes_overflow_drains_a_small_rest() {
        let (body, headers, chunks) = big_file();
        let (multipart, taken) = chunked(body, &headers, 1024);

        let error = read_bytes(multipart, 4, 100 * 1024).await.unwrap_err();

        assert!(
            matches!(error, MultipartError::Payload(PayloadError::Overflow)),
            "{error:?}"
        );
        assert_eq!(taken.load(Ordering::Relaxed), chunks);
    }
}
