#![no_main]

use aw_net_proto::{decode_msg, Codec, ServerToClient};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_msg::<ServerToClient>(Codec::PostcardLz4, data);
    let _ = decode_msg::<ServerToClient>(Codec::Bincode, data);
});
