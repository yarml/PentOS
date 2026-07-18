use {
    crate::{
        args::{BuildProfile, GeneratePartition},
        paths,
    }, chef_core::{command, config::CONFIG, result::ResultExt}, std::process::Command
};

const QEMU_FEATURES: [&str; 7] = [
    "qemu64-v1",
    "pdpe1gb",
    "pcid",
    "invpcid",
    "fsgsbase",
    "x2apic",
    "rdrand",
];

const QEMU_DEBUG: [&str; 2] = ["unimp", "guest_errors"];

pub fn base(profile: BuildProfile) -> Command {
    let mut command = Command::new(&CONFIG.qemu_bin);

    command.args(["-machine", "q35"]);
    command.arg("-full-screen");
    command.args(["-debugcon", "stdio"]);
    command.args(["-smp", &CONFIG.qemu_numcores]);
    command.args(["-m", &CONFIG.qemu_mem]);
    command.args(["-cpu", &QEMU_FEATURES.join(",")]);

    command.args([
        "-drive",
        "if=pflash,format=raw,readonly=on,file=.build/ovmf/code.fd",
    ]);
    command.args(["-drive", "if=pflash,format=raw,file=.build/ovmf/vars.fd"]);

    command.args([
        "-drive",
        &format!(
            "id=nvm0,if=none,format=raw,file={disk_img}",
            disk_img = paths::img(GeneratePartition::Disk, profile)
                .to_str()
                .unwrap()
        ),
    ]);
    command.args(["-device", "nvme,drive=nvm0,serial=deadbeef"]);

    let resolution = CONFIG.qemu_resolution();
    command.args([
        "-device",
        &format!(
            "VGA,vgamem_mb={mem},xres={xres},yres={yres}",
            mem = resolution.vgamem_mb,
            xres = resolution.xres,
            yres = resolution.yres,
        ),
    ]);

    command
}

pub fn debug() -> Command {
    let tmpdir = tempfile::tempdir().or_fatal("tempdir").keep();
    let qms = tmpdir.join("qms");

    let mut qemu = base(BuildProfile::Debug);
    qemu.args(["-monitor", &format!("unix:{},server", qms.display())]);
    qemu.args(["-d", &QEMU_DEBUG.join(",")]);
    qemu.args(["-s", "-S"]);

    let qemu = format!("{} | tee .build/qemu.log 2>&1", command::display(&qemu));

    let socat = format!(
        "sleep 1 && socat -,echo=0,icanon=0 unix-connect:{}",
        qms.display()
    );

    let gdb = "sleep 1 && gdb -x build-system/gdb.rc -tui";

    let mut tmux = Command::new("tmux");
    tmux.args(["-f", "build-system/tmux.rc"]);
    tmux.args(["new", "sh", "-c", gdb]);
    tmux.args([";", "split-window", "-h", "sh", "-c", &qemu]);
    tmux.args([";", "split-window", "-v", "sh", "-c", &socat]);

    tmux
}
