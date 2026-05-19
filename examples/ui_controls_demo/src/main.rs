use glam::{vec3, Vec2};
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use astraweave_physics::PhysicsWorld;
use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_render::{Instance, Renderer};

use astraweave_input::{BindingSet, InputContext, InputManager};
use astraweave_ui::{draw_ui, Accessibility, MenuManager, UiData, UiFlags, UiLayer};

use astraweave_gameplay::crafting::{CraftCost, CraftRecipe, RecipeBook};
use astraweave_gameplay::quests::QuestLog;
use astraweave_gameplay::stats::Stats;
use astraweave_gameplay::Inventory;
use astraweave_gameplay::{items::ItemKind, DamageType, ResourceKind};

fn build_sample_recipe_book() -> RecipeBook {
    RecipeBook {
        recipes: vec![
            CraftRecipe {
                name: "Echo Blade".into(),
                output_item: ItemKind::Weapon {
                    base_damage: 12,
                    dtype: DamageType::Physical,
                },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Ore,
                        count: 2,
                    },
                    CraftCost {
                        kind: ResourceKind::Crystal,
                        count: 1,
                    },
                ],
            },
            CraftRecipe {
                name: "Health Tonic".into(),
                output_item: ItemKind::Consumable { heal: 25 },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Essence,
                        count: 1,
                    },
                    CraftCost {
                        kind: ResourceKind::Fiber,
                        count: 2,
                    },
                ],
            },
        ],
    }
}

struct App {
    window: Option<std::sync::Arc<Window>>,
    renderer: Option<Renderer>,
    ui: Option<UiLayer>,
    camera: Camera,
    cam_ctl: CameraController,

    // Game state
    input: InputManager,
    flags: UiFlags,
    acc: Accessibility,
    menu_manager: MenuManager,
    player_stats: Stats,
    inventory: Inventory,
    recipes: RecipeBook,
    quest_log: QuestLog,
    last: Instant,
    instances: Vec<Instance>,
    #[allow(dead_code)]
    phys: PhysicsWorld,
}

impl App {
    fn new() -> Self {
        // Physics (for ground plane visual)
        let mut phys = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
        phys.create_ground_plane(vec3(100.0, 0.0, 100.0), 1.0);

        // Input system
        let bindings = BindingSet::default();
        let input = InputManager::new(InputContext::Gameplay, bindings);

        // Demo data
        let flags = UiFlags::default();
        let acc = Accessibility::default();
        let menu_manager = MenuManager::default();

        let player_stats = Stats::new(120);
        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceKind::Ore, 3);
        inventory.add_resource(ResourceKind::Crystal, 2);
        inventory.add_resource(ResourceKind::Essence, 1);
        inventory.add_resource(ResourceKind::Fiber, 3);

        let recipes = build_sample_recipe_book();
        let quest_log = QuestLog::default();

        let last = Instant::now();
        let instances: Vec<Instance> = vec![];

        let camera = Camera {
            position: vec3(0.0, 6.0, 14.0),
            yaw: -1.57,
            pitch: -0.35,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 300.0,
        };
        let cam_ctl = CameraController::new(12.0, 0.005);

        Self {
            window: None,
            renderer: None,
            ui: None,
            camera,
            cam_ctl,
            input,
            flags,
            acc,
            menu_manager,
            player_stats,
            inventory,
            recipes,
            quest_log,
            last,
            instances,
            phys,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win = std::sync::Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("UI & Controls Demo")
                            .with_inner_size(PhysicalSize::new(1280, 720)),
                    )
                    .unwrap(),
            );
            self.window = Some(win.clone());

            let r = pollster::block_on(Renderer::new(win.clone())).unwrap();
            let u = UiLayer::new(&win, r.device(), r.surface_format());

            self.renderer = Some(r);
            self.ui = Some(u);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(window), Some(ui), Some(renderer)) =
            (&self.window, &mut self.ui, &mut self.renderer)
        {
            let _consumed_by_ui = ui.on_event(window, &event);
            self.input.process_window_event(&event);

            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(s) => {
                    renderer.resize(s.width, s.height);
                    self.camera.aspect = s.width as f32 / s.height.max(1) as f32;
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state,
                            physical_key: PhysicalKey::Code(code),
                            ..
                        },
                    ..
                } => {
                    // Camera + toggles
                    self.cam_ctl
                        .process_keyboard(code, state == ElementState::Pressed);
                    if state == ElementState::Pressed {
                        match code {
                            KeyCode::KeyI => {
                                self.flags.show_inventory = !self.flags.show_inventory;
                            }
                            KeyCode::KeyC => {
                                self.flags.show_crafting = !self.flags.show_crafting;
                            }
                            KeyCode::KeyM => {
                                self.flags.show_map = !self.flags.show_map;
                            }
                            KeyCode::KeyJ => {
                                self.flags.show_quests = !self.flags.show_quests;
                            }
                            KeyCode::Escape => {
                                self.menu_manager.toggle_pause();
                            }
                            _ => {}
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == MouseButton::Right {
                        self.cam_ctl.process_mouse_button(
                            MouseButton::Right,
                            state == ElementState::Pressed,
                        );
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.cam_ctl.process_mouse_move(
                        &mut self.camera,
                        Vec2::new(position.x as f32, position.y as f32),
                    );
                }
                WindowEvent::RedrawRequested => {
                    let dt = (Instant::now() - self.last).as_secs_f32();
                    self.last = Instant::now();
                    self.input.clear_frame();
                    self.input.poll_gamepads();

                    self.cam_ctl.update_camera(&mut self.camera, dt);
                    renderer.update_view(&self.camera.to_render_view());

                    // Build UI frame
                    ui.begin(window);

                    let mut ui_data = UiData {
                        player_stats: &self.player_stats,
                        player_pos: self.camera.position,
                        inventory: &mut self.inventory,
                        recipe_book: Some(&self.recipes),
                        quest_log: Some(&mut self.quest_log),
                    };
                    let _ui_out = draw_ui(
                        ui.ctx(),
                        &mut self.flags,
                        &mut self.menu_manager,
                        &mut self.acc,
                        ui_data.player_stats,
                        ui_data.player_pos,
                        ui_data.inventory,
                        ui_data.recipe_book,
                        ui_data.quest_log.as_deref_mut(),
                    );

                    // Draw scene instances (simple markers)
                    self.instances.clear();
                    // Add a couple of cubes to show something on screen
                    for z in -3..=3 {
                        self.instances.push(Instance::from_pos_scale_color(
                            vec3(z as f32 * 1.5, 0.5, 0.0),
                            vec3(0.6, 1.0, 0.6),
                            [0.6, 0.7, 0.9, 1.0],
                        ));
                    }
                    renderer.update_instances(&self.instances);

                    // Render 3D + UI
                    let _size = renderer.surface_size();
                    let _ = renderer.render_with(|view, enc, dev, queue, size| {
                        ui.end_and_paint(window, view, enc, dev, queue, size);
                    });

                    window.request_redraw();
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
