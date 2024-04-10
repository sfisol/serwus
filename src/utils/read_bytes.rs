use actix_multipart::{Multipart, MultipartError};
use actix_web::error::PayloadError;
use bytes::{Bytes, BytesMut};
use futures::TryStreamExt;
use validator::ValidateLength;

/// Helper for reading multipart/form-data, returning vector of read bytes.
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
        let mut file = BytesMut::new();

        // Field in turn is a stream of *Bytes*
        while let Some(chunk) = field.try_next().await? {
            file.extend_from_slice(&chunk);
        }

        if file.len() > max_upload_size {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }

        if !file.is_empty() {
            let position: i32 = field
                .content_disposition()
                .get_name()
                .and_then(|r| r.parse::<i32>().ok())
                .unwrap_or(incr_default_position);

            files.push((
                file.freeze(),
                position,
                field
                    .headers()
                    .get("Content-Disposition")
                    .and_then(|v| v.to_str().ok().map(String::from)),
            ));
        }

        incr_default_position += 1; // Increment default position

        if files.length() >= Some(max_parts) {
            break;
        }
    }

    Ok(files)
}
