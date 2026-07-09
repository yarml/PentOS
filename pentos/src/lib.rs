//! # PentOS
//!
//! PentOS is a hobby x86_64 operating system written in Rust, featuring a UEFI
//! bootloader, an async kernel, a permission-based package model, and a custom
//! build system called `chef`.
//!
//! This documentation is published automatically at
//! <https://pentos.yarml.com> on every push to the main branch.
//!
//! ---
//!
//! ## Workspace layout
//!
//! The workspace is split into several groups of crates:
//!
//! ### System
//!
//! | Crate | Path | Description |
//! |---|---|---|
//! | `bootloader` | `system/bootloader` | UEFI bootloader — detects hardware, starts secondary CPUs, and hands control to the kernel |
//! | `kernel` | `system/kernel` | The kernel proper — drivers, interrupt handlers, and the scheduler entry point |
//! | `klib` | `system/klib` | Kernel library — memory management, the async executor, VFS, and all kernel subsystems |
//! | `system` | `system/system` | Shared conventions between the bootloader and kernel (memory layout, PAT indices, per-CPU structures) |
//! | `boot-protocol` | `system/boot-protocol` | Data structures passed from the bootloader to the kernel at handoff (`BootInfo`, `HartInfo`) |
//!
//! ### Libraries
//!
//! | Crate | Path | Description |
//! |---|---|---|
//! | `x64` | `lib/x64` | x86-64 architecture model — MSRs, control registers, paging, segmentation, GDT/IDT, APIC, I/O APIC |
//! | `acpi` | `lib/acpi` | ACPI table parser — discovers CPU cores, I/O APICs, and PCI devices |
//! | `elf` | `lib/elf` | ELF binary parser — used by the bootloader to load the kernel and by the kernel to load user programs |
//! | `spinlocks` | `lib/spinlocks` | Blocking `Mutex` and reader-writer lock primitives |
//! | `sync` | `lib/sync` | Higher-level synchronisation utilities |
//! | `utils` | `lib/utils` | Common data structures: lock-free queue, broadcast queue, and fixed-capacity `SmallVec` |
//! | `crypto` | `lib/crypto` | Cryptographic utilities — currently CRC32 for block-device integrity checks |
//! | `io` | `lib/io` | I/O abstractions |
//! | `keys` | `lib/keys` | Key/input event definitions shared across the stack |
//! | `log-debugcon` | `lib/log-debugcon` | `log`-compatible backend that writes to the x86 debug console port |
//! | `console-font` | `lib/console-font` | Embeds the [Tamzen](https://github.com/sunaku/tamzen-font) PSF console font |
//!
//! ### Filesystem
//!
//! | Crate | Path | Description |
//! |---|---|---|
//! | `fs` | `fs/fs` | Virtual filesystem layer — abstract `Directory`, `File`, and `FilesystemNode` interfaces |
//! | `block` | `fs/block` | Block device abstraction used by filesystem drivers |
//! | `gpt` | `fs/gpt` | GPT partition table parser/writer |
//! | `fat32` | `fs/fat32` | FAT32 filesystem driver (format, read, write) |
//!
//! ### Userspace
//!
//! | Crate | Path | Description |
//! |---|---|---|
//! | `runtime` | `user/lib/runtime` | Userspace runtime library linked into every user program |
//! | `init` | `user/core/init` | The `init` process — first userspace program launched by the kernel |
//!
//! ### Build system
//!
//! | Crate | Path | Description |
//! |---|---|---|
//! | `chef-dev` | `build-system/chef-dev` | Custom build tool — wraps `cargo`, manages OVMF/font downloads, generates disk images, runs QEMU, can be invoked with `cargo dev` |
//! | `chef-analyze` | `build-system/chef-analyze` | Generates JSON output for use with rust-analyzer. A glorified wrapper around clippy, can be invoked with `cargo analyze` |
//! | `chef-core` | `build-system/chef-core` | Backend for `chef-dev` and `chef-analyze`  |
//! | `builder` | `build-system/builder` | Build helpers shared between `chef` and crate build scripts (e.g. NASM integration) |
//!
//! ---
//!
//! ## Architecture overview
//!
//! ### Boot sequence
//!
//! 1. UEFI transfers control to the **bootloader**.
//! 2. The bootloader uses `CPUID` to verify all required CPU features are
//!    present, halting with a user-visible error if any are missing.
//! 3. It walks the **ACPI** tree (`XSDT → MADT, MCFG`) to discover CPUs,
//!    I/O APICs, and PCI devices.
//! 4. It sets up a framebuffer via `EFI_GRAPHICS_OUTPUT_PROTOCOL`.
//! 5. It exits boot services, obtains the physical memory map, and classifies
//!    memory into *legacy* (<1 MiB), *low* (1–16 MiB), *middle* (16 MiB–4 GiB),
//!    and *high* (>4 GiB) regions.
//! 6. It loads the kernel ELF from the system FAT partition, maps its segments
//!    into virtual memory, and allocates per-CPU TLS regions and 2 MiB stacks.
//! 7. It wakes secondary CPU cores via `INIT-IPI` / `STARTUP-IPI` and a
//!    16-bit trampoline that brings each core into 64-bit mode.
//! 8. Control is handed to the kernel entry point together with a `BootInfo`
//!    structure carrying the memory map, framebuffer info, ACPI data, and
//!    per-CPU `HartInfo`.
//!
//! ### Kernel task model
//!
//! The kernel is **fully async**. Each CPU core runs an event loop:
//!
//! 1. All *urgent* tasks run to completion.
//! 2. Up to *N* kernel tasks are polled (N is a build-time constant).
//! 3. One user process is given the CPU until the next 10 ms timer interrupt.
//! 4. When all queues are empty the core executes `HLT`.
//!
//! Kernel tasks are written with standard `async`/`await`. When a task awaits
//! a resource it registers a [`Waker`] in the relevant subsystem; the subsystem
//! re-enqueues the task when the resource becomes available.
//!
//! User processes are scheduled **round-robin** with 30 ms quanta. Each process
//! is managed by a kernel task whose body is a simple loop:
//!
//! ```rust,ignore
//! loop {
//!     let syscall = process.advance().await; // yields until a syscall arrives
//!     let keep_alive = handle(syscall).await;
//!     if !keep_alive { break; }
//! }
//! ```
//!
//! ### Virtual filesystem
//!
//! The kernel maintains a set of named virtual filesystems. Each filesystem has
//! an optional namespace and is registered by a kernel module that defines the
//! semantics of every file operation. The canonical path format is:
//!
//! ```text
//! namespace:fs-name:/dir/subdir/file
//! ```
//!
//! Built-in filesystems:
//!
//! | Path prefix | Description |
//! |---|---|
//! | `pkg:<name>:/` | Per-package directory tree (`data/`, `config/`, `bin/`, …) |
//! | `system:/` | System-wide configuration and state (user database, installed packages, …) |
//! | `device:<dev>:/` | Raw device access exposed by drivers |
//! | `user:/` | The running user's private directory |
//! | `user:<other>:/` | Directory shared between the running user and `<other>` |
//!
//! ### Package & permission model
//!
//! Programs may only execute as part of an installed **package/profile pair**.
//! Each program ships a *permissions file* that declares every resource it may
//! ever access. By default a program can only reach its own package/profile
//! data directory; any additional access must be declared explicitly. Users may
//! create multiple profiles for the same package, sandboxing instances from
//! one another.
//!
//! ---
//!
//! ## Building
//!
//! All build tasks go through `cargo dev` (see `build-system/chef-dev`).
//!
//! ```sh
//! # Build the bootloader and kernel (release)
//! cargo dev build bootloader
//! cargo dev build kernel
//!
//! # Build and run in QEMU
//! cargo dev run
//!
//! # Build and run with GDB attached
//! cargo dev debug
//!
//! # Generate a raw GPT disk image
//! cargo dev generate img
//!
//! # Regenerate this documentation
//! cargo dev doc
//! ```
//!
//! OVMF firmware and the console font are downloaded automatically by `chef`
//! the first time they are needed.
//!
//! ---
//!
//! ## License
//!
//! PentOS is distributed under the **GPL-3.0-or-later** license.
//! See [`LICENSE`](https://github.com/yarml/PentOS/blob/main/LICENSE) for the full text.