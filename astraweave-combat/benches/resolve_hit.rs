use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use astraweave_combat::{
    canonical_tables, hit_rng, resolve_hit, AttackerView, BlockGate, DefenderView, ImmuneGate,
    Modifier, ModifierKind,
};
use criterion::{criterion_group, criterion_main, Criterion};

struct CountAlloc;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountAlloc = CountAlloc;

fn bench_resolve(c: &mut Criterion) {
    let tables = canonical_tables().expect("fixture");
    let atk_mods = [
        Modifier {
            id: 1,
            kind: ModifierKind::AttackerBonusBp,
            bp_or_flat: 1_500,
        },
        Modifier {
            id: 2,
            kind: ModifierKind::AttackerBonusBp,
            bp_or_flat: 2_000,
        },
    ];
    let def_mods = [
        Modifier {
            id: 10,
            kind: ModifierKind::DefenderReduceBp,
            bp_or_flat: 4_000,
        },
        Modifier {
            id: 11,
            kind: ModifierKind::DefenderReduceBp,
            bp_or_flat: 2_000,
        },
    ];
    let atk = AttackerView {
        might: 100,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &atk_mods,
    };
    let def = DefenderView {
        hp: astraweave_combat::Fixed64::from_int(1_000),
        hp_max: astraweave_combat::Fixed64::from_int(1_000),
        poise: astraweave_combat::Fixed64::from_int(1),
        poise_max: astraweave_combat::Fixed64::from_int(1),
        armor: 400,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 2,
        modifiers: &def_mods,
        block: BlockGate {
            active: true,
            in_arc: true,
        },
        immune: ImmuneGate::default(),
    };
    let mut warm = hit_rng(1, 1, 0, 1);
    let _ = resolve_hit(&tables, &atk, &def, 1, &mut warm).unwrap();
    let before = ALLOCS.load(Ordering::Relaxed);
    for ordinal in 0..64u64 {
        let mut rng = hit_rng(1, 2, ordinal, 1);
        black_box(resolve_hit(&tables, &atk, &def, 1, &mut rng).unwrap());
    }
    assert_eq!(
        ALLOCS.load(Ordering::Relaxed),
        before,
        "resolve_hit allocated during the untimed loop"
    );
    c.bench_function("resolve_hit", |b| {
        b.iter(|| {
            let mut rng = hit_rng(1, 1, 0, 1);
            black_box(resolve_hit(&tables, &atk, &def, 1, &mut rng).unwrap())
        })
    });
    c.bench_function("resolve_hit_batch_10k", |b| {
        b.iter(|| {
            let mut last = 0i64;
            for ordinal in 0..10_000u64 {
                let mut rng = hit_rng(1, 9, ordinal, 1);
                last = resolve_hit(&tables, &atk, &def, 1, &mut rng)
                    .unwrap()
                    .hp_delta
                    .raw();
            }
            black_box(last)
        })
    });
}

criterion_group!(benches, bench_resolve);
criterion_main!(benches);
