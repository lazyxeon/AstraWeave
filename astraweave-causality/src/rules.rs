//! Baked consequence table. Pace and sack size are fields, not match arms.

/// Structure, route, actor, and quest-template namespaces.
pub const CLASS_STRUCTURE: u16 = 1;
pub const CLASS_ROUTE: u16 = 2;
pub const CLASS_ACTOR: u16 = 3;
pub const CLASS_QUEST: u16 = 4;

pub const KIND_GEOMETRY_CUT: u16 = 1;
pub const KIND_ROUTE_BLOCKED: u16 = 2;
pub const KIND_ROUTE_CLASS: u16 = 3;
pub const KIND_SUPPLY_DELTA: u16 = 4;
pub const KIND_SUPPLY_SHORT: u16 = 5;
pub const KIND_NPC_STANCE: u16 = 6;
pub const KIND_QUEST_OFFER: u16 = 7;
pub const KIND_ACTOR_CELL: u16 = 8;
pub const KIND_QUEST_COMPLETE: u16 = 9;

pub const ROUTE_BRIDGE: i32 = 1;
pub const ROUTE_FORD: i32 = 2;
pub const STANCE_TRADING: i32 = 1;
pub const STANCE_ASKING: i32 = 2;
pub const FORD_CELL: i32 = 2;
pub const QUEST_TEMPLATE: i32 = 1;

pub const BRIDGE_STRUCTURE: u64 = 1;
pub const TRADE_ROUTE: u64 = 1;
pub const MERCHANT: u64 = 7;

/// Authored span from the Aravath bridge mark, in milli-units.
/// Not the placeholder collapse notice.
pub const BRIDGE_SPAN_MIN: [i32; 3] = [-1_000, -1_000, 17_000];
pub const BRIDGE_SPAN_MAX: [i32; 3] = [5_000, 1_000, 31_999];

pub const OFFER_LINE: &str = "The ford is the only road. Carry the sacks across.";
pub const COMPLETE_LINE: &str = "The sacks are across the ford.";

pub const CAUSAL_BRIDGE_SEED: u64 = 0xCA05_B81D;
pub const BRIDGE_STOCK: i32 = 8;
pub const BRIDGE_PACE_TICKS: u32 = 10;
pub const BRIDGE_SACK_DELTA: i32 = -1;

/// Host input admitted on its tick. Region is milli-units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalInput {
    pub tick: u64,
    pub seq: u32,
    pub kind: u16,
    pub class: u16,
    pub subject: u64,
    pub region: [i32; 6],
    pub payload: [i32; 4],
}

/// One row of the rule table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalRule {
    pub id: u16,
    pub when_kind: u16,
    pub when_class: u16,
    pub when_subject: u64,
    pub any_subject: bool,
    pub check_payload: bool,
    pub payload_index: u8,
    pub payload_value: i32,
    pub requires_kind: u16,
    pub emit_kind: u16,
    pub emit_class: u16,
    pub emit_subject: u64,
    pub emit_payload: [i32; 4],
    pub delay_ticks: u32,
    pub every_ticks: u32,
    pub stock_delta: i32,
}

impl CausalRule {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            when_kind: 0,
            when_class: 0,
            when_subject: 0,
            any_subject: false,
            check_payload: false,
            payload_index: 0,
            payload_value: 0,
            requires_kind: 0,
            emit_kind: 0,
            emit_class: 0,
            emit_subject: 0,
            emit_payload: [0; 4],
            delay_ticks: 0,
            every_ticks: 0,
            stock_delta: 0,
        }
    }
}

/// Bridge shortage table. `pace_ticks` is the delay and the repeat. `sack_delta` is the stock step.
#[must_use]
pub fn bridge_rules(pace_ticks: u32, sack_delta: i32) -> [CausalRule; 7] {
    let mut cut = CausalRule::empty();
    cut.id = 1;
    cut.when_kind = KIND_GEOMETRY_CUT;
    cut.when_class = CLASS_STRUCTURE;
    cut.when_subject = BRIDGE_STRUCTURE;
    cut.emit_kind = KIND_ROUTE_BLOCKED;
    cut.emit_class = CLASS_ROUTE;
    cut.emit_subject = TRADE_ROUTE;
    cut.emit_payload = [TRADE_ROUTE as i32, 0, 0, 0];

    let mut blocked = CausalRule::empty();
    blocked.id = 2;
    blocked.when_kind = KIND_ROUTE_BLOCKED;
    blocked.when_class = CLASS_ROUTE;
    blocked.when_subject = TRADE_ROUTE;
    blocked.emit_kind = KIND_ROUTE_CLASS;
    blocked.emit_class = CLASS_ROUTE;
    blocked.emit_subject = TRADE_ROUTE;
    blocked.emit_payload = [ROUTE_FORD, 0, 0, 0];

    let mut ford = CausalRule::empty();
    ford.id = 3;
    ford.when_kind = KIND_ROUTE_CLASS;
    ford.when_class = CLASS_ROUTE;
    ford.when_subject = TRADE_ROUTE;
    ford.check_payload = true;
    ford.payload_index = 0;
    ford.payload_value = ROUTE_FORD;
    ford.emit_kind = KIND_SUPPLY_DELTA;
    ford.emit_class = CLASS_ROUTE;
    ford.emit_subject = TRADE_ROUTE;
    ford.delay_ticks = pace_ticks;
    ford.every_ticks = pace_ticks;
    ford.stock_delta = sack_delta;

    let mut short = CausalRule::empty();
    short.id = 4;
    short.when_kind = KIND_SUPPLY_DELTA;
    short.when_class = CLASS_ROUTE;
    short.any_subject = true;
    short.check_payload = true;
    short.payload_index = 1;
    short.payload_value = 0;
    short.emit_kind = KIND_SUPPLY_SHORT;
    short.emit_class = CLASS_ROUTE;
    short.emit_subject = TRADE_ROUTE;

    let mut stance = CausalRule::empty();
    stance.id = 5;
    stance.when_kind = KIND_SUPPLY_SHORT;
    stance.when_class = CLASS_ROUTE;
    stance.when_subject = TRADE_ROUTE;
    stance.emit_kind = KIND_NPC_STANCE;
    stance.emit_class = CLASS_ACTOR;
    stance.emit_subject = MERCHANT;
    stance.emit_payload = [STANCE_ASKING, 0, 0, 0];

    let mut offer = CausalRule::empty();
    offer.id = 6;
    offer.when_kind = KIND_NPC_STANCE;
    offer.when_class = CLASS_ACTOR;
    offer.when_subject = MERCHANT;
    offer.check_payload = true;
    offer.payload_value = STANCE_ASKING;
    offer.emit_kind = KIND_QUEST_OFFER;
    offer.emit_class = CLASS_QUEST;
    offer.emit_subject = QUEST_TEMPLATE as u64;
    offer.emit_payload = [QUEST_TEMPLATE, 0, 0, 0];

    let mut done = CausalRule::empty();
    done.id = 7;
    done.when_kind = KIND_ACTOR_CELL;
    done.when_class = CLASS_ACTOR;
    done.when_subject = MERCHANT;
    done.check_payload = true;
    done.payload_value = FORD_CELL;
    done.requires_kind = KIND_QUEST_OFFER;
    done.emit_kind = KIND_QUEST_COMPLETE;
    done.emit_class = CLASS_QUEST;
    done.emit_subject = QUEST_TEMPLATE as u64;
    done.emit_payload = [QUEST_TEMPLATE, 0, 0, 0];

    [cut, blocked, ford, short, stance, offer, done]
}

#[must_use]
pub fn bridge_cut_input() -> CausalInput {
    CausalInput {
        tick: 30,
        seq: 0,
        kind: KIND_GEOMETRY_CUT,
        class: CLASS_STRUCTURE,
        subject: BRIDGE_STRUCTURE,
        region: [
            BRIDGE_SPAN_MIN[0],
            BRIDGE_SPAN_MIN[1],
            BRIDGE_SPAN_MIN[2],
            BRIDGE_SPAN_MAX[0],
            BRIDGE_SPAN_MAX[1],
            BRIDGE_SPAN_MAX[2],
        ],
        payload: [1, 0, 0, 0],
    }
}

#[must_use]
pub fn merchant_ford_input() -> CausalInput {
    CausalInput {
        tick: 120,
        seq: 0,
        kind: KIND_ACTOR_CELL,
        class: CLASS_ACTOR,
        subject: MERCHANT,
        region: [16_000, 0, 0, 16_000, 0, 0],
        payload: [FORD_CELL, 0, 0, 0],
    }
}
