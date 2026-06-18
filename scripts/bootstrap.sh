#!/bin/bash
# AstraWeave Gaming Engine - Development Environment Bootstrap Script
# This script automatically sets up the development environment for the AstraWeave project
# Supports Linux (Ubuntu/Debian, Fedora/RHEL, Arch), macOS, and Windows (via WSL)

set -euo pipefail

# Color codes for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Platform detection
detect_platform() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if command -v apt-get >/dev/null 2>&1; then
            echo "ubuntu"
        elif command -v dnf >/dev/null 2>&1; then
            echo "fedora"
        elif command -v pacman >/dev/null 2>&1; then
            echo "arch"
        else
            echo "linux-unknown"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
        echo "windows"
    else
        echo "unknown"
    fi
}

# Check if running in CI environment
is_ci() {
    [[ "${CI:-false}" == "true" || "${GITHUB_ACTIONS:-false}" == "true" ]]
}

# Install system dependencies for Ubuntu/Debian
install_ubuntu_deps() {
    log_info "Installing system dependencies for Ubuntu/Debian..."
    
    # Update package list
    if is_ci; then
        sudo apt-get update -qq
    else
        sudo apt-get update
    fi
    
    # Essential build tools
    local build_tools=(
        build-essential
        pkg-config
        cmake
        ninja-build
        curl
        git
    )
    
    # Graphics and window system libraries
    local graphics_libs=(
        libx11-dev
        libxi-dev
        libxcursor-dev
        libxrandr-dev
        libxinerama-dev
        libxkbcommon-dev
        libxkbcommon-x11-dev
        libx11-xcb-dev
        libxcb1-dev
        libxcb-randr0-dev
        libxcb-xfixes0-dev
        libxcb-shape0-dev
        libxcb-xkb-dev
        libgl1-mesa-dev
        libegl1-mesa-dev
        wayland-protocols
        libwayland-dev
    )
    
    # Audio system libraries
    local audio_libs=(
        libasound2-dev
        libpulse-dev
    )
    
    # Hardware abstraction and graphics drivers
    local system_libs=(
        libudev-dev
        mesa-vulkan-drivers
        vulkan-tools
    )
    
    # Combine all packages
    local all_packages=("${build_tools[@]}" "${graphics_libs[@]}" "${audio_libs[@]}" "${system_libs[@]}")
    
    if is_ci; then
        sudo apt-get install -y -qq "${all_packages[@]}"
    else
        sudo apt-get install -y "${all_packages[@]}"
    fi
    
    log_success "Ubuntu/Debian system dependencies installed successfully"
}

# Install system dependencies for Fedora/RHEL
install_fedora_deps() {
    log_info "Installing system dependencies for Fedora/RHEL..."
    
    # Essential build tools
    local build_tools=(
        gcc
        gcc-c++
        pkg-config
        cmake
        ninja-build
        curl
        git
    )
    
    # Development libraries
    local dev_libs=(
        libX11-devel
        libXi-devel
        libXcursor-devel
        libXrandr-devel
        libXinerama-devel
        libxkbcommon-devel
        libxkbcommon-x11-devel
        libxcb-devel
        mesa-libGL-devel
        mesa-libEGL-devel
        wayland-devel
        wayland-protocols-devel
    )
    
    # Audio libraries
    local audio_libs=(
        alsa-lib-devel
        pulseaudio-libs-devel
    )
    
    # System libraries
    local system_libs=(
        systemd-devel
        vulkan-devel
        vulkan-tools
    )
    
    local all_packages=("${build_tools[@]}" "${dev_libs[@]}" "${audio_libs[@]}" "${system_libs[@]}")
    
    sudo dnf install -y "${all_packages[@]}"
    
    log_success "Fedora/RHEL system dependencies installed successfully"
}

# Install system dependencies for Arch Linux
install_arch_deps() {
    log_info "Installing system dependencies for Arch Linux..."
    
    # Update package database
    sudo pacman -Sy
    
    local packages=(
        base-devel
        pkg-config
        cmake
        ninja
        curl
        git
        libx11
        libxi
        libxcursor
        libxrandr
        libxinerama
        libxkbcommon
        libxkbcommon-x11
        libxcb
        mesa
        wayland
        wayland-protocols
        alsa-lib
        libpulse
        systemd
        vulkan-tools
        vulkan-icd-loader
    )
    
    sudo pacman -S --needed --noconfirm "${packages[@]}"
    
    log_success "Arch Linux system dependencies installed successfully"
}

# Install system dependencies for macOS
install_macos_deps() {
    log_info "Installing system dependencies for macOS..."
    
    # Check if Homebrew is installed
    if ! command -v brew >/dev/null 2>&1; then
        log_info "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    
    # Update Homebrew
    brew update
    
    # Install packages
    local packages=(
        pkg-config
        cmake
        ninja
    )
    
    # Uninstall existing cmake to avoid conflicts (as mentioned in CI)
    brew uninstall --ignore-dependencies cmake || true
    
    brew install "${packages[@]}"
    
    # Set OpenSSL environment variables for better compatibility
    if brew --prefix openssl >/dev/null 2>&1; then
        local openssl_path
        openssl_path=$(brew --prefix openssl)
        export OPENSSL_ROOT_DIR="$openssl_path"
        export OPENSSL_LIB_DIR="$openssl_path/lib"
        export OPENSSL_INCLUDE_DIR="$openssl_path/include"
        
        log_info "OpenSSL environment variables set for compatibility"
    fi
    
    log_success "macOS system dependencies installed successfully"
}

# Install Rust toolchain
install_rust() {
    if command -v rustc >/dev/null 2>&1; then
        log_info "Rust is already installed: $(rustc --version)"
        
        # Check if we need to update to the correct version
        local current_version
        current_version=$(rustc --version | cut -d' ' -f2)
        local required_version="1.89.0"
        
        if [[ "$current_version" != "$required_version" ]]; then
            log_warning "Current Rust version ($current_version) differs from required version ($required_version)"
            log_info "The rust-toolchain.toml file will override this automatically"
        fi
    else
        log_info "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
        
        # Source cargo environment
        source "$HOME/.cargo/env"
        
        log_success "Rust installed successfully"
    fi
    
    # Ensure rustup is up to date
    rustup self update || log_warning "Could not update rustup (might be managed by system package manager)"
    
    # Install the toolchain specified in rust-toolchain.toml (this will happen automatically)
    # But let's verify the components are available
    log_info "Verifying Rust toolchain components..."
    rustup component add rustfmt clippy rust-src rust-analyzer llvm-tools-preview || true
    
    log_success "Rust toolchain verified"
}

# Install additional tools
install_additional_tools() {
    log_info "Installing additional development tools..."
    
    # Install cargo tools that are commonly used
    local cargo_tools=(
        "cargo-audit"    # Security auditing
        "cargo-deny"     # Dependency policy enforcement
        "cargo-watch"    # File watching for development
    )
    
    for tool in "${cargo_tools[@]}"; do
        if ! cargo install --list | grep -q "^$tool "; then
            log_info "Installing $tool..."
            cargo install "$tool" --locked || log_warning "Failed to install $tool"
        else
            log_info "$tool is already installed"
        fi
    done
    
    # Install sccache for build caching if not in CI
    if ! is_ci && ! command -v sccache >/dev/null 2>&1; then
        log_info "Installing sccache for build caching..."
        cargo install sccache --locked || log_warning "Failed to install sccache"
    fi
    
    log_success "Additional tools installation completed"
}

# Validate installation
validate_installation() {
    log_info "Validating installation..."
    
    local failed=0
    
    # Check Rust
    if command -v rustc >/dev/null 2>&1; then
        log_success "Rust compiler: $(rustc --version)"
    else
        log_error "Rust compiler not found"
        failed=1
    fi
    
    if command -v cargo >/dev/null 2>&1; then
        log_success "Cargo: $(cargo --version)"
    else
        log_error "Cargo not found"
        failed=1
    fi
    
    # Check essential system tools
    local tools=("pkg-config" "cmake")
    for tool in "${tools[@]}"; do
        if command -v "$tool" >/dev/null 2>&1; then
            log_success "$tool: available"
        else
            log_error "$tool not found"
            failed=1
        fi
    done
    
    # Test a simple cargo check
    log_info "Testing core package compilation..."
    if cargo check -p astraweave-core --quiet; then
        log_success "Core package compiles successfully"
    else
        log_error "Core package compilation failed"
        failed=1
    fi
    
    if [[ $failed -eq 0 ]]; then
        log_success "🎉 All validation checks passed! Your development environment is ready."
        return 0
    else
        log_error "❌ Some validation checks failed. Please review the errors above."
        return 1
    fi
}

# Print usage information
print_usage() {
    cat << EOF
AstraWeave Gaming Engine - Development Environment Bootstrap

Usage: $0 [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -s, --skip-rust         Skip Rust installation (useful if Rust is already set up)
    -t, --skip-tools        Skip additional tool installation
    -v, --validate-only     Only run validation checks
    -q, --quiet             Suppress non-error output

This script will:
1. Detect your platform (Linux distro, macOS, Windows/WSL)
2. Install required system dependencies
3. Install/verify Rust toolchain
4. Install additional development tools
5. Validate the installation

Supported platforms:
- Ubuntu/Debian Linux
- Fedora/RHEL Linux  
- Arch Linux
- macOS (with Homebrew)
- Windows (via WSL)

For more information, see: https://github.com/lazyxeon/AstraWeave
EOF
}

# Main execution function
main() {
    local skip_rust=false
    local skip_tools=false
    local validate_only=false
    local quiet=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                print_usage
                exit 0
                ;;
            -s|--skip-rust)
                skip_rust=true
                shift
                ;;
            -t|--skip-tools)
                skip_tools=true
                shift
                ;;
            -v|--validate-only)
                validate_only=true
                shift
                ;;
            -q|--quiet)
                quiet=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
    
    # Redirect output if quiet mode
    if [[ "$quiet" == "true" ]]; then
        exec > >(grep -E "(ERROR|SUCCESS)" || true)
    fi
    
    log_info "🚀 Starting AstraWeave development environment bootstrap..."
    
    # If validation only, skip to validation
    if [[ "$validate_only" == "true" ]]; then
        validate_installation
        exit $?
    fi
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    log_info "Detected platform: $platform"
    
    # Install system dependencies based on platform
    case $platform in
        ubuntu)
            install_ubuntu_deps
            ;;
        fedora)
            install_fedora_deps
            ;;
        arch)
            install_arch_deps
            ;;
        macos)
            install_macos_deps
            ;;
        windows)
            log_warning "Windows detected. Please use WSL (Windows Subsystem for Linux) for development."
            log_info "Install WSL with Ubuntu and run this script inside WSL."
            exit 1
            ;;
        *)
            log_error "Unsupported platform: $platform"
            log_error "Please install system dependencies manually and re-run with --skip-rust --validate-only"
            exit 1
            ;;
    esac
    
    # Install Rust if not skipped
    if [[ "$skip_rust" != "true" ]]; then
        install_rust
    fi
    
    # Install additional tools if not skipped
    if [[ "$skip_tools" != "true" ]]; then
        install_additional_tools
    fi
    
    # Validate installation
    validate_installation
    
    if [[ $? -eq 0 ]]; then
        log_success "🎉 Bootstrap completed successfully!"
        log_info ""
        log_info "Next steps:"
        log_info "1. Build the core components: cargo build-core"
        log_info "2. Run tests: cargo test-all"
        log_info "3. Try the hello_companion example: cargo run -p hello_companion"
        log_info "4. Check out the documentation in docs/ directory"
        log_info ""
        log_info "Happy coding! 🦀"
    else
        log_error "Bootstrap completed with errors. Please check the output above."
        exit 1
    fi
}

# Execute main function with all arguments
main "$@"