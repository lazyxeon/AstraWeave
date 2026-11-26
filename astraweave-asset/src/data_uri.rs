//! `data:` URI decode with a caller-supplied byte cap.
//!
//! This file is not included by `astraweave-blend`. `image_limits.rs` is.

use anyhow::{bail, Context, Result};
use base64::Engine as _;

use crate::image_limits::MAX_IMAGE_ALLOC;

/// One glTF mesh buffer, BIN chunk, or `data:` buffer.
/// Four times the image cap. A mesh can be larger than 64 MiB. It cannot be unbounded.
pub const MAX_GLTF_BUFFER: u64 = 256 * 1024 * 1024;

/// Percent-decode a glTF file URI and reject a scheme or a drive letter.
///
/// A scheme is a colon before the first `/` or `\`: `file:`, `http:`, `C:`.
/// A colon after that slash is a filename character (`textures/my:file.bin`).
/// `data:` URIs are handled by [`read_capped_data_uri`] before this runs.
/// A bad percent sequence is an error. Decoding happens before the path jail,
/// so `%2e%2e` is `..` when the jail runs.
pub fn gltf_relative_uri(uri: &str) -> Result<String> {
    let without_query = uri.split('?').next().unwrap_or(uri);
    let decoded = percent_decode(without_query)?;
    if colon_before_first_slash(&decoded) {
        bail!("glTF URI scheme is not a relative file");
    }
    Ok(decoded)
}

/// `true` when `:` appears before the first path separator.
fn colon_before_first_slash(uri: &str) -> bool {
    match uri.find([':', '/', '\\']) {
        Some(index) => uri.as_bytes()[index] == b':',
        None => false,
    }
}

fn percent_decode(input: &str) -> Result<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                bail!("bad percent encoding in glTF URI");
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| anyhow::anyhow!("bad percent encoding in glTF URI"))?;
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| anyhow::anyhow!("bad percent encoding in glTF URI"))?;
            out.push(value);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).context("glTF URI is not utf-8")
}

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
    let estimate = base64_decoded_len(payload);
    if estimate > cap {
        bail!("data URI is {estimate} bytes, cap is {cap}");
    }
    let mut decoded = vec![0u8; estimate as usize];
    let written = base64::engine::general_purpose::STANDARD
        .decode_slice(payload, &mut decoded)
        .context("decode data URI")?;
    decoded.truncate(written);
    Ok(decoded)
}

/// Decoded byte length of a standard base64 payload, treating `=` as padding.
///
/// `(len * 3) / 4` counts padding as payload. `TQ==` is one byte, and a
/// length that is `1` mod 3 estimates two bytes over the real decode.
fn base64_decoded_len(payload: &str) -> u64 {
    let bytes = payload.as_bytes();
    let mut padding = 0u64;
    if bytes.last() == Some(&b'=') {
        padding += 1;
        if bytes.len() >= 2 && bytes[bytes.len() - 2] == b'=' {
            padding += 1;
        }
    }
    (bytes.len() as u64 / 4)
        .saturating_mul(3)
        .saturating_sub(padding)
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
    fn relative_uri_decodes_a_space_and_rejects_a_scheme() {
        assert_eq!(
            gltf_relative_uri("textures/my%20albedo.png").expect("space"),
            "textures/my albedo.png"
        );
        assert_eq!(
            gltf_relative_uri("%2e%2e/%2e%2e/secret.bin").expect("dots"),
            "../../secret.bin"
        );
        assert_eq!(
            gltf_relative_uri("textures/my%3afile.bin").expect("colon in the name"),
            "textures/my:file.bin"
        );
        let scheme = gltf_relative_uri("file:///etc/passwd").expect_err("scheme");
        assert!(format!("{scheme:#}").contains("scheme"), "{scheme}");
        let drive = gltf_relative_uri("C:/Windows/system32").expect_err("drive");
        assert!(format!("{drive:#}").contains("scheme"), "{drive}");
        let drive_backslash = gltf_relative_uri(r"D:\Windows").expect_err("drive slash");
        assert!(
            format!("{drive_backslash:#}").contains("scheme"),
            "{drive_backslash}"
        );
        let bad = gltf_relative_uri("%zz").expect_err("percent");
        assert!(format!("{bad:#}").contains("percent"), "{bad}");
    }

    #[test]
    fn data_uri_respects_a_caller_cap() {
        let err = read_capped_data_uri("data:application/octet-stream;base64,AAAA", 2)
            .expect_err("over the caller cap");
        assert!(format!("{err:#}").contains("cap is 2"));
    }

    #[test]
    fn padded_payload_exactly_at_the_cap_decodes() {
        let one = read_capped_data_uri("data:application/octet-stream;base64,TQ==", 1)
            .expect("TQ== is one byte");
        assert_eq!(one, b"M");
        assert_eq!(one.capacity(), 1);
        let under = read_capped_data_uri("data:application/octet-stream;base64,TQ==", 0)
            .expect_err("cap 0");
        assert!(
            format!("{under:#}").contains("1 bytes") && format!("{under:#}").contains("cap is 0"),
            "{under}"
        );

        let raw = vec![0xA5u8; 4];
        let payload = base64::engine::general_purpose::STANDARD.encode(&raw);
        assert!(payload.ends_with("=="), "{payload}");
        let uri = format!("data:application/octet-stream;base64,{payload}");
        let exact = read_capped_data_uri(&uri, raw.len() as u64).expect("exact cap");
        assert_eq!(exact, raw);
        assert_eq!(exact.capacity(), raw.len());
        let inflated = (payload.len() as u64).saturating_mul(3) / 4;
        assert!(
            inflated > raw.len() as u64,
            "old estimate should exceed the decode"
        );
        let rejected = read_capped_data_uri(&uri, raw.len() as u64 - 1).expect_err("one under");
        let msg = format!("{rejected:#}");
        assert!(
            msg.contains(&raw.len().to_string()) && !msg.contains(&inflated.to_string()),
            "{msg}"
        );
    }
}
