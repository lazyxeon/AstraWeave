//! `data:` URI decode with a caller-supplied byte cap.
//!
//! This file is not included by `astraweave-blend`. `image_limits.rs` is.

use anyhow::{bail, Context, Result};
use base64::Engine as _;

use crate::image_limits::MAX_IMAGE_ALLOC;

/// Decode a `data:` URI without allocating a payload over `cap`.
///
/// The base64 length is checked before `decode`. The scheme match is
/// case-insensitive, and that fold is paired with the length reject.
pub fn read_capped_data_uri(uri: &str, cap: u64) -> Result<Vec<u8>> {
    let scheme_end = uri.find(':').context("data URI missing scheme")?;
    let scheme = &uri[..scheme_end];
    if !scheme.eq_ignore_ascii_case("data") {
        bail!("not a data URI");
    }
    let rest = &uri[scheme_end + 1..];
    let payload = rest
        .split_once(',')
        .map(|(_, data)| data)
        .context("Invalid data URI")?;
    let estimate = (payload.len() as u64).saturating_mul(3) / 4;
    if estimate > cap {
        bail!("data URI is {estimate} bytes, cap is {cap}");
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .context("decode data URI")
}

/// Decode a `data:` image URI at [`MAX_IMAGE_ALLOC`].
pub fn read_capped_data_image(uri: &str) -> Result<Vec<u8>> {
    read_capped_data_uri(uri, MAX_IMAGE_ALLOC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_image_under_the_cap_decodes() {
        let bytes =
            read_capped_data_image("DATA:application/octet-stream;base64,AAAA").expect("decode");
        assert_eq!(bytes, vec![0, 0, 0]);
    }

    #[test]
    fn data_image_over_the_cap_is_rejected_before_decode() {
        let payload_len = ((MAX_IMAGE_ALLOC as usize) * 4 / 3) + 4;
        let mut uri = String::from("data:application/octet-stream;base64,");
        uri.extend(std::iter::repeat('A').take(payload_len));
        let err = read_capped_data_image(&uri).expect_err("over cap");
        let msg = format!("{err:#}");
        assert!(msg.contains(&MAX_IMAGE_ALLOC.to_string()), "{msg}");
    }

    #[test]
    fn data_uri_respects_a_caller_cap() {
        let err = read_capped_data_uri("data:application/octet-stream;base64,AAAA", 2)
            .expect_err("over the caller cap");
        assert!(format!("{err:#}").contains("cap is 2"));
    }
}
