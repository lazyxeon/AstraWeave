# Troubleshooting

This guide helps you solve common issues when working with AstraWeave. Issues are organized by category with specific solutions.

## Build Issues

### Rust Toolchain Problems

#### Error: "rustc version mismatch"
```
error: rustc version doesn't match the expected version
```

**Solution:**
```bash
# Remove existing toolchain and reinstall
rustup toolchain uninstall stable
rustup toolchain install 1.89.0
rustup default 1.89.0

# Verify version
rustc --version  # Should show 1.89.0
```

#### Error: "rust-toolchain.toml not respected"
**Solution:**
```bash
# Force toolchain installation
rustup toolchain install 1.89.0
rustup override set 1.89.0

# Clean and rebuild
cargo clean
cargo build -p astraweave-core
```

### Dependency Issues

#### Error: "linker `cc` not found"
**Linux Solution:**
```bash
sudo apt-get install build-essential
```

**macOS Solution:**
```bash
xcode-select --install
```

**Windows Solution:**
Install Visual Studio with C++ build tools.

#### Error: "failed to find required package"
```
error: could not find `wgpu` in the registry
```

**Solution:**
```bash
# Update Cargo registry
cargo update

# If still failing, clear cache
rm -rf ~/.cargo/registry
cargo update
```

### Graphics Dependencies

#### Error: "Vulkan not found"
**Linux Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install mesa-vulkan-drivers vulkan-tools

# Arch Linux  
sudo pacman -S vulkan-devel mesa

# Fedora
sudo dnf install vulkan-devel mesa-dri-drivers

# Verify Vulkan
vulkaninfo | head -20
```

**Windows Solution:**
Update your graphics drivers from manufacturer website:
- NVIDIA: Download latest drivers
- AMD: Download Adrenalin drivers
- Intel: Download latest graphics drivers

**macOS Solution:**
Vulkan support requires MoltenVK:
```bash
brew install molten-vk
```

#### Error: "wgpu adapter not found"
```
thread 'main' panicked at 'No suitable graphics adapter found'
```

**Solutions:**
1. **Check GPU compatibility:**
   ```bash
   # Linux: Check Vulkan support
   vulkaninfo
   
   # Should show at least one device
   ```

2. **Force software rendering:**
   ```bash
   export WGPU_BACKEND=gl
   cargo run -p hello_companion
   ```

3. **Update graphics drivers**

### Audio Dependencies

#### Error: "ALSA lib errors" (Linux)
```
ALSA lib pcm_dmix.c:1089:(snd_pcm_dmix_open) unable to open slave
```

**Solution:**
```bash
# Install audio libraries
sudo apt-get install libasound2-dev libpulse-dev

# Check audio devices
aplay -l

# If no devices, check PulseAudio
pulseaudio --check
```

#### Error: "No audio output device found"
**Linux Solution:**
```bash
# Restart audio services
systemctl --user restart pulseaudio
```

**Windows Solution:**
Check that Windows Audio service is running in Services.

**macOS Solution:**
Usually works out of the box. Check System Preferences > Sound.

### Example Compilation Issues

#### Error: "examples fail to compile"
Many examples have known compilation issues. Use only the working examples:

**Working Examples:**
```bash
cargo build -p hello_companion      # ✅ Works (expected panic)
cargo build -p ipc_loopback        # ✅ Should work  
cargo test -p astraweave-input      # ✅ Tests pass
```

**Known Broken Examples:**
```bash
# ❌ These have compilation issues:
# cargo build -p debug_toolkit_demo  # egui/winit/renderer API mismatches
# cargo build -p aw_editor           # eframe/glutin sync/send trait issues  
# cargo build -p aw_debug            # eframe API mismatches
# cargo build -p visual_3d           # clippy deny-level errors
# cargo build -p navmesh_demo        # clippy deny-level errors (approx_constant)
# cargo build -p physics_demo3d      # clippy deny-level errors (approx_constant)
```

**Workaround:**
Focus on the working core components and use the provided aliases:
```bash
cargo build -p astraweave-core -p astraweave-ecs -p astraweave-math -p astraweave-ai  # Build core components only
cargo check --workspace             # Check workspace (130/130 members, 0 errors)
cargo clippy --workspace --all-features -- -D warnings  # Run clippy across the workspace
```

## Runtime Issues

### Graphics Issues

#### Error: "Validation error in wgpu"
```
wgpu validation error: Buffer usage VERTEX | COPY_DST is not valid
```

**Solutions:**
1. **Update graphics drivers**
2. **Use older wgpu backend:**
   ```bash
   export WGPU_BACKEND=vulkan  # or gl, metal, dx12
   ```
3. **Reduce graphics settings in your code**

#### Error: "Surface creation failed"
```
Error creating surface: SurfaceError(OutOfMemory)
```

**Solutions:**
1. **Reduce window size:**
   ```rust
   // In your window configuration
   .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
   ```
2. **Lower graphics quality settings**
3. **Check available VRAM:**
   ```bash
   # Linux
   nvidia-smi  # for NVIDIA
   radeontop   # for AMD
   ```

### AI Model Issues

#### Error: "AI model not found"
```
Error: Could not load AI model 'companion-7b'
```

**Solutions:**
1. **Use mock AI for testing:**
   ```rust
   // In your configuration
   ai_agent.ai_model = AIModel::Mock;
   ```

2. **Download required models:**
   ```bash
   # Models not included in repository
   # Use mock or implement your own model loader
   ```

3. **Configure model path:**
   ```rust
   ai_agent.ai_model = AIModel::Local("path/to/your/model".to_string());
   ```

### Performance Issues

#### Issue: "Low FPS / Stuttering"
**Diagnosis:**
```bash
# Always use release builds for performance testing
cargo run -p hello_companion --release

# Check if running in debug mode
cargo run -p hello_companion  # This is debug mode - will be slow
```

**Solutions:**
1. **Always use release builds:**
   ```bash
   cargo build --release
   cargo run --release -p your_example
   ```

2. **Check system resources:**
   ```bash
   # Linux
   htop
   
   # Monitor GPU usage
   nvidia-smi  # NVIDIA
   radeontop   # AMD
   ```

3. **Reduce AI complexity:**
   ```rust
   // Lower AI planning frequency
   ai_agent.planning_interval = Duration::from_millis(1000); // Instead of 500
   
   // Reduce perception range
   ai_agent.perception_range = 5.0; // Instead of 10.0
   ```

#### Issue: "High memory usage"
**Diagnosis:**
```bash
# Check memory usage
cargo run --release -p hello_companion &
ps aux | grep hello_companion
```

**Solutions:**
1. **Limit AI memory:**
   ```rust
   ai_memory.max_episodic_memories = 100;
   ai_memory.max_working_memory = 10;
   ```

2. **Use memory profiling:**
   ```bash
   # Install valgrind (Linux)
   sudo apt-get install valgrind
   valgrind --tool=massif cargo run --release -p hello_companion
   ```

### Network Issues

#### Error: "Connection refused" (multiplayer examples)
```
Error: Connection refused (os error 111)
```

**Solutions:**
1. **Check if server is running:**
   ```bash
   # Terminal 1 - Start server first
   cargo run -p coop_server --release
   
   # Terminal 2 - Then client
   cargo run -p coop_client --release
   ```

2. **Check firewall settings:**
   ```bash
   # Linux: Check if port is open
   sudo ufw status
   
   # Allow port if needed
   sudo ufw allow 8080
   ```

3. **Use localhost:**
   ```rust
   // Make sure client connects to localhost
   let server_addr = "127.0.0.1:8080";
   ```

## Development Issues

### IDE Problems

#### Issue: "rust-analyzer not working"
**Solution:**
```bash
# Restart rust-analyzer
# In VS Code: Ctrl+Shift+P > "Rust Analyzer: Restart Server"

# Or reinstall
rustup component add rust-analyzer
```

#### Issue: "Slow code completion"
**Solutions:**
1. **Exclude target directory from indexing**
2. **Reduce project scope:**
   ```json
   // In VS Code settings.json
   {
     "rust-analyzer.cargo.allFeatures": false,
     "rust-analyzer.checkOnSave.allFeatures": false
   }
   ```

### Testing Issues

#### Error: "Tests hanging"
```bash
cargo test -p astraweave-input
# Hangs indefinitely
```

**Solutions:**
1. **Run with timeout:**
   ```bash
   timeout 30s cargo test -p astraweave-input
   ```

2. **Run single test:**
   ```bash
   cargo test -p astraweave-input test_input_system
   ```

3. **Use single-threaded execution:**
   ```bash
   cargo test -p astraweave-input -- --test-threads=1
   ```

#### Error: "Test failures due to timing"
```
thread 'ai_planning_test' panicked at 'assertion failed: plan.is_some()'
```

**Solution:**
Tests involving AI may have timing dependencies:
```rust
// Add delays in tests
#[test]
fn ai_planning_test() {
    let mut world = create_test_world();
    world.step(); // Let one frame pass
    
    std::thread::sleep(Duration::from_millis(100)); // Give AI time to plan
    
    let plan = world.get_ai_plan();
    assert!(plan.is_some());
}
```

## Platform-Specific Issues

### Linux Issues

#### Issue: "Wayland compatibility"
Some features may not work correctly on Wayland:
```bash
# Force X11 if needed
export WAYLAND_DISPLAY=""
export DISPLAY=:0

# Or force Wayland if X11 is causing issues
export DISPLAY=""
```

#### Issue: "Audio permission denied"
```bash
# Add user to audio group
sudo usermod -a -G audio $USER

# Restart session or reboot
```

### macOS Issues

#### Issue: "Code signing errors"
```
error: codesign failed with exit code 1
```

**Solution:**
```bash
# For development, disable code signing
export MACOSX_DEPLOYMENT_TARGET=11.0

# Or sign manually
codesign --force --deep --sign - target/release/hello_companion
```

#### Issue: "Metal validation errors"
Use software rendering if Metal causes issues:
```bash
export WGPU_BACKEND=gl
cargo run -p hello_companion --release
```

### Windows Issues

#### Issue: "MSVC runtime missing"
Install Microsoft Visual C++ Redistributable:
- Download from Microsoft's website
- Or install Visual Studio with C++ tools

#### Issue: "Antivirus blocking execution"
Add exclusions for:
- Project directory
- `%USERPROFILE%\.cargo`
- `target\` directory

#### Issue: "Path too long errors"
```bash
# Enable long paths in Windows
# Run as Administrator in PowerShell:
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

## Getting Help

### Before Asking for Help

1. **Check this troubleshooting guide**
2. **Verify your setup:**
   ```bash
   rustc --version  # Should be 1.89.0
   cargo --version
   ```
3. **Try with minimal example:**
   ```bash
   cargo run -p hello_companion --release
   ```
4. **Check system dependencies**

### Information to Include

When reporting issues, include:

1. **System information:**
   ```bash
   # Linux
   uname -a
   lsb_release -a
   
   # macOS
   sw_vers
   
   # Windows
   systeminfo
   ```

2. **Rust version:**
   ```bash
   rustc --version
   cargo --version
   ```

3. **Graphics information:**
   ```bash
   # Linux
   lspci | grep VGA
   vulkaninfo | head -20
   
   # Windows
   dxdiag
   
   # macOS
   system_profiler SPDisplaysDataType
   ```

4. **Full error output:**
   ```bash
   # Include full error with backtrace
   RUST_BACKTRACE=full cargo run -p hello_companion 2>&1 | tee error.log
   ```

5. **Steps to reproduce**

### Community Resources

- **GitHub Issues**: For bug reports and feature requests
- **Discussions**: For questions and general help
- **Matrix/Discord**: For real-time community support (if available)

### Known Limitations

#### Current Development State
AstraWeave is under active development. Known limitations:

1. **Many examples don't compile** due to API evolution
2. **Limited AI model integration** - mostly uses mock AI
3. **Graphics API compatibility** - some newer GPU features not supported
4. **Documentation gaps** - some advanced features lack documentation

#### Workarounds
1. **Focus on working examples** (hello_companion, core components)
2. **Use mock AI** for learning the architecture
3. **Stick to stable APIs** in core crates
4. **Contribute fixes** for broken examples

---

*If you're still having issues after trying these solutions, please create an issue on GitHub with the requested information. The community is here to help!*