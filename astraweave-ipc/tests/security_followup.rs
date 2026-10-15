use astraweave_ipc::{ipc_ws_config, MAX_IPC_MESSAGE};

#[test]
fn ipc_websocket_message_cap_is_one_mib() {
    let config = ipc_ws_config();
    assert_eq!(config.max_message_size, Some(MAX_IPC_MESSAGE));
    assert_eq!(config.max_frame_size, Some(MAX_IPC_MESSAGE));
    assert_eq!(MAX_IPC_MESSAGE, 1024 * 1024);
}
