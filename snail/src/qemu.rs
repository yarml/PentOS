use std::fmt::Display;
use std::process::Command;

pub struct Qemu {
    numcores: usize,
    memory: Memory,
    debugcon: Option<String>,
    drives: Vec<Drive>,
}

pub enum Memory {
    Giga(usize),
    Mega(usize),
    Kilo(usize),
    Byte(usize),
}

pub struct Drive {
    interface: Option<DriveInterface>,
    raw: bool,
    readonly: bool,
    file: String,
}

pub enum DriveInterface {
    Ide,
    Scsi,
    Sd,
    Mtd,
    Floppy,
    Pflash,
    Virtio,
    None,
}

impl Qemu {
    pub fn new() -> Qemu {
        Qemu {
            numcores: 1,
            memory: Memory::Giga(1),
            debugcon: None,
            drives: vec![],
        }
    }
    pub fn command(&self) -> Command {
        let mut cmd = Command::new("qemu-system-x86_64");
        self.append_args(&mut cmd);
        cmd
    }
}

impl Qemu {
    pub fn numcores(mut self, numcores: usize) -> Self {
        self.numcores = numcores;
        self
    }
    pub fn memory(mut self, memory: Memory) -> Self {
        self.memory = memory;
        self
    }
    pub fn debugcon(mut self, debugcon: &str) -> Self {
        self.debugcon = Some(debugcon.to_string());
        self
    }
    pub fn drive(mut self, drive: Drive) -> Self {
        self.drives.push(drive);
        self
    }
}

impl Drive {
    pub fn new(file: &str) -> Drive {
        Drive {
            interface: None,
            raw: false,
            readonly: false,
            file: file.to_string(),
        }
    }
}

impl Drive {
    pub fn interface(mut self, interface: DriveInterface) -> Self {
        self.interface = Some(interface);
        self
    }
    pub fn raw(mut self) -> Self {
        self.raw = true;
        self
    }
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }
}

impl Qemu {
    fn append_args(&self, cmd: &mut Command) {
        cmd.arg("-smp").arg(format!("{}", self.numcores));
        self.memory.append_args(cmd);
        if let Some(debugcon) = &self.debugcon {
            cmd.arg("-debugcon").arg(debugcon);
        }
        for drive in &self.drives {
            drive.append_args(cmd);
        }
    }
}

impl Memory {
    fn append_args(&self, cmd: &mut Command) {
        match self {
            Memory::Giga(size) => cmd.arg("-m").arg(format!("{}G", size)),
            Memory::Mega(size) => cmd.arg("-m").arg(format!("{}M", size)),
            Memory::Kilo(size) => cmd.arg("-m").arg(format!("{}K", size)),
            Memory::Byte(size) => cmd.arg("-m").arg(format!("{}", size)),
        };
    }
}

impl Drive {
    fn append_args(&self, cmd: &mut Command) {
        let mut options = Vec::new();
        if let Some(interface) = &self.interface {
            options.push(format!("if={}", interface));
        }
        if self.raw {
            options.push("format=raw".to_string());
        }
        if self.readonly {
            options.push("readonly=on".to_string());
        }
        options.push(format!("file={}", self.file));
        cmd.arg("-drive").arg(options.join(","));
    }
}

impl Default for Qemu {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for DriveInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveInterface::Ide => write!(f, "ide"),
            DriveInterface::Scsi => write!(f, "scsi"),
            DriveInterface::Sd => write!(f, "sd"),
            DriveInterface::Mtd => write!(f, "mtd"),
            DriveInterface::Floppy => write!(f, "floppy"),
            DriveInterface::Pflash => write!(f, "pflash"),
            DriveInterface::Virtio => write!(f, "virtio"),
            DriveInterface::None => write!(f, "none"),
        }
    }
}
