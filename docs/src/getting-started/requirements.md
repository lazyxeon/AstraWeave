# System Requirements

This page outlines the hardware and software requirements for developing and running AstraWeave applications.

## Table of Contents

- [What this repository actually pins](#what-this-repository-actually-pins)
- [Software Dependencies](#software-dependencies)
- [Platform-Specific Notes](#platform-specific-notes)

## What this repository actually pins

Rust is **1.89.0**, pinned in `rust-toolchain.toml`. Replay hashes are compared on the GitHub-hosted Linux, Windows, and macOS runners named in the determinism workflow. That is not a minimum OS version and not a claim that every windowed example has been run on all three.

No measured minimum CPU, RAM, or GPU is published here. The frame-budget sample and its adapter are on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page. The default `hello_companion` run is headless. Windowed crates need a device wgpu can open (Vulkan, Metal, or Direct3D 12, depending on the OS).

Older copies of this page listed specific cards (GTX 1060, RTX 3060, and others) and OS point releases. Those lists were not measured for this tree and are removed.

## Confirm the install

Headless companion, no GPU:

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
rustc --version
cargo run -p hello_companion --release
```

`rustc --version` should print 1.89.0. The walkthrough of that binary is [Your first companion](./first-companion.md).

Windowed binaries (`shadow_csm_demo`, `aravath_greybox`, `physics_demo3d`, `fluids_demo`) need the graphics packages listed in the [quick start](./quick-start.md). [`docs/current/ARAVATH_GREYBOX_PROGRESS.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ARAVATH_GREYBOX_PROGRESS.md) records that `c++` on that machine is Clang 18, which selects a GCC 14 toolchain whose libstdc++ headers are not installed. Cargo builds of `basis-universal-sys` (pulled in by `astraweave-render`) need `CXX=g++`.

## Software Dependencies

### Development Dependencies

#### Required

- **Rust**: 1.89.0, pinned in `rust-toolchain.toml`
  - Install via [rustup](https://rustup.rs/)
  - Verify: `rustc --version`

- **Git**: Any recent version
  - Install from [git-scm.com](https://git-scm.com/)
  - Verify: `git --version`

#### Platform-Specific

##### Windows

- **Visual Studio Build Tools 2022** or **Visual Studio 2022**
  - Required for MSVC linker
  - Download: [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/)
  - Select "Desktop development with C++" workload

- **Vulkan SDK** (optional, for Vulkan backend)
  - Download: [LunarG Vulkan SDK](https://vulkan.lunarg.com/)

##### Linux

**Debian/Ubuntu**:
```bash
sudo apt install -y \
  build-essential \
  pkg-config \
  libx11-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxi-dev \
  libasound2-dev \
  libudev-dev \
  vulkan-tools \
  libvulkan-dev
```

**Fedora/RHEL**:
```bash
sudo dnf install -y \
  gcc gcc-c++ \
  pkg-config \
  libX11-devel \
  libXcursor-devel \
  libXrandr-devel \
  libXi-devel \
  alsa-lib-devel \
  systemd-devel \
  vulkan-tools \
  vulkan-loader-devel
```

**Arch Linux**:
```bash
sudo pacman -S --needed \
  base-devel \
  libx11 \
  libxcursor \
  libxrandr \
  libxi \
  alsa-lib \
  vulkan-tools \
  vulkan-headers
```

##### macOS

- **Xcode Command Line Tools**
  ```bash
  xcode-select --install
  ```

- **Homebrew** (recommended for package management)
  ```bash
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  ```

### GPU Drivers

Always use the latest drivers for optimal performance:

#### NVIDIA

- Download from [NVIDIA Driver Downloads](https://www.nvidia.com/download/index.aspx)
- Linux: Use distribution's package manager or NVIDIA's `.run` installer
- Minimum driver version: 525.60.11 (Linux) / 528.24 (Windows)

#### AMD

- Windows: [AMD Software Adrenalin Edition](https://www.amd.com/en/support)
- Linux: Mesa drivers (usually pre-installed)
  ```bash
  # Ubuntu/Debian
  sudo apt install mesa-vulkan-drivers
  
  # Fedora
  sudo dnf install mesa-vulkan-drivers
  ```

#### Intel

- [Intel Graphics Drivers](https://www.intel.com/content/www/us/en/download-center/home.html)
- Linux: Mesa drivers (usually pre-installed)

## Platform-Specific Notes

### Windows

- **Windows Defender**: May slow initial builds. Add exclusion for project directory:
  ```
  Settings > Windows Security > Virus & threat protection > Exclusion settings
  ```

- **Long Path Support**: Enable for deep project structures:
  ```powershell
  # Run as Administrator
  New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
    -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
  ```

- **Power Settings**: Use "High Performance" or "Ultimate Performance" power plan for best results

### Linux

- **GPU Permissions**: Ensure user is in the `video` group:
  ```bash
  sudo usermod -a -G video $USER
  # Log out and back in
  ```

- **Wayland**: Some features work better with X11. Switch if experiencing issues:
  ```bash
  # Edit /etc/gdm3/custom.conf (Ubuntu/Debian)
  # Uncomment: WaylandEnable=false
  ```

- **Memory Limits**: Increase limits for large projects:
  ```bash
  # Add to ~/.bashrc
  ulimit -n 4096
  ```

### macOS

- **Metal Support**: Requires macOS 10.15+ and Metal-compatible GPU
  - Check: `system_profiler SPDisplaysDataType | grep Metal`

- **Apple Silicon (M1/M2/M3)**: Fully supported with native ARM builds
  ```bash
  # Verify ARM toolchain
  rustc --version --verbose | grep host
  # Should show: aarch64-apple-darwin
  ```

- **Rosetta 2**: Not required for M1/M2/M3 Macs (native ARM support)

- **Security Settings**: May need to allow apps in System Preferences on first run

## Check the toolchain

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
rustc --version
cargo run -p hello_companion --release
```

`rustc --version` inside the repo should be 1.89.0. There is no `system_check` example. Compile times and FPS tables that used to sit here were not measured for this tree.

## Next steps

1. [Install](installation.md)
2. [Build from source](../dev/building.md)
3. [First companion](first-companion.md)

Questions go to [GitHub issues](https://github.com/lazyxeon/AstraWeave/issues).
