use astraweave_ecs::{App, SystemStage};
use astraweave_scripting::{script_system, CScript, ScriptEngineResource};
use criterion::{criterion_group, criterion_main, Criterion};
use rhai::{Engine, Scope};
use std::hint::black_box;
use std::sync::Arc;

fn bench_script_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("script_execution");

    let engine = Engine::new();
    let ast = engine.compile("let x = 10; let y = 20; x + y").unwrap();
    let ast_arc = Arc::new(ast);

    group.bench_function("rhai_raw_execution", |b| {
        b.iter(|| {
            let mut scope = Scope::new();
            engine
                .run_ast_with_scope(&mut scope, black_box(&ast_arc))
                .unwrap();
        })
    });

    let ecs_engine = Engine::new();
    let ecs_ast = ecs_engine.compile("let x = 10; let y = 20; x + y").unwrap();
    let ecs_ast_arc = Arc::new(ecs_ast);

    let mut app = App::new().insert_resource(ScriptEngineResource(Arc::new(ecs_engine)));
    app.add_system(SystemStage::SIMULATION, script_system)
        .expect("registered schedule stage");

    for _ in 0..1000 {
        let e = app.world.spawn();
        let mut script = CScript::new("bench.rhai", "let x = 10; let y = 20; x + y");
        script.cached_ast = Some(ecs_ast_arc.clone());
        app.world.insert(e, script);
    }

    group.bench_function("ecs_script_system_1000_entities", |b| {
        b.iter(|| {
            app.schedule.run(&mut app.world);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_script_execution);
criterion_main!(benches);
