# System Requirements

This page outlines the hardware and software requirements for developing and running AstraWeave applications.

## Table of Contents

- [Minimum Requirements](#minimum-requirements)
- [Recommended Requirements](#recommended-requirements)
- [GPU Requirements](#gpu-requirements)
- [Software Dependencies](#software-dependencies)
- [Platform-Specific Notes](#platform-specific-notes)

## Minimum Requirements

These are the bare minimum specifications to run AstraWeave applications:

### Hardware

| Component | Specification |
|-----------|---------------|
| **CPU** | Quad-core processor (Intel i5-6600K / AMD Ryzen 3 1300X or equivalent) |
| **RAM** | 8 GB |
| **GPU** | NVIDIA GTX 1060 (6GB) / AMD RX 580 (8GB) / Intel Arc A380 |
| **VRAM** | 4 GB |
| **Storage** | 2 GB available space (SSD recommended) |
| **Display** | 1920x1080 resolution |

```admonish warning
Minimum requirements may result in reduced performance. For optimal experience, see recommended specifications.
```

### Operating System

- **Windows**: Windows 10 (64-bit) version 1909 or later
- **Linux**: Any modern distribution with kernel 5.4+ (Ubuntu 20.04, Fedora 33, etc.)
- **macOS**: macOS 10.15 (Catalina) or later

## Recommended Requirements

For optimal performance and development experience:

### Hardware

| Component | Specification |
|-----------|---------------|
| **CPU** | 8-core processor (Intel i7-9700K / AMD Ryzen 7 3700X or better) |
| **RAM** | 16 GB or more |
| **GPU** | NVIDIA RTX 3060 / AMD RX 6700 XT / Intel Arc A750 or better |
| **VRAM** | 8 GB or more |
| **Storage** | 10 GB available space on NVMe SSD |
| **Display** | 2560x1440 or higher resolution |

### Operating System

- **Windows**: Windows 11 (64-bit)
- **Linux**: Ubuntu 22.04 LTS / Fedora 38 or later
- **macOS**: macOS 13 (Ventura) or later

```admonish tip
More RAM and VRAM allow for larger scenes, more AI companions, and higher-quality assets.
```

## GPU Requirements

AstraWeave is GPU-accelerated and requires modern graphics hardware with compute shader support.

### Graphics API Support

Your GPU must support one of the following:

| Platform | Graphics API | Minimum Version |
|----------|--------------|-----------------|
| **Windows** | DirectX 12 | Feature Level 12.0 |
| **Windows/Linux** | Vulkan | 1.2 |
| **macOS** | Metal | Metal 2 |

### Supported GPUs

#### NVIDIA

**Minimum**:
- GTX 1060 (6GB)
- GTX 1070
- GTX 1660 Ti

**Recommended**:
- RTX 3060 or better
- RTX 4060 or better
- RTX A2000 or better (workstation)

**Optimal**:
- RTX 4070 Ti or better
- RTX 4080 / 4090
- RTX A4000 or better (workstation)

#### AMD

**Minimum**:
- RX 580 (8GB)
- RX 5500 XT (8GB)
- RX 6600

**Recommended**:
- RX 6700 XT or better
- RX 7600 XT or better

**Optimal**:
- RX 7800 XT or better
- RX 7900 XTX
- Radeon Pro W6800 or better (workstation)

#### Intel

**Minimum**:
- Arc A380

**Recommended**:
- Arc A750 or better
- Arc A770

**Optimal**:
- Arc A770 (16GB)

```admonish note
Intel Arc GPUs require the latest drivers for optimal performance. Visit [Intel's driver page](https://www.intel.com/content/www/us/en/download/726609/intel-arc-graphics-windows-dch-driver.html) for updates.
```

### Integrated Graphics

Integrated GPUs are **not recommended** but may work for basic development:

- Intel Iris Xe (11th gen or later)
- AMD Radeon Graphics (Ryzen 5000 series or later)

```admonish warning
Integrated graphics will have significant performance limitations. Dedicated GPU strongly recommended.
```

### Verifying GPU Support

#### Windows

```powershell
# Check DirectX version
dxdiag

# Check GPU with PowerShell
Get-WmiObject Win32_VideoController | Select-Object Name, DriverVersion
```

#### Linux

```bash
# Check Vulkan support
vulkaninfo | grep "deviceName"

# Check GPU details
lspci | grep -i vga
nvidia-smi  # For NVIDIA GPUs
```

#### macOS

```bash
# Check Metal support
system_profiler SPDisplaysDataType
```

## Software Dependencies

### Development Dependencies

#### Required

- **Rust**: Version 1.75.0 or later
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

## Performance Expectations

### Development Workloads

| Specification | Compile Time (full rebuild) | Test Suite Runtime | Editor Performance |
|---------------|----------------------------|--------------------|--------------------|
| **Minimum** | 5-10 minutes | 2-3 minutes | Playable (30+ FPS) |
| **Recommended** | 2-4 minutes | 1-2 minutes | Smooth (60+ FPS) |
| **Optimal** | 1-2 minutes | <1 minute | Excellent (120+ FPS) |

### Runtime Performance

Example scene: 10 AI companions with full perception and emotion systems

| Specification | Average FPS | 1% Low FPS | Max Companions |
|---------------|-------------|------------|----------------|
| **Minimum** | 30-45 | 25-30 | 10-20 |
| **Recommended** | 60-90 | 50-60 | 50-100 |
| **Optimal** | 120-144+ | 90-100 | 200-500+ |

```admonish note
Performance varies based on scene complexity, AI behavior count, and graphics settings.
```

## Checking Your System

Run this command to verify your setup:

```bash
# Clone and run the system check example
git clone https://github.com/verdentlabs/astraweave.git
cd astraweave
cargo run --example system_check
```

The system check will report:
- Rust version
- GPU capabilities
- Graphics API support
- Available memory
- Recommended settings

```admonish success
If all checks pass, your system is ready for AstraWeave development!
```

## Upgrading Your System

If your system doesn't meet requirements:

### Priority Upgrades

1. **GPU**: Biggest impact on runtime performance
2. **RAM**: Enables larger scenes and faster compilation
3. **SSD**: Dramatically reduces build times
4. **CPU**: Improves compilation speed and AI performance

### Budget Recommendations

- **Entry Level** ($300-500): Used RTX 3060 or RX 6700 XT + 16GB RAM
- **Mid Range** ($800-1200): RTX 4060 Ti or RX 7700 XT + 32GB RAM + NVMe SSD
- **High End** ($2000+): RTX 4080 or RX 7900 XTX + 64GB RAM + Fast CPU

## Next Steps

Once your system meets the requirements:

1. [Install AstraWeave](installation.md)
2. [Build from source](../dev/building.md)
3. [Create your first companion](first-companion.md)

If you have questions about requirements, join our [Discord community](https://discord.gg/astraweave).
