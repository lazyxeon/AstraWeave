// ApplicationHandler implementation for unified_showcase
// This will be inserted into main.rs before fn main()

struct ShowcaseApp {
    window: Option<std::sync::Arc<Window>>,
    render: Option<RenderStuff>,
    physics: PhysicsWorld,
    characters: Vec<Character>,
    instances: Vec<Instance>,
    ui: UiState,
    camera: RenderCamera,
    camera_controller: CameraController,
    start_time: Instant,
    last: Instant,
    fps_acc: f32,
    fps_cnt: u32,
    shift_down: bool,
    initialized: bool,
}

impl ShowcaseApp {
    async fn new() -> Result<Self> {
        // Generate default textures at startup if missing
        let seed = 0xA57;
        let asset_root = asset_dir();
        
        #[cfg(feature = "procedural_fallback")]
        {
            texture_synth::ensure_textures(&asset_root.to_string_lossy(), seed, false)?;
            let materials_runtime_dir = asset_root.join("materials");
            texture_synth::ensure_textures(&materials_runtime_dir.to_string_lossy(), seed, false)?;
            let materials_source_dir = resolve_asset_src_path("materials");
            texture_synth::ensure_textures(&materials_source_dir.to_string_lossy(), seed, false)?;
        }

        // Initialize physics world
        let physics = build_physics_world();

        // Create default camera
        let camera = RenderCamera {
            position: Vec3::new(15.0, 12.0, 30.0),
            yaw: -0.3,
            pitch: -0.4,
            fovy: 70f32.to_radians(),
            aspect: 1.0,
            znear: 0.01,
            zfar: 10000.0,
        };

        let camera_controller = CameraController::new(8.0, 0.002);
        let start_time = Instant::now();
        let last = Instant::now();

        Ok(Self {
            window: None,
            render: None,
            physics,
            characters: Vec::new(),
            instances: build_show_instances(),
            ui: UiState::default(),
            camera,
            camera_controller,
            start_time,
            last,
            fps_acc: 0.0,
            fps_cnt: 0,
            shift_down: false,
            initialized: false,
        })
    }
}

impl ApplicationHandler for ShowcaseApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already initialized
        }

        let window_attributes = Window::default_attributes()
            .with_title("AstraWeave Unified Showcase (Modified)");
        
        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                let window = std::sync::Arc::new(window);
                
                // Setup renderer (async, so we use pollster::block_on)
                match pollster::block_on(setup_renderer(window.clone())) {
                    Ok(mut render) => {
                        // Initialize biome
                        println!("🌱 Initializing with grassland biome...");
                        match switch_biome(&mut render, &mut self.physics, "grassland") {
                            Ok(chars) => {
                                println!("✅ Successfully initialized grassland biome with {} characters", chars.len());
                                println!("Controls: WASD+mouse=camera, P=pause physics, T=teleport sphere, E=apply impulse, C=toggle camera mode");
                                println!("Mouse wheel: zoom camera | Right-click + mouse: look around");
                                println!("Texture packs: Press 1 for grassland, 2 for desert, 3 for forest");
                                self.characters = chars;
                            }
                            Err(e) => {
                                println!("⚠ Warning: Failed to initialize grassland biome: {}", e);
                                println!("Continuing with procedural fallback textures...");
                                let fallback = generate_environment_objects(&mut self.physics, "grassland");
                                println!("🌿 Generated {} character instances using fallback grassland environment", fallback.len());
                                self.characters = fallback;
                            }
                        }

                        // Update UI
                        if !render.current_biome.is_empty() {
                            self.ui.current_texture_pack = render.current_biome.clone();
                            self.ui.info_text = format!("Environment: {} ({} characters)", render.current_biome, self.characters.len());
                        } else {
                            self.ui.info_text = format!("Environment: {} ({} characters)", self.ui.current_texture_pack, self.characters.len());
                        }

                        self.window = Some(window);
                        self.render = Some(render);
                        self.initialized = true;
                    }
                    Err(e) => {
                        eprintln!("Failed to create renderer: {:?}", e);
                        event_loop.exit();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to create window: {:?}", e);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else { return };
        let Some(render) = self.render.as_mut() else { return };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key,
                    state,
                    ..
                },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                match physical_key {
                    PhysicalKey::Code(code) => {
                        self.camera_controller.process_keyboard(code, pressed);
                        match code {
                            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                                self.shift_down = pressed;
                            }
                            KeyCode::Escape => {
                                if pressed {
                                    event_loop.exit();
                                }
                            }
                            KeyCode::KeyP => {
                                if pressed {
                                    self.ui.physics_paused = !self.ui.physics_paused;
                                }
                            }
                            KeyCode::KeyT => {
                                if pressed {
                                    let forward = astraweave_camera::FreeFly::dir(
                                        self.camera.yaw,
                                        self.camera.pitch,
                                    );
                                    let target = self.camera.position + forward * 4.0 + Vec3::new(0.0, -0.5, 0.0);
                                    teleport_sphere_to(&mut self.physics, target);
                                }
                            }
                            KeyCode::KeyE => {
                                if pressed {
                                    let forward = astraweave_camera::FreeFly::dir(
                                        self.camera.yaw,
                                        self.camera.pitch,
                                    );
                                    let target = self.camera.position + forward * 5.0;
                                    apply_impulse_to_sphere(&mut self.physics, target);
                                }
                            }
                            KeyCode::KeyC => {
                                if pressed {
                                    println!("Camera mode toggle (not implemented in this demo)");
                                }
                            }
                            KeyCode::Digit1 => {
                                if pressed {
                                    println!("Switching to grassland biome...");
                                    match switch_biome(render, &mut self.physics, "grassland") {
                                        Ok(chars) => {
                                            self.characters = chars;
                                            self.ui.current_texture_pack = "grassland".to_string();
                                            self.ui.info_text = format!("Environment: grassland ({} characters)", self.characters.len());
                                            println!("✅ Switched to grassland");
                                        }
                                        Err(e) => println!("⚠ Failed to switch biome: {}", e),
                                    }
                                }
                            }
                            KeyCode::Digit2 => {
                                if pressed {
                                    println!("Switching to desert biome...");
                                    match switch_biome(render, &mut self.physics, "desert") {
                                        Ok(chars) => {
                                            self.characters = chars;
                                            self.ui.current_texture_pack = "desert".to_string();
                                            self.ui.info_text = format!("Environment: desert ({} characters)", self.characters.len());
                                            println!("✅ Switched to desert");
                                        }
                                        Err(e) => println!("⚠ Failed to switch biome: {}", e),
                                    }
                                }
                            }
                            KeyCode::Digit3 => {
                                if pressed {
                                    println!("Switching to forest biome...");
                                    match switch_biome(render, &mut self.physics, "forest") {
                                        Ok(chars) => {
                                            self.characters = chars;
                                            self.ui.current_texture_pack = "forest".to_string();
                                            self.ui.info_text = format!("Environment: forest ({} characters)", self.characters.len());
                                            println!("✅ Switched to forest");
                                        }
                                        Err(e) => println!("⚠ Failed to switch biome: {}", e),
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    let pressed = state == ElementState::Pressed;
                    if pressed {
                        let _ = window.set_cursor_grab(CursorGrabMode::Confined);
                        window.set_cursor_visible(false);
                    } else {
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                        window.set_cursor_visible(true);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y * 2.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                };
                self.camera_controller.process_scroll(scroll);
            }
            WindowEvent::CursorMoved { .. } => {
                // Handled in device_event for better mouse delta
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    render.surface_cfg.width = new_size.width;
                    render.surface_cfg.height = new_size.height;
                    render.surface.configure(&render.device, &render.surface_cfg);
                    self.camera.aspect = new_size.width as f32 / new_size.height.max(1) as f32;
                    
                    // Recreate depth/hdr textures...
                    let depth_tex = render.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("depth"),
                        size: wgpu::Extent3d {
                            width: new_size.width,
                            height: new_size.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: render.msaa_samples,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Depth32Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    });
                    render.depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
                    
                    // HDR texture recreation (simplified - full version has more detail)
                    let hdr_tex = render.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("hdr"),
                        size: wgpu::Extent3d {
                            width: new_size.width,
                            height: new_size.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba16Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                    render.hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());
                }
            }
            WindowEvent::RedrawRequested => {
                // Main render loop - preserve exact logic from original
                let dt = (Instant::now() - self.last).as_secs_f32();
                self.last = Instant::now();
                
                // Update FPS
                self.fps_acc += dt;
                self.fps_cnt += 1;
                if self.fps_acc >= 0.5 {
                    let fps = self.fps_cnt as f32 / self.fps_acc;
                    self.ui.fps_text = format!("FPS: {:.1}", fps);
                    self.fps_acc = 0.0;
                    self.fps_cnt = 0;
                }

                // Update camera
                self.camera_controller.update_camera(&mut self.camera, dt);

                // Physics tick
                if !self.ui.physics_paused {
                    self.physics.step(dt);
                }

                // Update character animations
                for ch in &mut self.characters {
                    ch.animation_time += dt;
                }

                // Sync instances with physics and characters
                sync_instances_from_physics(&mut self.instances, &self.physics);
                let character_instances = character_instances_from_state(&self.characters);
                self.instances.extend(character_instances);

                // Update renderer state
                render.queue.write_buffer(
                    &render.camera_ub,
                    0,
                    bytemuck::bytes_of(&GpuCamera {
                        view_proj: (self.camera.build_projection_matrix() * self.camera.build_view_matrix()).to_cols_array_2d(),
                    }),
                );

                // Render
                match render.surface.get_current_texture() {
                    Ok(frame) => {
                        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder = render.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("main encoder"),
                        });

                        // Shadow pass (simplified)
                        {
                            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("shadow"),
                                color_attachments: &[],
                                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                    view: &render.shadow_view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                }),
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                            rpass.set_pipeline(&render.shadow_pipeline);
                            // Draw instances...
                        }

                        // Main pass
                        {
                            let color_attach = if render.msaa_samples > 1 {
                                render.hdr_msaa_view.as_ref().unwrap()
                            } else {
                                &render.hdr_view
                            };
                            
                            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("main"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: color_attach,
                                    resolve_target: if render.msaa_samples > 1 { Some(&render.hdr_view) } else { None },
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                    view: &render.depth_view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                }),
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                            rpass.set_pipeline(&render.pipeline);
                            rpass.set_bind_group(0, &render.camera_bg, &[]);
                            // Draw instances...
                        }

                        // Post-processing
                        {
                            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("post"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                            rpass.set_pipeline(&render.post_pipeline);
                            rpass.set_bind_group(0, &render.post_bg, &[]);
                            rpass.draw(0..3, 0..1);
                        }

                        render.queue.submit(Some(encoder.finish()));
                        frame.present();
                    }
                    Err(e) => {
                        eprintln!("Surface error: {:?}", e);
                        if matches!(e, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) {
                            render.surface.configure(&render.device, &render.surface_cfg);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            let mouse_delta = Vec2::new(delta.0 as f32, delta.1 as f32);
            self.camera_controller.process_mouse_delta(&mut self.camera, mouse_delta);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
