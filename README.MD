# PentOS
A hobby x86_64 operating system written in Rust, featuring a UEFI bootloader, an async kernel, a custom build system, and much more to come.

This is the 5th iteration of [HeliumOS](https://github.com/yarml/HeliumOS), hence the name.

**WIP Developer documentation:** [pentos.yarml.com](https://pentos.yarml.com)

## Table of Contents
- [Dependencies](#dependencies)
- [Building](#building)
- [Running in QEMU](#running-in-qemu)
- [Creating Disk Images](#creating-disk-images)
- [Building the Documentation](#building-the-documentation)
- [Build Configuration](#build-configuration)
- [Runtime Configuration](#runtime-configuration)
- [Acknowledgements](#acknowledgements)

## Dependencies

### Base Dependencies
The following tools must be installed manually before building:

- `cargo` / `rustup`
- `nasm`

### Optional Dependencies
| Tool | Purpose |
|-|-|
| `qemu-system-x86_64` | Required for running PentOS in a VM (`cargo dev run` & `cargo dev debug`). |
| `gdb`, `tmux`, `tee`, & `socat` | Required for debugging the kernel alongside QEMU (`cargo dev debug`). |

### Automatic Dependencies
PentOS depends on additional tools and assets, but those not listed above are automatically managed by the build system itself.

A comprehensive list of all Rust build dependencies can be found in `Cargo.toml`.

OVMF and the console font are downloaded and managed automatically by chef the first time they are needed.

## Building
### Building the Bootloader and Kernel
```sh
# Release build (default)
cargo dev build bootloader
cargo dev build kernel

# Debug build
cargo dev build -p debug bootloader
cargo dev build -p debug kernel
```

Output locations:
- Bootloader: `target/uefi/{debug,release}/bootloader.efi`
- Kernel: `target/kernel/{debug,release}/kernel`

### Building a User Package
```sh
cargo dev build pkg <name>
cargo dev build -p debug pkg <name>
```

Output location: `target/user/{debug,release}/<name>`

### Linting
```sh
cargo dev lint
```

### Running Unit Tests
```sh
cargo dev test
```

## Running in QEMU
These commands build and generate everything automatically before launching QEMU.

```sh
# Run release build
cargo dev run

# Run debug build (no debugger attached)
cargo dev run -p debug

# Run debug build with GDB + tmux (attaches GDB and a QEMU monitor)
cargo dev debug
```

The QEMU configuration is controlled via `build.toml` (and overrides in `.build.toml`). The defaults are:
- 12 SMP cores
- 8 GB of RAM
- Resolution set to 1920x1080

## Creating Disk Images

### Flat (directory layout)
Creates a directory tree representing the filesystem contents.

```sh
# Release
cargo dev generate flat

# Debug
cargo dev generate -p debug flat
```

You can also generate individual partitions:

```sh
cargo dev generate flat boot
cargo dev generate flat system
```

### Raw `.img`
Creates a raw GPT disk image containing both the boot and system partitions.

```sh
# Release
cargo dev generate img disk

# Debug
cargo dev generate -p debug img disk
```

Individual partition images:

```sh
cargo dev generate img boot
cargo dev generate img system
```

Output: `.build/{debug,release}/img/pentos.img`

Optional page/frame size flags (defaults: `--page-size 512`, `--frame-size 4096`):

```sh
cargo dev generate -p release img disk --page-size 512 --frame-size 4096
```

## Building the Documentation
```sh
cargo dev doc
```

This generates rustdoc for the entire workspace under `target/doc/`.

The hosted version of the developer documentation is available at [pentos.yarml.com](https://pentos.yarml.com).

## Build Configuration

The following settings in `build.toml` control build-time behaviour:

| Key | Example | Description |
|-----|---------|-------------|
| `ovmf-version` | `edk2-stable202511-r1` | Version of the OVMF firmware to download. |
| `ovmf-source-template` | GitHub release URL | URL template for the OVMF tarball (`$$` is replaced by the version string). |
| `ovmf-varsfd-path-template` | `$$-bin/x64/vars.fd` | Path inside the tarball for the VARS firmware volume. |
| `ovmf-codefd-path-template` | `$$-bin/x64/code.fd` | Path inside the tarball for the CODE firmware volume. |
| `font-version` | `1.11.6` | Version of the Tamzen console font to download. |
| `font-source-template` | GitHub release URL | URL template for the font tarball. |
| `font-path-template` | `tamzen-font-Tamzen-$$/psf/TamzenForPowerline10x20.psf` | Path inside the tarball for the PSF font file. |
| `install-bootloader` | `/boot/efi/pentos.efi` | Destination for the bootloader EFI binary on install. |
| `install-kernel` | `/boot/efi/pentos.kernel` | Destination for the kernel ELF binary on install. |
| `qemu-bin` | `qemu-system-x86_64` | QEMU binary name or path. |
| `qemu-numcores` | `12` | Number of SMP cores used by QEMU. |
| `qemu-mem` | `8G` | Amount of memory allocated to the QEMU VM. |
| `qemu-resolution` | `1920x1080` | Maximum resolution supported by the QEMU VM. |

## Runtime Configuration

Kernel behaviour is controlled by compile-time constants in the `config` crate (`config/src/`). The crate is organised into modules:

- `config::topology` hardware topology limits (e.g. maximum number of supported CPUs, interrupt controllers, and stack sizes per hart).
- `config::task` async executor limits (e.g. maximum number of concurrent urgent tasks).
- `config::dev` device driver settings (e.g. PS/2 keyboard retry limits and event queue sizes).

As an example, `config/src/topology/hart.rs` exposes `MAX_HART_COUNT` (default `16`), which determines the maximum number of CPU cores the OS will boot. The OS will panic at startup if more cores are detected than this constant allows.

## Acknowledgements

- [rust-osdev/ovmf-prebuilt](https://github.com/rust-osdev/ovmf-prebuilt) for pre-built OVMF firmware binaries.
- [sunaku/tamzen-font](https://github.com/sunaku/tamzen-font) for the Tamzen console font.