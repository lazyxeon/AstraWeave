use astraweave_net::{engine_ws_config, MAX_ENGINE_WS_MESSAGE};

#[test]
fn game_server_websocket_message_cap_is_one_mib() {
    let config = engine_ws_config();
    assert_eq!(config.max_message_size, Some(MAX_ENGINE_WS_MESSAGE));
    assert_eq!(config.max_frame_size, Some(MAX_ENGINE_WS_MESSAGE));
    assert_eq!(MAX_ENGINE_WS_MESSAGE, 1024 * 1024);
}
