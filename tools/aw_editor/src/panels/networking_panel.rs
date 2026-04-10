//! Networking Panel for the editor UI
//!
//! Provides a comprehensive GUI for configuring and monitoring multiplayer:
//! - Server/client configuration
//! - Connection management
//! - Snapshot and delta compression settings
//! - Interest management policies
//! - Network statistics and debugging
//! - Lag simulation for testing

use egui::{Color32, RichText, Ui, Vec2};
use std::collections::VecDeque;
use tracing::info;

use crate::panels::Panel;

/// Network role (server, client, or offline)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NetworkRole {
    #[default]
    Offline,
    Server,
    Client,
    ListenServer, // Server that also plays
}

impl std::fmt::Display for NetworkRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl NetworkRole {
    pub fn all() -> &'static [NetworkRole] {
        &[
            NetworkRole::Offline,
            NetworkRole::Server,
            NetworkRole::Client,
            NetworkRole::ListenServer,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            NetworkRole::Offline => "Offline",
            NetworkRole::Server => "Server",
            NetworkRole::Client => "Client",
            NetworkRole::ListenServer => "Listen Server",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            NetworkRole::Offline => "📴",
            NetworkRole::Server => "[Mon]",
            NetworkRole::Client => "[PC]",
            NetworkRole::ListenServer => "[Gp]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NetworkRole::Offline => "No networking, single-player only",
            NetworkRole::Server => "Dedicated server, no local player",
            NetworkRole::Client => "Connect to remote server",
            NetworkRole::ListenServer => "Host and play locally",
        }
    }

    pub fn is_server(&self) -> bool {
        matches!(self, NetworkRole::Server | NetworkRole::ListenServer)
    }

    pub fn is_online(&self) -> bool {
        !matches!(self, NetworkRole::Offline)
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ConnectionState {
    pub fn all() -> &'static [ConnectionState] {
        &[
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Error,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting",
            ConnectionState::Connected => "Connected",
            ConnectionState::Reconnecting => "Reconnecting",
            ConnectionState::Error => "Error",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            ConnectionState::Disconnected => Color32::GRAY,
            ConnectionState::Connecting => Color32::YELLOW,
            ConnectionState::Connected => Color32::GREEN,
            ConnectionState::Reconnecting => Color32::from_rgb(255, 165, 0), // Orange
            ConnectionState::Error => Color32::RED,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "⚫",
            ConnectionState::Connecting => "[Sync]",
            ConnectionState::Connected => "[G]",
            ConnectionState::Reconnecting => "[Y]",
            ConnectionState::Error => "[R]",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ConnectionState::Connecting
                | ConnectionState::Connected
                | ConnectionState::Reconnecting
        )
    }

    pub fn is_stable(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }
}

/// Interest management policy for entity replication
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InterestPolicy {
    #[default]
    Full,
    Radius,
    FieldOfView,
    FieldOfViewWithLOS,
    Custom,
}

impl std::fmt::Display for InterestPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl InterestPolicy {
    pub fn all() -> &'static [InterestPolicy] {
        &[
            InterestPolicy::Full,
            InterestPolicy::Radius,
            InterestPolicy::FieldOfView,
            InterestPolicy::FieldOfViewWithLOS,
            InterestPolicy::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            InterestPolicy::Full => "Full",
            InterestPolicy::Radius => "Radius",
            InterestPolicy::FieldOfView => "Field of View",
            InterestPolicy::FieldOfViewWithLOS => "FOV + LOS",
            InterestPolicy::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            InterestPolicy::Full => "[Net]",
            InterestPolicy::Radius => "⭕",
            InterestPolicy::FieldOfView => "👁️",
            InterestPolicy::FieldOfViewWithLOS => "[Srch]",
            InterestPolicy::Custom => "⚙️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            InterestPolicy::Full => "Send all entities to all clients",
            InterestPolicy::Radius => "Filter by distance from player",
            InterestPolicy::FieldOfView => "Filter by player's view cone",
            InterestPolicy::FieldOfViewWithLOS => "FOV + line-of-sight check",
            InterestPolicy::Custom => "Custom scripted interest logic",
        }
    }

    pub fn is_spatial(&self) -> bool {
        matches!(
            self,
            InterestPolicy::Radius
                | InterestPolicy::FieldOfView
                | InterestPolicy::FieldOfViewWithLOS
        )
    }

    pub fn has_visibility_check(&self) -> bool {
        matches!(
            self,
            InterestPolicy::FieldOfView | InterestPolicy::FieldOfViewWithLOS
        )
    }
}

/// Compression level for network data
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompressionLevel {
    None,
    #[default]
    Fast,
    Balanced,
    Maximum,
}

impl std::fmt::Display for CompressionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl CompressionLevel {
    pub fn all() -> &'static [CompressionLevel] {
        &[
            CompressionLevel::None,
            CompressionLevel::Fast,
            CompressionLevel::Balanced,
            CompressionLevel::Maximum,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            CompressionLevel::None => "None",
            CompressionLevel::Fast => "Fast",
            CompressionLevel::Balanced => "Balanced",
            CompressionLevel::Maximum => "Maximum",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CompressionLevel::None => "No compression (fastest, largest)",
            CompressionLevel::Fast => "Fast compression (good speed/size)",
            CompressionLevel::Balanced => "Balanced compression (default)",
            CompressionLevel::Maximum => "Maximum compression (slowest, smallest)",
        }
    }

    pub fn cpu_cost(&self) -> u8 {
        match self {
            CompressionLevel::None => 0,
            CompressionLevel::Fast => 1,
            CompressionLevel::Balanced => 2,
            CompressionLevel::Maximum => 4,
        }
    }
}

/// Connected client information
#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
    pub id: u64,
    pub name: String,
    pub address: String,
    pub ping_ms: u32,
    pub packet_loss_percent: f32,
    pub state: ConnectionState,
    pub player_entity_id: Option<u32>,
    pub last_input_tick: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub bytes_sent_per_sec: f32,
    pub bytes_received_per_sec: f32,
    pub packets_sent_per_sec: f32,
    pub packets_received_per_sec: f32,
    pub current_tick: u64,
    pub snapshot_size_bytes: usize,
    pub delta_size_bytes: usize,
    pub compression_ratio: f32,
    pub avg_ping_ms: f32,
    pub jitter_ms: f32,
    pub packet_loss_percent: f32,
    pub entities_replicated: usize,
    pub entities_filtered: usize,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NetworkTab {
    #[default]
    Connection,
    Clients,
    Replication,
    Statistics,
    Debug,
}

impl std::fmt::Display for NetworkTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl NetworkTab {
    pub fn all() -> &'static [NetworkTab] {
        &[
            NetworkTab::Connection,
            NetworkTab::Clients,
            NetworkTab::Replication,
            NetworkTab::Statistics,
            NetworkTab::Debug,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            NetworkTab::Connection => "Connection",
            NetworkTab::Clients => "Clients",
            NetworkTab::Replication => "Replication",
            NetworkTab::Statistics => "Statistics",
            NetworkTab::Debug => "Debug",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            NetworkTab::Connection => "🔌",
            NetworkTab::Clients => "[Grp]",
            NetworkTab::Replication => "[Sync]",
            NetworkTab::Statistics => "[Chart]",
            NetworkTab::Debug => "🐛",
        }
    }
}

/// Actions that can be performed on the networking panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NetworkAction {
    /// Start server on specified port
    StartServer { port: u16 },
    /// Stop the server
    StopServer,
    /// Connect to a remote server
    Connect { address: String, port: u16 },
    /// Disconnect from current server
    Disconnect,
    /// Kick a client from the server
    KickClient { client_id: u64, reason: String },
    /// Ban a client from the server
    BanClient {
        client_id: u64,
        duration_seconds: Option<u64>,
    },
    /// Set the network role
    SetRole(NetworkRole),
    /// Set the interest management policy
    SetInterestPolicy(InterestPolicy),
    /// Set the compression level
    SetCompressionLevel(CompressionLevel),
    /// Set the tick rate
    SetTickRate(u32),
    /// Set the snapshot rate
    SetSnapshotRate(u32),
    /// Toggle lag simulation
    ToggleLagSimulation(bool),
    /// Set lag simulation latency
    SetSimulatedLatency(u32),
    /// Set lag simulation packet loss
    SetSimulatedPacketLoss(f32),
    /// Toggle network overlay display
    ToggleNetworkOverlay(bool),
    /// Toggle packet logging
    TogglePacketLogging(bool),
    /// Clear packet log
    ClearPacketLog,
    /// Send test packet to validate connection
    SendTestPacket,
    /// Request server statistics refresh
    RefreshStats,
}

/// Lag simulation settings for testing
#[derive(Debug, Clone, Default)]
pub struct LagSimulationSettings {
    pub enabled: bool,
    pub latency_ms: u32,
    pub jitter_ms: u32,
    pub packet_loss_percent: f32,
    pub duplicate_percent: f32,
    pub out_of_order_percent: f32,
}

/// Main Networking Panel
pub struct NetworkingPanel {
    // Tab state
    active_tab: NetworkTab,

    // Connection settings
    role: NetworkRole,
    connection_state: ConnectionState,
    server_address: String,
    server_port: u16,
    max_clients: u32,
    password: String,
    use_tls: bool,

    // Replication settings
    tick_rate: u32,
    snapshot_rate: u32,
    interest_policy: InterestPolicy,
    interest_radius: f32,
    interest_fov_angle: f32,
    delta_compression: bool,
    compression_level: CompressionLevel,
    interpolation_delay: f32,
    prediction_enabled: bool,
    reconciliation_enabled: bool,

    // Connected clients (for server)
    clients: Vec<ClientInfo>,
    selected_client_id: Option<u64>,

    // Statistics
    stats: NetworkStats,
    bandwidth_history: VecDeque<(f32, f32)>, // (sent, received) samples
    ping_history: VecDeque<f32>,

    // Debug / Lag simulation
    lag_sim: LagSimulationSettings,
    show_network_overlay: bool,
    log_packets: bool,
    packet_log: VecDeque<String>,

    // Status
    error_message: Option<String>,
    uptime_seconds: f32,

    // Action queue
    pending_actions: Vec<NetworkAction>,
}

impl Default for NetworkingPanel {
    fn default() -> Self {
        Self {
            active_tab: NetworkTab::Connection,

            role: NetworkRole::Offline,
            connection_state: ConnectionState::Disconnected,
            server_address: "127.0.0.1".to_string(),
            server_port: 7777,
            max_clients: 16,
            password: String::new(),
            use_tls: false,

            tick_rate: 60,
            snapshot_rate: 20,
            interest_policy: InterestPolicy::Radius,
            interest_radius: 50.0,
            interest_fov_angle: 90.0,
            delta_compression: true,
            compression_level: CompressionLevel::Fast,
            interpolation_delay: 100.0,
            prediction_enabled: true,
            reconciliation_enabled: true,

            clients: Vec::new(),
            selected_client_id: None,

            stats: NetworkStats::default(),
            bandwidth_history: VecDeque::with_capacity(120),
            ping_history: VecDeque::with_capacity(120),

            lag_sim: LagSimulationSettings::default(),
            show_network_overlay: false,
            log_packets: false,
            packet_log: VecDeque::with_capacity(100),

            error_message: None,
            uptime_seconds: 0.0,

            pending_actions: Vec::new(),
        }
    }
}

impl NetworkingPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions, leaving the queue empty
    pub fn take_actions(&mut self) -> Vec<NetworkAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: NetworkAction) {
        self.pending_actions.push(action);
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (NetworkTab::Connection, "🔌 Connection"),
                (NetworkTab::Clients, "[Grp] Clients"),
                (NetworkTab::Replication, "Replication"),
                (NetworkTab::Statistics, "Statistics"),
                (NetworkTab::Debug, "🐛 Debug"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;
                let button = egui::Button::new(label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // Connection status indicator
        ui.horizontal(|ui| {
            ui.label(self.connection_state.icon());
            ui.label(format!("{:?}", self.connection_state));
            if self.connection_state == ConnectionState::Connected {
                ui.label(format!("| {} clients", self.clients.len()));
                ui.label(format!("| Tick {}", self.stats.current_tick));
            }
        });

        ui.separator();
    }

    fn show_connection_tab(&mut self, ui: &mut Ui) {
        ui.heading("🔌 Network Connection");
        ui.add_space(10.0);

        // Role selection
        ui.group(|ui| {
            ui.label(RichText::new("Network Role").strong());
            ui.horizontal_wrapped(|ui| {
                for role in NetworkRole::all() {
                    let is_selected = self.role == *role;
                    let button_text = format!("{} {:?}", role.icon(), role);
                    let button = egui::Button::new(button_text).fill(if is_selected {
                        Color32::from_rgb(60, 100, 160)
                    } else {
                        Color32::from_rgb(50, 50, 55)
                    });

                    if ui.add(button).clicked() {
                        self.role = *role;
                    }
                }
            });
            ui.label(self.role.description());
        });

        ui.add_space(10.0);

        // Connection settings based on role
        match self.role {
            NetworkRole::Offline => {
                ui.label("Networking disabled. Switch role to enable multiplayer.");
            }
            NetworkRole::Client => {
                self.show_client_settings(ui);
            }
            NetworkRole::Server | NetworkRole::ListenServer => {
                self.show_server_settings(ui);
            }
        }

        ui.add_space(10.0);

        // Connect/Disconnect buttons
        if self.role != NetworkRole::Offline {
            ui.horizontal(|ui| match self.connection_state {
                ConnectionState::Disconnected | ConnectionState::Error => {
                    if ui.button("> Start").clicked() {
                        self.start_networking();
                    }
                }
                ConnectionState::Connecting | ConnectionState::Reconnecting => {
                    if ui.button("[] Cancel").clicked() {
                        self.stop_networking();
                    }
                    ui.spinner();
                }
                ConnectionState::Connected => {
                    if ui.button("[] Stop").clicked() {
                        self.stop_networking();
                    }
                    ui.label(format!("Uptime: {:.0}s", self.uptime_seconds));
                }
            });
        }

        // Error display
        if let Some(ref error) = self.error_message {
            ui.add_space(10.0);
            ui.colored_label(Color32::RED, format!("Error: {}", error));
        }
    }

    fn show_client_settings(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label(RichText::new("Server Connection").strong());

            egui::Grid::new("client_settings_grid")
                .num_columns(2)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Server Address:");
                    ui.text_edit_singleline(&mut self.server_address);
                    ui.end_row();

                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut self.server_port).range(1..=65535));
                    ui.end_row();

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                    ui.end_row();

                    ui.label("Use TLS:");
                    ui.checkbox(&mut self.use_tls, "Encrypt connection");
                    ui.end_row();
                });
        });
    }

    fn show_server_settings(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label(RichText::new("Server Settings").strong());

            egui::Grid::new("server_settings_grid")
                .num_columns(2)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Listen Port:");
                    ui.add(egui::DragValue::new(&mut self.server_port).range(1..=65535));
                    ui.end_row();

                    ui.label("Max Clients:");
                    ui.add(egui::DragValue::new(&mut self.max_clients).range(1..=64));
                    ui.end_row();

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                    ui.end_row();

                    ui.label("Use TLS:");
                    ui.checkbox(&mut self.use_tls, "Require encrypted connections");
                    ui.end_row();

                    ui.label("Tick Rate:");
                    ui.add(
                        egui::DragValue::new(&mut self.tick_rate)
                            .range(20..=128)
                            .suffix(" Hz"),
                    );
                    ui.end_row();

                    ui.label("Snapshot Rate:");
                    ui.add(
                        egui::DragValue::new(&mut self.snapshot_rate)
                            .range(10..=60)
                            .suffix(" Hz"),
                    );
                    ui.end_row();
                });
        });
    }

    fn show_clients_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Grp] Connected Clients");
        ui.add_space(10.0);

        if self.role == NetworkRole::Client {
            ui.label("Client mode - showing server connection only.");
            ui.add_space(10.0);

            // Show own connection to server
            ui.group(|ui| {
                ui.label(RichText::new("Connection to Server").strong());
                egui::Grid::new("own_connection_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Server:");
                        ui.label(format!("{}:{}", self.server_address, self.server_port));
                        ui.end_row();

                        ui.label("State:");
                        ui.colored_label(
                            self.connection_state.color(),
                            format!("{:?}", self.connection_state),
                        );
                        ui.end_row();

                        ui.label("Ping:");
                        ui.label(format!("{:.0} ms", self.stats.avg_ping_ms));
                        ui.end_row();

                        ui.label("Packet Loss:");
                        ui.label(format!("{:.1}%", self.stats.packet_loss_percent));
                        ui.end_row();
                    });
            });
            return;
        }

        // Server view - client list
        ui.horizontal(|ui| {
            ui.label(format!(
                "{}/{} clients connected",
                self.clients.len(),
                self.max_clients
            ));
            if ui.button("Refresh").clicked() {
                // Refresh client list
            }
        });

        ui.add_space(10.0);

        // Client list
        ui.group(|ui| {
            ui.label(RichText::new("Client List").strong());
            ui.separator();

            if self.clients.is_empty() {
                ui.label("No clients connected.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for client in &self.clients {
                            let is_selected = self.selected_client_id == Some(client.id);
                            ui.horizontal(|ui| {
                                ui.label(client.state.icon());
                                if ui.selectable_label(is_selected, &client.name).clicked() {
                                    self.selected_client_id = Some(client.id);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(format!("{} ms", client.ping_ms));
                                    },
                                );
                            });
                        }
                    });
            }
        });

        ui.add_space(10.0);

        // Selected client details
        if let Some(client_id) = self.selected_client_id {
            if let Some(client) = self.clients.iter().find(|c| c.id == client_id) {
                ui.group(|ui| {
                    ui.label(RichText::new("Client Details").strong());
                    ui.separator();

                    egui::Grid::new("client_details_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("ID:");
                            ui.label(format!("{}", client.id));
                            ui.end_row();

                            ui.label("Name:");
                            ui.label(&client.name);
                            ui.end_row();

                            ui.label("Address:");
                            ui.label(&client.address);
                            ui.end_row();

                            ui.label("State:");
                            ui.colored_label(client.state.color(), format!("{:?}", client.state));
                            ui.end_row();

                            ui.label("Ping:");
                            ui.label(format!("{} ms", client.ping_ms));
                            ui.end_row();

                            ui.label("Packet Loss:");
                            ui.label(format!("{:.1}%", client.packet_loss_percent));
                            ui.end_row();

                            ui.label("Bytes Sent:");
                            ui.label(format_bytes(client.bytes_sent));
                            ui.end_row();

                            ui.label("Bytes Received:");
                            ui.label(format_bytes(client.bytes_received));
                            ui.end_row();

                            if let Some(entity_id) = client.player_entity_id {
                                ui.label("Player Entity:");
                                ui.label(format!("#{}", entity_id));
                                ui.end_row();
                            }
                        });

                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        if ui.button("👢 Kick").clicked() {
                            // Kick client
                        }
                        if ui.button("🚫 Ban").clicked() {
                            // Ban client
                        }
                    });
                });
            }
        }
    }

    fn show_replication_tab(&mut self, ui: &mut Ui) {
        ui.heading("Entity Replication");
        ui.add_space(10.0);

        // Interest policy
        ui.group(|ui| {
            ui.label(RichText::new("Interest Management").strong());
            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                for policy in InterestPolicy::all() {
                    if ui
                        .selectable_label(self.interest_policy == *policy, format!("{:?}", policy))
                        .clicked()
                    {
                        info!(policy = ?policy, "Network: replication interest policy changed");
                        self.interest_policy = *policy;
                    }
                }
            });

            ui.label(self.interest_policy.description());

            ui.add_space(5.0);

            // Policy-specific settings
            match self.interest_policy {
                InterestPolicy::Radius => {
                    ui.add(
                        egui::Slider::new(&mut self.interest_radius, 10.0..=200.0)
                            .text("Radius")
                            .suffix(" units"),
                    );
                }
                InterestPolicy::FieldOfView | InterestPolicy::FieldOfViewWithLOS => {
                    ui.add(
                        egui::Slider::new(&mut self.interest_radius, 10.0..=200.0)
                            .text("View Distance")
                            .suffix(" units"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.interest_fov_angle, 30.0..=180.0)
                            .text("FOV Angle")
                            .suffix("°"),
                    );
                }
                _ => {}
            }

            ui.add_space(5.0);

            // Stats
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Entities: {} replicated / {} filtered",
                    self.stats.entities_replicated, self.stats.entities_filtered
                ));
            });
        });

        ui.add_space(10.0);

        // Compression settings
        ui.group(|ui| {
            ui.label(RichText::new("Delta Compression").strong());

            ui.checkbox(&mut self.delta_compression, "Enable delta compression");

            if self.delta_compression {
                ui.horizontal(|ui| {
                    ui.label("Level:");
                    for level in CompressionLevel::all() {
                        if ui
                            .selectable_label(
                                self.compression_level == *level,
                                format!("{:?}", level),
                            )
                            .clicked()
                        {
                            self.compression_level = *level;
                        }
                    }
                });

                ui.add_space(5.0);

                // Compression stats
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Full snapshot: {} | Delta: {} | Ratio: {:.1}%",
                        format_bytes(self.stats.snapshot_size_bytes as u64),
                        format_bytes(self.stats.delta_size_bytes as u64),
                        self.stats.compression_ratio * 100.0
                    ));
                });
            }
        });

        ui.add_space(10.0);

        // Client-side prediction
        ui.group(|ui| {
            ui.label(RichText::new("Prediction & Reconciliation").strong());

            ui.checkbox(&mut self.prediction_enabled, "Client-side prediction");
            ui.checkbox(&mut self.reconciliation_enabled, "Server reconciliation");
            ui.add(
                egui::Slider::new(&mut self.interpolation_delay, 50.0..=300.0)
                    .text("Interpolation delay")
                    .suffix(" ms"),
            );
        });
    }

    fn show_statistics_tab(&mut self, ui: &mut Ui) {
        ui.heading("Network Statistics");
        ui.add_space(10.0);

        // Bandwidth stats
        ui.group(|ui| {
            ui.label(RichText::new("Bandwidth").strong());

            egui::Grid::new("bandwidth_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Upload:");
                    ui.label(format!(
                        "{}/s ({:.0} pkt/s)",
                        format_bytes_per_sec(self.stats.bytes_sent_per_sec),
                        self.stats.packets_sent_per_sec
                    ));
                    ui.end_row();

                    ui.label("Download:");
                    ui.label(format!(
                        "{}/s ({:.0} pkt/s)",
                        format_bytes_per_sec(self.stats.bytes_received_per_sec),
                        self.stats.packets_received_per_sec
                    ));
                    ui.end_row();
                });

            // Bandwidth graph
            ui.add_space(5.0);
            self.draw_bandwidth_graph(ui);
        });

        ui.add_space(10.0);

        // Latency stats
        ui.group(|ui| {
            ui.label(RichText::new("Latency").strong());

            egui::Grid::new("latency_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Average Ping:");
                    ui.label(format!("{:.0} ms", self.stats.avg_ping_ms));
                    ui.end_row();

                    ui.label("Jitter:");
                    ui.label(format!("{:.0} ms", self.stats.jitter_ms));
                    ui.end_row();

                    ui.label("Packet Loss:");
                    let loss_color = if self.stats.packet_loss_percent > 5.0 {
                        Color32::RED
                    } else if self.stats.packet_loss_percent > 1.0 {
                        Color32::YELLOW
                    } else {
                        Color32::GREEN
                    };
                    ui.colored_label(
                        loss_color,
                        format!("{:.1}%", self.stats.packet_loss_percent),
                    );
                    ui.end_row();
                });

            // Ping graph
            ui.add_space(5.0);
            self.draw_ping_graph(ui);
        });

        ui.add_space(10.0);

        // Tick info
        ui.group(|ui| {
            ui.label(RichText::new("Simulation").strong());

            egui::Grid::new("simulation_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Current Tick:");
                    ui.label(format!("{}", self.stats.current_tick));
                    ui.end_row();

                    ui.label("Tick Rate:");
                    ui.label(format!("{} Hz", self.tick_rate));
                    ui.end_row();

                    ui.label("Snapshot Rate:");
                    ui.label(format!("{} Hz", self.snapshot_rate));
                    ui.end_row();
                });
        });
    }

    fn show_debug_tab(&mut self, ui: &mut Ui) {
        ui.heading("🐛 Network Debug");
        ui.add_space(10.0);

        // Lag simulation
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Lag Simulation").strong());
                ui.checkbox(&mut self.lag_sim.enabled, "Enable");
            });

            if self.lag_sim.enabled {
                ui.add_space(5.0);

                ui.add(
                    egui::Slider::new(&mut self.lag_sim.latency_ms, 0..=500)
                        .text("Added Latency")
                        .suffix(" ms"),
                );
                ui.add(
                    egui::Slider::new(&mut self.lag_sim.jitter_ms, 0..=100)
                        .text("Jitter")
                        .suffix(" ms"),
                );
                ui.add(
                    egui::Slider::new(&mut self.lag_sim.packet_loss_percent, 0.0..=50.0)
                        .text("Packet Loss")
                        .suffix("%"),
                );
                ui.add(
                    egui::Slider::new(&mut self.lag_sim.duplicate_percent, 0.0..=20.0)
                        .text("Duplicate")
                        .suffix("%"),
                );
                ui.add(
                    egui::Slider::new(&mut self.lag_sim.out_of_order_percent, 0.0..=20.0)
                        .text("Out of Order")
                        .suffix("%"),
                );

                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if ui.button("Home (0ms)").clicked() {
                        self.lag_sim.latency_ms = 0;
                        self.lag_sim.jitter_ms = 0;
                        self.lag_sim.packet_loss_percent = 0.0;
                    }
                    if ui.button("Broadband (20ms)").clicked() {
                        self.lag_sim.latency_ms = 20;
                        self.lag_sim.jitter_ms = 5;
                        self.lag_sim.packet_loss_percent = 0.5;
                    }
                    if ui.button("📱 4G Mobile (80ms)").clicked() {
                        self.lag_sim.latency_ms = 80;
                        self.lag_sim.jitter_ms = 30;
                        self.lag_sim.packet_loss_percent = 2.0;
                    }
                    if ui.button("🛰️ Satellite (600ms)").clicked() {
                        self.lag_sim.latency_ms = 600;
                        self.lag_sim.jitter_ms = 50;
                        self.lag_sim.packet_loss_percent = 1.0;
                    }
                });
            }
        });

        ui.add_space(10.0);

        // Debug options
        ui.group(|ui| {
            ui.label(RichText::new("Debug Options").strong());

            ui.checkbox(
                &mut self.show_network_overlay,
                "Show network overlay in viewport",
            );
            ui.checkbox(&mut self.log_packets, "Log packets to console");
        });

        ui.add_space(10.0);

        // Packet log
        if self.log_packets {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Packet Log").strong());
                    if ui.button("Clear").clicked() {
                        self.packet_log.clear();
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for entry in &self.packet_log {
                            ui.label(entry);
                        }
                        if self.packet_log.is_empty() {
                            ui.label("No packets logged yet.");
                        }
                    });
            });
        }

        ui.add_space(10.0);

        // Quick actions
        ui.group(|ui| {
            ui.label(RichText::new("Quick Actions").strong());

            ui.horizontal_wrapped(|ui| {
                if ui.button("📤 Force Sync").clicked() {
                    // Force full snapshot sync
                }
                if ui.button("Reconnect All").clicked() {
                    // Reconnect all clients
                }
                if ui.button("Export Stats").clicked() {
                    // Export statistics to file
                }
                if ui.button("[Test] Send Test Packet").clicked() {
                    // Send test packet
                }
            });
        });
    }

    fn draw_bandwidth_graph(&self, ui: &mut Ui) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 60.0), egui::Sense::hover());

        ui.painter()
            .rect_filled(rect, 2.0, Color32::from_rgb(30, 30, 35));

        if self.bandwidth_history.len() < 2 {
            return;
        }

        let max_value = self
            .bandwidth_history
            .iter()
            .flat_map(|(s, r)| [*s, *r])
            .fold(1.0f32, |a, b| a.max(b));

        // Draw sent (green) and received (blue)
        let step = rect.width() / self.bandwidth_history.len().max(1) as f32;

        for (i, (sent, recv)) in self.bandwidth_history.iter().enumerate() {
            let x = rect.left() + i as f32 * step;

            // Sent (green bar)
            let sent_height = (sent / max_value) * rect.height() * 0.9;
            let sent_rect = egui::Rect::from_min_size(
                egui::Pos2::new(x, rect.bottom() - sent_height),
                Vec2::new(step * 0.4, sent_height),
            );
            ui.painter()
                .rect_filled(sent_rect, 0.0, Color32::from_rgb(80, 200, 120));

            // Received (blue bar)
            let recv_height = (recv / max_value) * rect.height() * 0.9;
            let recv_rect = egui::Rect::from_min_size(
                egui::Pos2::new(x + step * 0.5, rect.bottom() - recv_height),
                Vec2::new(step * 0.4, recv_height),
            );
            ui.painter()
                .rect_filled(recv_rect, 0.0, Color32::from_rgb(100, 150, 255));
        }
    }

    fn draw_ping_graph(&self, ui: &mut Ui) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), egui::Sense::hover());

        ui.painter()
            .rect_filled(rect, 2.0, Color32::from_rgb(30, 30, 35));

        if self.ping_history.len() < 2 {
            return;
        }

        let max_ping = self.ping_history.iter().cloned().fold(50.0f32, f32::max);
        let points: Vec<egui::Pos2> = self
            .ping_history
            .iter()
            .enumerate()
            .map(|(i, &ping)| {
                let x = rect.left() + (i as f32 / self.ping_history.len() as f32) * rect.width();
                let y = rect.bottom() - (ping / max_ping) * rect.height() * 0.9;
                egui::Pos2::new(x, y)
            })
            .collect();

        if points.len() >= 2 {
            ui.painter().add(egui::Shape::line(
                points,
                egui::Stroke::new(1.5, Color32::from_rgb(255, 200, 100)),
            ));
        }
    }

    fn start_networking(&mut self) {
        info!(role = ?self.role, "Network: connecting");
        self.connection_state = ConnectionState::Connecting;
        self.error_message = None;
        // In production, this would actually start the network stack
        // For now, simulate connection after a short delay
        self.connection_state = ConnectionState::Connected;
        info!("Network: connected");
        self.uptime_seconds = 0.0;
    }

    fn stop_networking(&mut self) {
        info!("Network: disconnecting");
        self.connection_state = ConnectionState::Disconnected;
        self.clients.clear();
        self.uptime_seconds = 0.0;
    }

    // Getters for testing
    pub fn role(&self) -> NetworkRole {
        self.role
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.connection_state
    }

    pub fn interest_policy(&self) -> InterestPolicy {
        self.interest_policy
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    pub fn server_port(&self) -> u16 {
        self.server_port
    }

    pub fn set_role(&mut self, role: NetworkRole) {
        self.role = role;
    }

    pub fn set_interest_policy(&mut self, policy: InterestPolicy) {
        self.interest_policy = policy;
    }

    pub fn add_client(&mut self, client: ClientInfo) {
        self.clients.push(client);
    }

    pub fn is_lag_simulation_enabled(&self) -> bool {
        self.lag_sim.enabled
    }

    pub fn set_lag_simulation(&mut self, enabled: bool, latency_ms: u32) {
        self.lag_sim.enabled = enabled;
        self.lag_sim.latency_ms = latency_ms;
    }
}

impl Panel for NetworkingPanel {
    fn name(&self) -> &'static str {
        "Networking"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            NetworkTab::Connection => self.show_connection_tab(ui),
            NetworkTab::Clients => self.show_clients_tab(ui),
            NetworkTab::Replication => self.show_replication_tab(ui),
            NetworkTab::Statistics => self.show_statistics_tab(ui),
            NetworkTab::Debug => self.show_debug_tab(ui),
        }
    }

    fn update(&mut self) {
        // Update uptime
        if self.connection_state == ConnectionState::Connected {
            self.uptime_seconds += 1.0 / 60.0; // Assuming 60 FPS
        }

        // Sample bandwidth history
        if self.bandwidth_history.len() >= 120 {
            self.bandwidth_history.pop_front();
        }
        self.bandwidth_history.push_back((
            self.stats.bytes_sent_per_sec,
            self.stats.bytes_received_per_sec,
        ));

        // Sample ping history
        if self.ping_history.len() >= 120 {
            self.ping_history.pop_front();
        }
        self.ping_history.push_back(self.stats.avg_ping_ms);
    }
}

// Helper functions
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_bytes_per_sec(bytes_per_sec: f32) -> String {
    if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1} MB", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.1} KB", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0} B", bytes_per_sec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // NETWORK ROLE TESTS
    // ============================================================

    #[test]
    fn test_network_role_default() {
        let role = NetworkRole::default();
        assert_eq!(role, NetworkRole::Offline);
    }

    #[test]
    fn test_network_role_all() {
        let all = NetworkRole::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_network_role_icons() {
        assert_eq!(NetworkRole::Offline.icon(), "📴");
        assert_eq!(NetworkRole::Server.icon(), "[Mon]");
        assert_eq!(NetworkRole::Client.icon(), "[PC]");
        assert_eq!(NetworkRole::ListenServer.icon(), "[Gp]");
    }

    #[test]
    fn test_network_role_descriptions() {
        assert!(!NetworkRole::Offline.description().is_empty());
        assert!(!NetworkRole::Server.description().is_empty());
        assert!(!NetworkRole::Client.description().is_empty());
        assert!(!NetworkRole::ListenServer.description().is_empty());
    }

    #[test]
    fn test_network_role_all_coverage() {
        let all = NetworkRole::all();
        assert!(all.contains(&NetworkRole::Offline));
        assert!(all.contains(&NetworkRole::Server));
        assert!(all.contains(&NetworkRole::Client));
        assert!(all.contains(&NetworkRole::ListenServer));
    }

    // ============================================================
    // CONNECTION STATE TESTS
    // ============================================================

    #[test]
    fn test_connection_state_default() {
        let state = ConnectionState::default();
        assert_eq!(state, ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_colors() {
        assert_eq!(ConnectionState::Disconnected.color(), Color32::GRAY);
        assert_eq!(ConnectionState::Connecting.color(), Color32::YELLOW);
        assert_eq!(ConnectionState::Connected.color(), Color32::GREEN);
        assert_eq!(ConnectionState::Error.color(), Color32::RED);
    }

    #[test]
    fn test_connection_state_icons() {
        assert_eq!(ConnectionState::Disconnected.icon(), "⚫");
        assert_eq!(ConnectionState::Connecting.icon(), "[Sync]");
        assert_eq!(ConnectionState::Connected.icon(), "[G]");
        assert_eq!(ConnectionState::Reconnecting.icon(), "[Y]");
        assert_eq!(ConnectionState::Error.icon(), "[R]");
    }

    #[test]
    fn test_connection_state_all_have_colors() {
        let states = [
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Error,
        ];
        for state in states {
            let color = state.color();
            assert!(color.r() > 0 || color.g() > 0 || color.b() > 0 || color == Color32::GRAY);
        }
    }

    // ============================================================
    // INTEREST POLICY TESTS
    // ============================================================

    #[test]
    fn test_interest_policy_default() {
        let policy = InterestPolicy::default();
        assert_eq!(policy, InterestPolicy::Full);
    }

    #[test]
    fn test_interest_policy_all() {
        let all = InterestPolicy::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_interest_policy_descriptions() {
        for policy in InterestPolicy::all() {
            assert!(!policy.description().is_empty());
        }
    }

    #[test]
    fn test_interest_policy_all_coverage() {
        let all = InterestPolicy::all();
        assert!(all.contains(&InterestPolicy::Full));
        assert!(all.contains(&InterestPolicy::Radius));
        assert!(all.contains(&InterestPolicy::FieldOfView));
        assert!(all.contains(&InterestPolicy::FieldOfViewWithLOS));
        assert!(all.contains(&InterestPolicy::Custom));
    }

    // ============================================================
    // COMPRESSION LEVEL TESTS
    // ============================================================

    #[test]
    fn test_compression_level_default() {
        let level = CompressionLevel::default();
        assert_eq!(level, CompressionLevel::Fast);
    }

    #[test]
    fn test_compression_level_all() {
        let all = CompressionLevel::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_compression_level_all_coverage() {
        let all = CompressionLevel::all();
        assert!(all.contains(&CompressionLevel::None));
        assert!(all.contains(&CompressionLevel::Fast));
        assert!(all.contains(&CompressionLevel::Balanced));
        assert!(all.contains(&CompressionLevel::Maximum));
    }

    // ============================================================
    // NETWORK TAB TESTS
    // ============================================================

    #[test]
    fn test_network_tab_default() {
        let tab = NetworkTab::default();
        assert_eq!(tab, NetworkTab::Connection);
    }

    #[test]
    fn test_network_tab_all_variants() {
        let variants = [
            NetworkTab::Connection,
            NetworkTab::Clients,
            NetworkTab::Replication,
            NetworkTab::Statistics,
            NetworkTab::Debug,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // CLIENT INFO TESTS
    // ============================================================

    #[test]
    fn test_client_info_default() {
        let ci = ClientInfo::default();
        assert_eq!(ci.id, 0);
        assert!(ci.name.is_empty());
        assert!(ci.address.is_empty());
        assert_eq!(ci.state, ConnectionState::Disconnected);
    }

    #[test]
    fn test_client_info_default_stats() {
        let ci = ClientInfo::default();
        assert_eq!(ci.ping_ms, 0);
        assert_eq!(ci.packet_loss_percent, 0.0);
        assert_eq!(ci.bytes_sent, 0);
        assert_eq!(ci.bytes_received, 0);
    }

    #[test]
    fn test_client_info_clone() {
        let ci = ClientInfo {
            name: "Player1".to_string(),
            id: 42,
            ..Default::default()
        };
        let cloned = ci.clone();
        assert_eq!(cloned.name, "Player1");
        assert_eq!(cloned.id, 42);
    }

    // ============================================================
    // NETWORK STATS TESTS
    // ============================================================

    #[test]
    fn test_network_stats_default() {
        let stats = NetworkStats::default();
        assert_eq!(stats.bytes_sent_per_sec, 0.0);
        assert_eq!(stats.bytes_received_per_sec, 0.0);
        assert_eq!(stats.current_tick, 0);
    }

    #[test]
    fn test_network_stats_entity_counts() {
        let stats = NetworkStats::default();
        assert_eq!(stats.entities_replicated, 0);
        assert_eq!(stats.entities_filtered, 0);
    }

    #[test]
    fn test_network_stats_compression() {
        let stats = NetworkStats::default();
        assert_eq!(stats.compression_ratio, 0.0);
        assert_eq!(stats.snapshot_size_bytes, 0);
        assert_eq!(stats.delta_size_bytes, 0);
    }

    #[test]
    fn test_network_stats_clone() {
        let stats = NetworkStats::default();
        let cloned = stats.clone();
        assert_eq!(cloned.bytes_sent_per_sec, 0.0);
    }

    // ============================================================
    // LAG SIMULATION SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_lag_simulation_default() {
        let lag = LagSimulationSettings::default();
        assert!(!lag.enabled);
        assert_eq!(lag.latency_ms, 0);
        assert_eq!(lag.jitter_ms, 0);
        assert_eq!(lag.packet_loss_percent, 0.0);
    }

    #[test]
    fn test_lag_simulation_duplication() {
        let lag = LagSimulationSettings::default();
        assert_eq!(lag.duplicate_percent, 0.0);
        assert_eq!(lag.out_of_order_percent, 0.0);
    }

    #[test]
    fn test_lag_simulation_clone() {
        let lag = LagSimulationSettings::default();
        let cloned = lag.clone();
        assert!(!cloned.enabled);
    }

    // ============================================================
    // HELPER FUNCTION TESTS
    // ============================================================

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(999), "999 B");
    }

    #[test]
    fn test_format_bytes_kilobytes() {
        assert_eq!(format_bytes(1_000), "1.0 KB");
        assert_eq!(format_bytes(1_500), "1.5 KB");
    }

    #[test]
    fn test_format_bytes_megabytes() {
        assert_eq!(format_bytes(1_000_000), "1.0 MB");
        assert_eq!(format_bytes(1_500_000), "1.5 MB");
    }

    #[test]
    fn test_format_bytes_gigabytes() {
        assert_eq!(format_bytes(1_000_000_000), "1.0 GB");
        assert_eq!(format_bytes(1_500_000_000), "1.5 GB");
    }

    // ============================================================
    // NETWORKING PANEL TESTS
    // ============================================================

    #[test]
    fn test_networking_panel_creation() {
        let panel = NetworkingPanel::new();
        assert_eq!(panel.role(), NetworkRole::Offline);
        assert_eq!(panel.connection_state(), ConnectionState::Disconnected);
        assert_eq!(panel.server_port(), 7777);
    }

    #[test]
    fn test_role_switching() {
        let mut panel = NetworkingPanel::new();

        panel.set_role(NetworkRole::Server);
        assert_eq!(panel.role(), NetworkRole::Server);

        panel.set_role(NetworkRole::Client);
        assert_eq!(panel.role(), NetworkRole::Client);
    }

    #[test]
    fn test_role_switching_all() {
        let mut panel = NetworkingPanel::new();
        for role in NetworkRole::all() {
            panel.set_role(*role);
            assert_eq!(panel.role(), *role);
        }
    }

    #[test]
    fn test_interest_policy() {
        let mut panel = NetworkingPanel::new();
        assert_eq!(panel.interest_policy(), InterestPolicy::Radius);

        panel.set_interest_policy(InterestPolicy::FieldOfViewWithLOS);
        assert_eq!(panel.interest_policy(), InterestPolicy::FieldOfViewWithLOS);
    }

    #[test]
    fn test_interest_policy_switching_all() {
        let mut panel = NetworkingPanel::new();
        for policy in InterestPolicy::all() {
            panel.set_interest_policy(*policy);
            assert_eq!(panel.interest_policy(), *policy);
        }
    }

    #[test]
    fn test_client_management() {
        let mut panel = NetworkingPanel::new();
        assert_eq!(panel.client_count(), 0);

        panel.add_client(ClientInfo {
            id: 1,
            name: "Player1".to_string(),
            address: "127.0.0.1".to_string(),
            ping_ms: 50,
            ..Default::default()
        });

        assert_eq!(panel.client_count(), 1);
    }

    #[test]
    fn test_add_multiple_clients() {
        let mut panel = NetworkingPanel::new();
        for i in 0..5 {
            panel.add_client(ClientInfo {
                id: i,
                name: format!("Player{}", i),
                address: format!("10.0.0.{}", i),
                ..Default::default()
            });
        }
        assert_eq!(panel.client_count(), 5);
    }

    #[test]
    fn test_lag_simulation() {
        let mut panel = NetworkingPanel::new();
        assert!(!panel.is_lag_simulation_enabled());

        panel.set_lag_simulation(true, 100);
        assert!(panel.is_lag_simulation_enabled());
        assert_eq!(panel.lag_sim.latency_ms, 100);
    }

    #[test]
    fn test_lag_simulation_disabled() {
        let panel = NetworkingPanel::new();
        assert!(!panel.is_lag_simulation_enabled());
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = NetworkingPanel::new();
        assert_eq!(panel.name(), "Networking");
    }

    // ============================================================
    // INTEGRATION TESTS
    // ============================================================

    #[test]
    fn test_all_roles_have_icons() {
        for role in NetworkRole::all() {
            assert!(!role.icon().is_empty());
        }
    }

    #[test]
    fn test_all_roles_have_descriptions() {
        for role in NetworkRole::all() {
            assert!(!role.description().is_empty());
        }
    }

    #[test]
    fn test_all_states_have_colors() {
        let states = [
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Error,
        ];
        for state in states {
            // All states should have non-black colors
            let color = state.color();
            assert!(color.r() > 0 || color.g() > 0 || color.b() > 0 || color == Color32::GRAY);
        }
    }

    #[test]
    fn test_all_states_have_icons() {
        let states = [
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Error,
        ];
        for state in states {
            assert!(!state.icon().is_empty());
        }
    }

    // NEW: NetworkRole Display and helper tests (only non-duplicates)
    #[test]
    fn test_network_role_display() {
        assert_eq!(format!("{}", NetworkRole::Server), "[Mon] Server");
        assert_eq!(format!("{}", NetworkRole::Client), "[PC] Client");
    }

    #[test]
    fn test_network_role_name() {
        assert_eq!(NetworkRole::ListenServer.name(), "Listen Server");
        assert_eq!(NetworkRole::Offline.name(), "Offline");
    }

    #[test]
    fn test_network_role_is_server() {
        assert!(NetworkRole::Server.is_server());
        assert!(NetworkRole::ListenServer.is_server());
        assert!(!NetworkRole::Client.is_server());
    }

    #[test]
    fn test_network_role_is_online() {
        assert!(NetworkRole::Server.is_online());
        assert!(NetworkRole::Client.is_online());
        assert!(!NetworkRole::Offline.is_online());
    }

    #[test]
    fn test_network_role_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NetworkRole::Server);
        set.insert(NetworkRole::Client);
        assert_eq!(set.len(), 2);
    }

    // NEW: ConnectionState Display and helper tests (only non-duplicates)
    #[test]
    fn test_connection_state_all_method() {
        let states = ConnectionState::all();
        assert_eq!(states.len(), 5);
    }

    #[test]
    fn test_connection_state_display() {
        assert_eq!(format!("{}", ConnectionState::Connected), "[G] Connected");
        assert_eq!(format!("{}", ConnectionState::Error), "[R] Error");
    }

    #[test]
    fn test_connection_state_name() {
        assert_eq!(ConnectionState::Reconnecting.name(), "Reconnecting");
        assert_eq!(ConnectionState::Connecting.name(), "Connecting");
    }

    #[test]
    fn test_connection_state_is_active() {
        assert!(ConnectionState::Connecting.is_active());
        assert!(ConnectionState::Connected.is_active());
        assert!(ConnectionState::Reconnecting.is_active());
        assert!(!ConnectionState::Disconnected.is_active());
        assert!(!ConnectionState::Error.is_active());
    }

    #[test]
    fn test_connection_state_is_stable() {
        assert!(ConnectionState::Connected.is_stable());
        assert!(!ConnectionState::Connecting.is_stable());
        assert!(!ConnectionState::Reconnecting.is_stable());
    }

    #[test]
    fn test_connection_state_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ConnectionState::Connected);
        set.insert(ConnectionState::Error);
        assert_eq!(set.len(), 2);
    }

    // NEW: InterestPolicy Display and helper tests (only non-duplicates)
    #[test]
    fn test_interest_policy_display() {
        assert_eq!(format!("{}", InterestPolicy::Radius), "⭕ Radius");
        assert_eq!(format!("{}", InterestPolicy::Full), "[Net] Full");
    }

    #[test]
    fn test_interest_policy_name() {
        assert_eq!(InterestPolicy::FieldOfView.name(), "Field of View");
        assert_eq!(InterestPolicy::FieldOfViewWithLOS.name(), "FOV + LOS");
    }

    #[test]
    fn test_interest_policy_icon() {
        assert_eq!(InterestPolicy::FieldOfView.icon(), "👁️");
        assert_eq!(InterestPolicy::Custom.icon(), "⚙️");
    }

    #[test]
    fn test_interest_policy_is_spatial() {
        assert!(InterestPolicy::Radius.is_spatial());
        assert!(InterestPolicy::FieldOfView.is_spatial());
        assert!(!InterestPolicy::Full.is_spatial());
    }

    #[test]
    fn test_interest_policy_has_visibility_check() {
        assert!(InterestPolicy::FieldOfView.has_visibility_check());
        assert!(InterestPolicy::FieldOfViewWithLOS.has_visibility_check());
        assert!(!InterestPolicy::Radius.has_visibility_check());
    }

    #[test]
    fn test_interest_policy_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(InterestPolicy::Radius);
        set.insert(InterestPolicy::Full);
        assert_eq!(set.len(), 2);
    }

    // NEW: CompressionLevel Display and helper tests (only non-duplicates)
    #[test]
    fn test_compression_level_display() {
        assert_eq!(format!("{}", CompressionLevel::Fast), "Fast");
        assert_eq!(format!("{}", CompressionLevel::None), "None");
    }

    #[test]
    fn test_compression_level_name() {
        assert_eq!(CompressionLevel::Balanced.name(), "Balanced");
        assert_eq!(CompressionLevel::Maximum.name(), "Maximum");
    }

    #[test]
    fn test_compression_level_description() {
        assert!(CompressionLevel::Fast.description().contains("Fast"));
        assert!(CompressionLevel::Maximum.description().contains("Maximum"));
    }

    #[test]
    fn test_compression_level_cpu_cost() {
        assert_eq!(CompressionLevel::None.cpu_cost(), 0);
        assert_eq!(CompressionLevel::Fast.cpu_cost(), 1);
        assert_eq!(CompressionLevel::Balanced.cpu_cost(), 2);
        assert_eq!(CompressionLevel::Maximum.cpu_cost(), 4);
    }

    #[test]
    fn test_compression_level_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CompressionLevel::Fast);
        set.insert(CompressionLevel::Balanced);
        assert_eq!(set.len(), 2);
    }

    // NEW: NetworkTab Display and helper tests
    #[test]
    fn test_network_tab_all_method() {
        let tabs = NetworkTab::all();
        assert_eq!(tabs.len(), 5);
    }

    #[test]
    fn test_network_tab_display() {
        assert_eq!(format!("{}", NetworkTab::Connection), "🔌 Connection");
        assert_eq!(format!("{}", NetworkTab::Statistics), "[Chart] Statistics");
    }

    #[test]
    fn test_network_tab_name() {
        assert_eq!(NetworkTab::Replication.name(), "Replication");
        assert_eq!(NetworkTab::Clients.name(), "Clients");
    }

    #[test]
    fn test_network_tab_icon() {
        assert_eq!(NetworkTab::Debug.icon(), "🐛");
        assert_eq!(NetworkTab::Clients.icon(), "[Grp]");
    }

    #[test]
    fn test_network_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NetworkTab::Connection);
        set.insert(NetworkTab::Statistics);
        assert_eq!(set.len(), 2);
    }

    // NetworkAction tests
    #[test]
    fn test_network_action_start_server() {
        let action = NetworkAction::StartServer { port: 7777 };
        assert!(matches!(action, NetworkAction::StartServer { port: 7777 }));
    }

    #[test]
    fn test_network_action_connect() {
        let action = NetworkAction::Connect {
            address: "127.0.0.1".to_string(),
            port: 7777,
        };
        if let NetworkAction::Connect { address, port } = action {
            assert_eq!(address, "127.0.0.1");
            assert_eq!(port, 7777);
        } else {
            panic!("Expected Connect action");
        }
    }

    #[test]
    fn test_network_action_kick_client() {
        let action = NetworkAction::KickClient {
            client_id: 42,
            reason: "Cheating".to_string(),
        };
        if let NetworkAction::KickClient { client_id, reason } = action {
            assert_eq!(client_id, 42);
            assert_eq!(reason, "Cheating");
        } else {
            panic!("Expected KickClient action");
        }
    }

    #[test]
    fn test_network_action_ban_client() {
        let action = NetworkAction::BanClient {
            client_id: 99,
            duration_seconds: Some(3600),
        };
        if let NetworkAction::BanClient {
            client_id,
            duration_seconds,
        } = action
        {
            assert_eq!(client_id, 99);
            assert_eq!(duration_seconds, Some(3600));
        } else {
            panic!("Expected BanClient action");
        }
    }

    #[test]
    fn test_network_action_set_role() {
        let action = NetworkAction::SetRole(NetworkRole::Server);
        assert!(matches!(
            action,
            NetworkAction::SetRole(NetworkRole::Server)
        ));
    }

    #[test]
    fn test_network_action_set_interest_policy() {
        let action = NetworkAction::SetInterestPolicy(InterestPolicy::Radius);
        assert!(matches!(
            action,
            NetworkAction::SetInterestPolicy(InterestPolicy::Radius)
        ));
    }

    #[test]
    fn test_network_action_toggle_lag_simulation() {
        let action = NetworkAction::ToggleLagSimulation(true);
        assert!(matches!(action, NetworkAction::ToggleLagSimulation(true)));
    }

    #[test]
    fn test_network_action_set_simulated_latency() {
        let action = NetworkAction::SetSimulatedLatency(100);
        assert!(matches!(action, NetworkAction::SetSimulatedLatency(100)));
    }

    #[test]
    fn test_network_action_equality() {
        let a1 = NetworkAction::StopServer;
        let a2 = NetworkAction::StopServer;
        assert_eq!(a1, a2);
    }

    #[test]
    fn test_network_action_queue_and_take() {
        let mut panel = NetworkingPanel::new();
        assert!(!panel.has_pending_actions());

        panel.queue_action(NetworkAction::StartServer { port: 8080 });
        panel.queue_action(NetworkAction::SetTickRate(60));
        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_network_action_send_test_packet() {
        let action = NetworkAction::SendTestPacket;
        assert!(matches!(action, NetworkAction::SendTestPacket));
    }

    #[test]
    fn test_network_action_refresh_stats() {
        let action = NetworkAction::RefreshStats;
        assert!(matches!(action, NetworkAction::RefreshStats));
    }

    #[test]
    fn test_network_action_debug() {
        let action = NetworkAction::TogglePacketLogging(true);
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("TogglePacketLogging"));
    }
}
