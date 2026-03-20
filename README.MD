# PentOS
A hobby x86_64 operating system written in Rust, featuring a UEFI bootloader, an async kernel, a custom build system, and much more to come.

This is the 5th iteration of [HeliumOS](https://github.com/yarml/HeliumOS), hence the name.

**Developer documentation:** [pentos.yarml.com](https://pentos.yarml.com)

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
## Base Dependencies
The following tools must be installed manually before building:

- `cargo` / `rustup`
- `nasm`
- `make`

## Optional Dependencies
| Tool | Purpose |
|-|-|
| `qemu-system-x86_64` | Required for quickly running PentOS on a VM (`make run-*` & `make debug`). |
| `gdb`, `tmux`, & `socat` | Required for debugging the kernel (alongside Qemu) (`make debug`). |
| `mtools`, `e2fsprogs`, & `sfdisk` | Required to create raw `.img` disk images (`make img-*`). |
| `VBoxManage` | Required to convert images to `.vmdk` or `.iso` format (`make vmdk-*` / `make iso-*`). |

## Automatic dependencies
PentOS depends on other tools, but those not listed above are autmatically managed by the build system itself.

A comprehensive list of all build dependencies can be found in (Cargo.toml).

Additionally OVMF is downloaded and managed automatically by the build system the first time you run `make`.

## Building
### First-time setup
Run `make` once to bootstrap the build system and download OVMF:

```sh
make
```

### Building the bootloader, kernel, & user binaries
```sh
# Debug build
make build-bootloader-debug
make build-kernel-debug
make build-userbin-debug

# Release build
make build-bootloader-release
make build-userbin-release
```

Output locations:
- Bootloader: `target/uefi/{debug,release}/bootloader.efi`
- Kernel: `target/kernel/{debug,release}/kernel`
- user binaries `target/user/{debug,release}/*`

## Running in QEMU
These targets build everything automatically before launching QEMU.

```sh
# Run release build
make run

# Run debug build (no debugger attached)
make run-debug

# Run debug build with GDB + tmux (attaches GDB and a QEMU monitor)
make debug
```

The configuration Qemu uses for these commands can be found at the root `Cargo.toml`. The defaults are:
- 12 SMP Cores
- 8GB of RAM
- Resolution set to 1920x1080

## Creating Disk Images
### Flat
These will create normal directories containing the system
```sh
make flat-release   # run/release/flat
make flat-debug     # run/debug/flat
```

### Raw `.img`
Requires `mtools` and `parted`.

```sh
make img-release   # run/release/pentos.img
make img-debug     # run/debug/pentos.img
```

### VMDK (VirtualBox / VMware)
Requires `VBoxManage`. Depends on the `.img` targets above.

```sh
make vmdk-release  # run/release/pentos.vmdk
make vmdk-debug    # run/debug/pentos.vmdk
```

### ISO
Requires `VBoxManage`. Also depends on the `.img` targets.

```sh
make iso-release   # run/release/pentos.iso
make iso-debug     # run/debug/pentos.iso
```

## Building the Documentation
```sh
make doc
```

This generates rustdoc for the entire workspace under `target/doc/`. An `index.html` redirect to the `pentos` root crate is written automatically.

The hosted version of the developer documentation is available at [pentos.yarml.com](https://pentos.yarml.com).

## Build Configuration

The following settings in the root `Cargo.toml` under `[workspace.metadata.chef]` control build-time behaviour:

| Key | Default | Description |
|-----|---------|-------------|
| `ovmf-version` | `edk2-stable202511-r1` | Version of the OVMF firmware to download. |
| `ovmf-source-template` | GitHub release URL | URL template used to fetch the OVMF tarball (`$$` is replaced by the version string). |
| `ovmf-varsfd-path-template` | `$$-bin/x64/vars.fd` | Path inside the tarball for the VARS firmware volume (`$$` is replaced by the version string). |
| `ovmf-codefd-path-template` | `$$-bin/x64/code.fd` | Path inside the tarball for the CODE firmware volume (`$$` is replaced by the version string). |
| `install-bootloader` | `/boot/efi/pentos.efi` | Destination used by `make install` for the bootloader EFI binary. |
| `install-kernel` | `/boot/efi/pentos.kernel` | Destination used by `make install` for the kernel ELF binary. |
| `qemu-numcores` | `12` | Number of SMP cores used by Qemu. |
| `qemu-mem` | `8G` | Amount of memory allocated to Qemu VM. |
| `qemu-resolution` | `1920x1080` | Maximm resolution supported by the Qemu VM. |

## Runtime Configuration

Kernel behaviour is controlled by compile-time constants in the `config` crate (`config/src/`). The crate is organised into modules:

- `config::topology` — hardware topology limits (e.g. maximum number of supported CPUs, interrupt controllers, and stack sizes per hart).
- `config::task` — async executor limits (e.g. maximum number of concurrent urgent tasks).
- `config::dev` — device driver settings (e.g. PS/2 keyboard retry limits and event queue sizes).

As an example, `config/src/topology/hart.rs` exposes `MAX_HART_COUNT` (default `16`), which determines the maximum number of CPU cores the OS will boot. The OS will panic at startup if more cores are detected than this constant allows.

Changing any constant in the `config` crate requires a rebuild of the kernel & bootloader.

## Acknowledgements

The following libraries were not taken as direct Cargo dependencies, but code or design from them was copied and adapted into this repository:

- **[spin](https://github.com/mvdnes/spin-rs)** (MIT) - The `lib/spinlocks` crate is heavily inspired by and contains code adapted from `spin`. It provides `SpinMutex`, `SpinRwLock`, and `SpinOnce` primitives tailored to the kernel environment.
- **[futures](https://github.com/rust-lang/futures-rs)** (MIT/Apache-2.0) - The async stream in `system/klib/src/task/stream.rs` in addition to other system in the future probably

## License

PentOS is licensed under the [GNU General Public License v3.0 or later](LICENSE).
