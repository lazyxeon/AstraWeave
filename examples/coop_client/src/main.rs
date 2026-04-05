use anyhow::Result;
use astraweave_core::{ActionStep, IVec2, PlanIntent};
use astraweave_net::{apply_delta, Msg, Snapshot};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut policy = "radius".to_string();
    for w in args.windows(2) {
        if w[0] == "--policy" {
            policy = w[1].clone();
        }
    }
    let (ws, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9090").await?;
    let (mut tx, mut rx) = ws.split();

    // say hello
    tx.send(Message::Text(
        serde_json::to_string(&Msg::ClientHello {
            name: "player1".into(),
            token: Some("dev".into()),
            policy: Some(policy),
        })?
        .into(),
    ))
    .await?;

    // propose a plan for actor_id=2 (companion in our server)
    let plan = PlanIntent {
        plan_id: "client-plan".into(),
        steps: vec![
            ActionStep::MoveTo {
                x: 4,
                y: 3,
                speed: None,
            },
            ActionStep::Throw {
                item: "smoke".into(),
                x: 7,
                y: 3,
            },
            ActionStep::CoverFire {
                target_id: 3,
                duration: 2.0,
            },
        ],
    };
    // Client-side prediction scaffold
    let mut local: Option<Snapshot> = None;
    let mut seq: u32 = 0;
    let mut last_acked: u32 = 0;
    let mut history: Vec<(u32, PlanIntent)> = Vec::new();
    let actor_id: u32 = 2;
    // Send as ClientInput (prediction-ready)
    tx.send(Message::Text(
        serde_json::to_string(&Msg::ClientInput {
            seq,
            tick: 0,
            actor_id,
            intent: plan.clone(),
        })?
        .into(),
    ))
    .await?;
    history.push((seq, plan.clone()));
    #[allow(unused_assignments)]
    {
        seq = seq.wrapping_add(1);
    }
    // Naive local prediction for MoveTo: adjust actor position if we have a snapshot
    let predicted_target = plan.steps.iter().find_map(|s| {
        if let ActionStep::MoveTo { x, y, .. } = s {
            Some(IVec2 { x: *x, y: *y })
        } else {
            None
        }
    });

    // Process a few server messages and reconcile
    for _ in 0..10 {
        if let Some(msg) = rx.next().await {
            let txt = msg?.into_text()?;
            if let Ok(m) = serde_json::from_str::<Msg>(&txt) {
                match m {
                    Msg::ServerSnapshot { snap } => {
                        // authoritative reset
                        if let Some(pred) = predicted_target {
                            // perform naive prediction on top of snapshot
                            let mut s = snap.clone();
                            if let Some(e) = s.entities.iter_mut().find(|e| e.id == actor_id) {
                                e.pos = pred;
                                println!(
                                    "predicted pos for actor {} -> ({},{})",
                                    actor_id, pred.x, pred.y
                                );
                            }
                            local = Some(s);
                        } else {
                            local = Some(snap);
                        }
                    }
                    Msg::ServerDelta { delta } => {
                        if let Some(ref mut base) = local {
                            if base.tick == delta.base_tick {
                                apply_delta(base, &delta);
                                println!(
                                    "reconciled to tick {} (hash={})",
                                    base.tick, base.world_hash
                                );
                                // Re-apply unacked inputs (very simplified; assumes instantaneous intent effects)
                                let pending: Vec<_> = history
                                    .iter()
                                    .filter(|(s, _)| *s > last_acked)
                                    .cloned()
                                    .collect();
                                for (_s, intent) in pending {
                                    // At this level, we don't have a local physics sim; we could re-run naive prediction if desired
                                    if let Some(ActionStep::MoveTo { x, y, .. }) =
                                        intent.steps.first()
                                    {
                                        if let Some(e) =
                                            base.entities.iter_mut().find(|e| e.id == actor_id)
                                        {
                                            e.pos = IVec2 { x: *x, y: *y };
                                        }
                                    }
                                }
                            } else {
                                println!("delta base mismatch (have {}, got {}) — waiting for full snapshot", base.tick, delta.base_tick);
                            }
                        } else {
                            println!("no local snapshot yet — waiting for full snapshot");
                        }
                    }
                    Msg::ServerApplyResult { ok, err } => {
                        println!("apply_result ok={} err={:?}", ok, err);
                    }
                    Msg::ServerAck { seq, tick_applied } => {
                        println!("ack seq={} applied_at_tick={}", seq, tick_applied);
                        last_acked = last_acked.max(seq);
                        // Drop any history up to and including this seq
                        history.retain(|(s, _)| *s > last_acked);
                    }
                    _ => {
                        println!("<< {}", txt);
                    }
                }
            } else {
                println!("<< {}", txt);
            }
        }
    }
    Ok(())
}
