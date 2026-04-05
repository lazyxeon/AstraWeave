#![forbid(unsafe_code)]
//! # AstraWeave Networking
//!
//! Snapshot-based networking for multiplayer games with delta compression.
//!
//! ## Core Concepts
//! - **Snapshot**: Complete world state at a point in time
//! - **Delta**: Minimal diff between snapshots for bandwidth efficiency
//! - **Interest**: Filter entities sent to each client
//!
//! ## Features
//! - Delta compression with `diff_snapshots()` / `apply_delta()`
//! - Interest management: `FullInterest`, `RadiusTeamInterest`, `FovInterest`
//! - Deterministic replay support
//! - Built-in server with `GameServer`
//! - **TLS/SSL support** (enable with `tls` feature)

pub mod error;
#[cfg(feature = "tls")]
pub mod tls;

pub use error::{NetError, NetResult};

use anyhow::Result;
use astraweave_core::*;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{broadcast, Mutex};
use tokio::time::{sleep, Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

const SNAPSHOT_VERSION: u16 = 1;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EntityState {
    pub id: u32,
    pub pos: IVec2,
    pub hp: i32,
    pub team: u8,
    pub ammo: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Snapshot {
    pub version: u16,
    pub tick: u64,
    pub t: f32,
    pub seq: u32,
    pub world_hash: u64,
    pub entities: Vec<EntityState>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EntityDeltaMask(u8);

impl EntityDeltaMask {
    const POS: u8 = 1 << 0;
    const HP: u8 = 1 << 1;
    const TEAM: u8 = 1 << 2;
    const AMMO: u8 = 1 << 3;
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EntityDelta {
    pub id: u32,
    pub mask: u8,
    pub pos: Option<IVec2>,
    pub hp: Option<i32>,
    pub team: Option<u8>,
    pub ammo: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Delta {
    pub base_tick: u64,
    pub tick: u64,
    pub changed: Vec<EntityDelta>,
    pub removed: Vec<u32>,
    pub head_hash: u64,
}

pub trait Interest: Send + Sync {
    fn include(&self, viewer: &EntityState, e: &EntityState) -> bool;
}

pub struct FullInterest;
impl Interest for FullInterest {
    fn include(&self, _viewer: &EntityState, _e: &EntityState) -> bool {
        true
    }
}

pub struct RadiusTeamInterest {
    pub radius: i32,
}
impl Interest for RadiusTeamInterest {
    fn include(&self, viewer: &EntityState, e: &EntityState) -> bool {
        if viewer.team == e.team {
            return true;
        }
        let dx = e.pos.x - viewer.pos.x;
        let dy = e.pos.y - viewer.pos.y;
        (dx * dx + dy * dy) <= self.radius * self.radius
    }
}

pub struct FovInterest {
    pub radius: i32,
    pub half_angle_deg: f32,
    pub facing: IVec2, // approximate facing vector
}
impl Interest for FovInterest {
    fn include(&self, viewer: &EntityState, e: &EntityState) -> bool {
        if viewer.team == e.team {
            return true;
        }
        let dx = (e.pos.x - viewer.pos.x) as f32;
        let dy = (e.pos.y - viewer.pos.y) as f32;
        let dist2 = dx * dx + dy * dy;
        if dist2 > (self.radius * self.radius) as f32 {
            return false;
        }
        let fx = self.facing.x as f32;
        let fy = self.facing.y as f32;
        let fmag = (fx * fx + fy * fy).sqrt();
        if fmag == 0.0 {
            return true;
        }
        let vmag = (dist2).sqrt();
        if vmag == 0.0 {
            return true;
        }
        let dot = fx * dx + fy * dy;
        let cos_theta = dot / (fmag * vmag);
        let cos_half = (self.half_angle_deg.to_radians()).cos();
        cos_theta >= cos_half
    }
}

pub struct FovLosInterest {
    pub radius: i32,
    pub half_angle_deg: f32,
    pub facing: IVec2,
    pub obstacles: BTreeSet<(i32, i32)>,
}

fn has_los(a: IVec2, b: IVec2, obstacles: &BTreeSet<(i32, i32)>) -> bool {
    // Bresenham's line algorithm; return false if any obstacle cell intersects
    let (mut x0, mut y0) = (a.x, a.y);
    let (x1, y1) = (b.x, b.y);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if !(x0 == a.x && y0 == a.y) {
            // skip the starting cell occupied by viewer
            if obstacles.contains(&(x0, y0)) {
                return false;
            }
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    true
}

impl Interest for FovLosInterest {
    fn include(&self, viewer: &EntityState, e: &EntityState) -> bool {
        if viewer.team == e.team {
            return true;
        }
        let dx = (e.pos.x - viewer.pos.x) as f32;
        let dy = (e.pos.y - viewer.pos.y) as f32;
        let dist2 = dx * dx + dy * dy;
        if dist2 > (self.radius * self.radius) as f32 {
            return false;
        }
        let fx = self.facing.x as f32;
        let fy = self.facing.y as f32;
        let fmag = (fx * fx + fy * fy).sqrt();
        if fmag == 0.0 {
            return has_los(viewer.pos, e.pos, &self.obstacles);
        }
        let vmag = (dist2).sqrt();
        if vmag == 0.0 {
            return true;
        }
        let dot = fx * dx + fy * dy;
        let cos_theta = dot / (fmag * vmag);
        let cos_half = (self.half_angle_deg.to_radians()).cos();
        cos_theta >= cos_half && has_los(viewer.pos, e.pos, &self.obstacles)
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum InterestPolicy {
    Radius {
        radius: i32,
    },
    Fov {
        radius: i32,
        half_angle_deg: f32,
        facing: IVec2,
    },
    FovLos {
        radius: i32,
        half_angle_deg: f32,
        facing: IVec2,
    },
}

fn stable_hash_snapshot(ents: &[EntityState], obstacles: &BTreeSet<(i32, i32)>) -> u64 {
    // Canonical ordering
    let mut hasher = DefaultHasher::new();
    for e in ents.iter() {
        e.id.hash(&mut hasher);
        e.pos.x.hash(&mut hasher);
        e.pos.y.hash(&mut hasher);
        e.hp.hash(&mut hasher);
        e.team.hash(&mut hasher);
        e.ammo.hash(&mut hasher);
    }
    for o in obstacles.iter() {
        o.hash(&mut hasher);
    }
    hasher.finish()
}

fn subset_hash(ents: &[EntityState]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for e in ents.iter() {
        e.id.hash(&mut hasher);
        e.pos.x.hash(&mut hasher);
        e.pos.y.hash(&mut hasher);
        e.hp.hash(&mut hasher);
        e.team.hash(&mut hasher);
        e.ammo.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod tests;

fn world_to_entities(w: &World) -> Vec<EntityState> {
    // Stable by sorting by id
    let mut ids: Vec<u32> = w
        .all_of_team(0)
        .into_iter()
        .chain(w.all_of_team(1))
        .chain(w.all_of_team(2))
        .collect();
    ids.sort_unstable();
    ids.into_iter()
        .filter_map(|id| {
            let pos = w.pos_of(id)?;
            let hp = w.health(id)?.hp;
            let team = w.team(id)?.id;
            let ammo = w.ammo(id)?.rounds;
            Some(EntityState {
                id,
                pos,
                hp,
                team,
                ammo,
            })
        })
        .collect()
}

fn world_obstacles_btree(w: &World) -> BTreeSet<(i32, i32)> {
    w.obstacles.iter().cloned().collect()
}

pub fn build_snapshot(world: &World, tick: u64, seq: u32) -> Snapshot {
    let entities = world_to_entities(world);
    let obstacles = world_obstacles_btree(world);
    let world_hash = stable_hash_snapshot(&entities, &obstacles);
    Snapshot {
        version: SNAPSHOT_VERSION,
        tick,
        t: world.t,
        seq,
        world_hash,
        entities,
    }
}

pub fn filter_snapshot_for_viewer(
    head: &Snapshot,
    interest: &(impl Interest + ?Sized),
    viewer: &EntityState,
) -> Snapshot {
    let entities: Vec<EntityState> = head
        .entities
        .iter()
        .cloned()
        .filter(|e| interest.include(viewer, e))
        .collect();
    let mut snap = head.clone();
    snap.entities = entities;
    snap.world_hash = subset_hash(&snap.entities);
    snap
}

pub fn diff_snapshots(
    base: &Snapshot,
    head: &Snapshot,
    interest: &impl Interest,
    viewer: &EntityState,
) -> Delta {
    let mut base_map: BTreeMap<u32, &EntityState> = BTreeMap::new();
    for e in &base.entities {
        base_map.insert(e.id, e);
    }
    let mut changed = Vec::new();
    let mut present: BTreeSet<u32> = BTreeSet::new();
    for e in &head.entities {
        if !interest.include(viewer, e) {
            continue;
        }
        present.insert(e.id);
        let mut mask = 0u8;
        let mut pos = None;
        let mut hp = None;
        let mut team = None;
        let mut ammo = None;
        match base_map.get(&e.id) {
            Some(prev) => {
                if prev.pos != e.pos {
                    mask |= EntityDeltaMask::POS;
                    pos = Some(e.pos);
                }
                if prev.hp != e.hp {
                    mask |= EntityDeltaMask::HP;
                    hp = Some(e.hp);
                }
                if prev.team != e.team {
                    mask |= EntityDeltaMask::TEAM;
                    team = Some(e.team);
                }
                if prev.ammo != e.ammo {
                    mask |= EntityDeltaMask::AMMO;
                    ammo = Some(e.ammo);
                }
            }
            None => {
                // treat as full entity update
                mask = EntityDeltaMask::POS
                    | EntityDeltaMask::HP
                    | EntityDeltaMask::TEAM
                    | EntityDeltaMask::AMMO;
                pos = Some(e.pos);
                hp = Some(e.hp);
                team = Some(e.team);
                ammo = Some(e.ammo);
            }
        }
        if mask != 0 {
            changed.push(EntityDelta {
                id: e.id,
                mask,
                pos,
                hp,
                team,
                ammo,
            });
        }
    }
    let removed: Vec<u32> = base_map
        .keys()
        .filter(|id| !present.contains(id))
        .cloned()
        .collect();
    let head_hash = subset_hash(&head.entities);
    Delta {
        base_tick: base.tick,
        tick: head.tick,
        changed,
        removed,
        head_hash,
    }
}

pub fn apply_delta(base: &mut Snapshot, delta: &Delta) {
    if base.tick != delta.base_tick {
        return;
    }
    let mut map: BTreeMap<u32, EntityState> =
        base.entities.iter().cloned().map(|e| (e.id, e)).collect();
    for d in &delta.changed {
        let mut e = map.remove(&d.id).unwrap_or(EntityState {
            id: d.id,
            pos: IVec2 { x: 0, y: 0 },
            hp: 0,
            team: 0,
            ammo: 0,
        });
        if d.mask & EntityDeltaMask::POS != 0 {
            if let Some(v) = d.pos {
                e.pos = v;
            }
        }
        if d.mask & EntityDeltaMask::HP != 0 {
            if let Some(v) = d.hp {
                e.hp = v;
            }
        }
        if d.mask & EntityDeltaMask::TEAM != 0 {
            if let Some(v) = d.team {
                e.team = v;
            }
        }
        if d.mask & EntityDeltaMask::AMMO != 0 {
            if let Some(v) = d.ammo {
                e.ammo = v;
            }
        }
        map.insert(e.id, e);
    }
    for id in &delta.removed {
        map.remove(id);
    }
    base.entities = map.into_values().collect();
    base.tick = delta.tick;
    base.world_hash = delta.head_hash;
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum Msg {
    ClientHello {
        name: String,
        token: Option<String>,
        policy: Option<String>,
    },
    ServerWelcome {
        id: u32,
    },
    ServerSnapshot {
        snap: Snapshot,
    },
    ServerDelta {
        delta: Delta,
    },
    ClientProposePlan {
        actor_id: u32,
        intent: PlanIntent,
    },
    ClientInput {
        seq: u32,
        tick: u64,
        actor_id: u32,
        intent: PlanIntent,
    },
    ServerApplyResult {
        ok: bool,
        err: Option<String>,
    },
    ServerAck {
        seq: u32,
        tick_applied: u64,
    },
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ServerEvent {
    Snapshot(Snapshot),
    ForceSnapshot(Snapshot),
    ApplyResult { ok: bool, err: Option<String> },
    Ack { seq: u32, tick_applied: u64 },
}

pub struct GameServer {
    pub world: Mutex<World>,
    pub player_id: u32,
    pub companion_id: u32,
    pub enemy_id: u32,
    pub tx: broadcast::Sender<ServerEvent>,
    pub tick: AtomicU64,
    pub replay: Mutex<Vec<ReplayEvent>>,
    pub obstacles: std::sync::Arc<Mutex<BTreeSet<(i32, i32)>>>,
}

impl Default for GameServer {
    fn default() -> Self {
        Self::new()
    }
}

impl GameServer {
    pub fn new() -> Self {
        let mut w = World::new();
        for y in 1..=8 {
            w.obstacles.insert((6, y));
        }
        let player = w.spawn("P", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
        let comp = w.spawn("C", IVec2 { x: 2, y: 3 }, Team { id: 1 }, 80, 30);
        let enemy = w.spawn("E", IVec2 { x: 12, y: 2 }, Team { id: 2 }, 60, 0);
        let (tx, _) = broadcast::channel(1024);
        Self {
            world: Mutex::new(w),
            player_id: player,
            companion_id: comp,
            enemy_id: enemy,
            tx,
            tick: AtomicU64::new(0),
            replay: Mutex::new(Vec::new()),
            obstacles: std::sync::Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    pub async fn run_ws(self: &std::sync::Arc<Self>, addr: &str) -> Result<()> {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind(addr).await?;
        self.run_ws_on_listener(listener).await
    }

    pub async fn run_ws_on_listener(
        self: &std::sync::Arc<Self>,
        listener: tokio::net::TcpListener,
    ) -> Result<()> {
        let addr = listener.local_addr()?;
        println!("Server on {addr}");
        // Fixed-tick loop at ~60 Hz; broadcast snapshots at ~20 Hz and full once per second
        let me = self.clone();
        tokio::spawn(async move {
            let dt = 1.0f32 / 60.0f32;
            let mut last_broadcast = 0u64;
            let mut last_full = 0u64;
            let mut seq: u32 = 0;
            let mut next = Instant::now();
            loop {
                let now = Instant::now();
                if now < next {
                    sleep(next - now).await;
                }
                next += Duration::from_micros(16_666);
                let tick = me.tick.fetch_add(1, Ordering::Relaxed) + 1;
                if tick % 60 == 0 {
                    println!("Server tick {tick}");
                }
                {
                    let mut w = me.world.lock().await;
                    w.tick(dt);
                    // Build snapshot for this tick and update obstacles cache
                    let snap = build_snapshot(&w, tick, seq);
                    let obs = world_obstacles_btree(&w);
                    {
                        let mut o = me.obstacles.lock().await;
                        *o = obs;
                    }
                    seq = seq.wrapping_add(1);
                    if tick - last_full >= 60 {
                        let _ = me.tx.send(ServerEvent::Snapshot(snap.clone()));
                        last_full = tick;
                    } else if tick - last_broadcast >= 3 {
                        let _ = me.tx.send(ServerEvent::Snapshot(snap));
                        last_broadcast = tick;
                    }
                }
            }
        });
        while let Ok((stream, _)) = listener.accept().await {
            let me = self.clone();
            tokio::spawn(async move {
                if let Err(e) = me.handle_conn(stream).await {
                    eprintln!("conn error: {e:?}");
                }
            });
        }
        Ok(())
    }

    async fn handle_conn(self: std::sync::Arc<Self>, stream: tokio::net::TcpStream) -> Result<()> {
        let ws = tokio_tungstenite::accept_async(stream).await?;
        let (mut tx, mut rx) = ws.split();
        let mut rx_bcast = self.tx.subscribe();

        // send welcome (id will be updated after ClientHello)
        tx.send(Message::Text(
            serde_json::to_string(&Msg::ServerWelcome { id: 1 })?.into(),
        ))
        .await?;

        // spawn a task to push snapshots
        let viewer_id = std::sync::Arc::new(Mutex::new(self.player_id)); // updated on ClientHello
        let policy = std::sync::Arc::new(Mutex::new(InterestPolicy::Radius { radius: 6 }));
        let writer_viewer = viewer_id.clone();
        let writer_policy = policy.clone();
        let obstacles_ref = self.obstacles.clone();
        tokio::spawn(async move {
            println!("Snapshot pusher started for viewer");
            let mut last_sent: Option<Snapshot> = None;
            loop {
                let evt = match rx_bcast.recv().await {
                    Ok(e) => e,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                };
                match evt {
                    ServerEvent::ForceSnapshot(snap) => {
                        let vid = { *writer_viewer.lock().await };
                        if let Some(viewer) = snap.entities.iter().find(|e| e.id == vid).cloned() {
                            let interest_obj: Box<dyn Interest> = {
                                let pol = writer_policy.lock().await.clone();
                                match pol {
                                    InterestPolicy::Radius { radius } => {
                                        Box::new(RadiusTeamInterest { radius }) as Box<dyn Interest>
                                    }
                                    InterestPolicy::Fov {
                                        radius,
                                        half_angle_deg,
                                        facing,
                                    } => Box::new(FovInterest {
                                        radius,
                                        half_angle_deg,
                                        facing,
                                    })
                                        as Box<dyn Interest>,
                                    InterestPolicy::FovLos {
                                        radius,
                                        half_angle_deg,
                                        facing,
                                    } => {
                                        let obs = obstacles_ref.lock().await.clone();
                                        Box::new(FovLosInterest {
                                            radius,
                                            half_angle_deg,
                                            facing,
                                            obstacles: obs,
                                        })
                                            as Box<dyn Interest>
                                    }
                                }
                            };
                            let filtered =
                                filter_snapshot_for_viewer(&snap, &*interest_obj, &viewer);
                            let msg = Msg::ServerSnapshot {
                                snap: filtered.clone(),
                            };
                            println!("Forcing full snapshot to viewer {}", vid);
                            #[allow(clippy::unwrap_used)]
                            // Msg implements Serialize; serialization is infallible for known enum variants
                            let _ = tx
                                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
                                .await;
                            last_sent = Some(filtered);
                        }
                    }
                    ServerEvent::Snapshot(snap) => {
                        // Choose viewer state
                        let vid = { *writer_viewer.lock().await };
                        if let Some(viewer) = snap.entities.iter().find(|e| e.id == vid).cloned() {
                            // Build interest based on current policy
                            let interest_obj: Box<dyn Interest> = {
                                let pol = writer_policy.lock().await.clone();
                                match pol {
                                    InterestPolicy::Radius { radius } => {
                                        Box::new(RadiusTeamInterest { radius }) as Box<dyn Interest>
                                    }
                                    InterestPolicy::Fov {
                                        radius,
                                        half_angle_deg,
                                        facing,
                                    } => Box::new(FovInterest {
                                        radius,
                                        half_angle_deg,
                                        facing,
                                    })
                                        as Box<dyn Interest>,
                                    InterestPolicy::FovLos {
                                        radius,
                                        half_angle_deg,
                                        facing,
                                    } => {
                                        let obs = obstacles_ref.lock().await.clone();
                                        Box::new(FovLosInterest {
                                            radius,
                                            half_angle_deg,
                                            facing,
                                            obstacles: obs,
                                        })
                                            as Box<dyn Interest>
                                    }
                                }
                            };
                            // Filter the snapshot to the viewer interest
                            let filtered =
                                filter_snapshot_for_viewer(&snap, &*interest_obj, &viewer);
                            if let Some(base) = &last_sent {
                                let delta = diff_snapshots(base, &filtered, &FullInterest, &viewer);
                                if delta.changed.is_empty() && delta.removed.is_empty() {
                                    continue;
                                }
                                let msg = Msg::ServerDelta { delta };
                                println!("Sending delta to viewer {}", vid);
                                #[allow(clippy::unwrap_used)] // Msg Serialize is infallible
                                let res = tx
                                    .send(Message::Text(
                                        serde_json::to_string(&msg).unwrap().into(),
                                    ))
                                    .await;
                                println!("Send result: {:?}", res);
                            } else {
                                let msg = Msg::ServerSnapshot {
                                    snap: filtered.clone(),
                                };
                                println!("Sending initial snapshot to viewer {}", vid);
                                #[allow(clippy::unwrap_used)] // Msg Serialize is infallible
                                let res = tx
                                    .send(Message::Text(
                                        serde_json::to_string(&msg).unwrap().into(),
                                    ))
                                    .await;
                                println!("Send result: {:?}", res);
                            }
                            last_sent = Some(filtered);
                        } else {
                            // viewer not in snapshot; send full snapshot
                            let msg = Msg::ServerSnapshot { snap: snap.clone() };
                            #[allow(clippy::unwrap_used)] // Msg Serialize is infallible
                            let _ = tx
                                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
                                .await;
                            last_sent = Some(snap);
                        }
                    }
                    ServerEvent::ApplyResult { ok, err } => {
                        #[allow(clippy::unwrap_used)] // Msg Serialize is infallible
                        let _ = tx
                            .send(Message::Text(
                                serde_json::to_string(&Msg::ServerApplyResult { ok, err })
                                    .unwrap()
                                    .into(),
                            ))
                            .await;
                    }
                    ServerEvent::Ack { seq, tick_applied } => {
                        #[allow(clippy::unwrap_used)] // Msg Serialize is infallible
                        let _ = tx
                            .send(Message::Text(
                                serde_json::to_string(&Msg::ServerAck { seq, tick_applied })
                                    .unwrap()
                                    .into(),
                            ))
                            .await;
                    }
                }
            }
        });

        while let Some(msg) = rx.next().await {
            let msg = msg?;
            if msg.is_text() {
                let text = msg.into_text()?;
                let m: Msg = serde_json::from_str(&text)?;
                match m {
                    Msg::ClientHello {
                        name,
                        token,
                        policy: pol,
                    } => {
                        println!("Hello from {name}");
                        if let Some(tok) = token {
                            if tok != "dev" {
                                println!("unauthenticated or unknown token: {}", tok);
                            }
                        }
                        // Map name to viewer id
                        let mapped = match name.as_str() {
                            "player" | "player1" => self.player_id,
                            "comp" | "companion" => self.companion_id,
                            "enemy" => self.enemy_id,
                            _ => self.player_id,
                        };
                        {
                            let mut v = viewer_id.lock().await;
                            *v = mapped;
                        }
                        if let Some(kind) = pol.as_deref() {
                            let mut p = policy.lock().await;
                            *p = match kind {
                                "radius" => InterestPolicy::Radius { radius: 6 },
                                "fov" => InterestPolicy::Fov {
                                    radius: 6,
                                    half_angle_deg: 60.0,
                                    facing: IVec2 { x: 1, y: 0 },
                                },
                                "fovlos" => InterestPolicy::FovLos {
                                    radius: 6,
                                    half_angle_deg: 60.0,
                                    facing: IVec2 { x: 1, y: 0 },
                                },
                                _ => InterestPolicy::Radius { radius: 6 },
                            };
                        }
                        // send immediate full snapshot via broadcast
                        let w = self.world.lock().await;
                        let snap = build_snapshot(&w, self.tick.load(Ordering::Relaxed), 0);
                        let _ = self.tx.send(ServerEvent::ForceSnapshot(snap));
                    }
                    Msg::ClientProposePlan { actor_id, intent } => {
                        let mut w = self.world.lock().await;
                        let mut log = |s: String| println!("{}", s);
                        let vcfg = ValidateCfg {
                            world_bounds: (0, 0, 19, 9),
                        };
                        let res = validate_and_execute(&mut w, actor_id, &intent, &vcfg, &mut log);
                        let ok = res.is_ok();
                        let err = res.err().map(|e| e.to_string());
                        // Capture snapshot + append replay
                        let tick = self.tick.load(Ordering::Relaxed);
                        let snap = build_snapshot(&w, tick, 0);
                        {
                            let mut rec = self.replay.lock().await;
                            rec.push(ReplayEvent {
                                tick,
                                seq: 0,
                                actor_id,
                                intent: intent.clone(),
                                world_hash: snap.world_hash,
                            });
                        }
                        // Broadcast snapshot; per-connection task will produce deltas
                        let _ = self.tx.send(ServerEvent::Snapshot(snap));
                        let _ = self.tx.send(ServerEvent::ApplyResult { ok, err });
                    }
                    Msg::ClientInput {
                        actor_id,
                        intent,
                        seq,
                        ..
                    } => {
                        let mut w = self.world.lock().await;
                        let mut log = |s: String| println!("{}", s);
                        let vcfg = ValidateCfg {
                            world_bounds: (0, 0, 19, 9),
                        };
                        let res = validate_and_execute(&mut w, actor_id, &intent, &vcfg, &mut log);
                        let ok = res.is_ok();
                        let err = res.err().map(|e| e.to_string());
                        // Capture snapshot + append replay
                        let tick = self.tick.load(Ordering::Relaxed);
                        let snap = build_snapshot(&w, tick, 0);
                        {
                            let mut rec = self.replay.lock().await;
                            rec.push(ReplayEvent {
                                tick,
                                seq: 0,
                                actor_id,
                                intent: intent.clone(),
                                world_hash: snap.world_hash,
                            });
                        }
                        // Broadcast snapshot; per-connection task will produce deltas
                        let _ = self.tx.send(ServerEvent::Snapshot(snap));
                        let _ = self.tx.send(ServerEvent::ApplyResult { ok, err });
                        // Ack this input to help client reconcile prediction history
                        let _ = self.tx.send(ServerEvent::Ack {
                            seq,
                            tick_applied: tick,
                        });
                    }
                    Msg::ServerWelcome { .. }
                    | Msg::ServerSnapshot { .. }
                    | Msg::ServerDelta { .. }
                    | Msg::ServerApplyResult { .. }
                    | Msg::ServerAck { .. } => {
                        // ignore from clients
                    }
                }
            } else if msg.is_close() {
                break;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayEvent {
    pub tick: u64,
    pub seq: u32,
    pub actor_id: u32,
    pub intent: PlanIntent,
    pub world_hash: u64,
}

/// Replays a sequence of inputs against a fresh world state provided by caller, stepping deterministically at 60 Hz.
pub fn replay_from(mut world: World, events: &[ReplayEvent]) -> Result<u64> {
    let dt = 1.0f32 / 60.0f32;
    let mut current_tick: u64 = 0;
    // Sort by (tick, seq)
    let mut evs = events.to_vec();
    evs.sort_by_key(|e| (e.tick, e.seq));
    for e in evs {
        while current_tick < e.tick {
            world.tick(dt);
            current_tick += 1;
        }
        let mut log = |s: String| {
            let _ = s;
        };
        let vcfg = ValidateCfg {
            world_bounds: (0, 0, 19, 9),
        };
        let _ = validate_and_execute(&mut world, e.actor_id, &e.intent, &vcfg, &mut log);
    }
    let snap = build_snapshot(&world, current_tick, 0);
    Ok(snap.world_hash)
}
