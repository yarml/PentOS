use {
    crate::{result::ResultExt, status::Status},
    std::process::{Command, Output, Stdio},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[derive(Clone, Copy)]
pub struct CommandOptions {
    expect_success: bool,
    capture_output: bool,
    show_output: bool,
}

/// Run command in a new process
pub fn run(mut command: Command, opt: CommandOptions) -> Option<Output> {
    Status::push("Running", display(&command));

    let (output, status) = if opt.capture_output {
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .or_fatal("could not execute command");
        if opt.show_output {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                eprintln!("{}", Status::indent(line));
            }
            for line in String::from_utf8_lossy(&output.stderr).lines() {
                eprintln!("{}", Status::indent(line));
            }
        }
        let status = output.status;
        (Some(output), status)
    } else {
        if !opt.show_output {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let status = command.status().or_fatal("status");
        (None, status)
    };

    if opt.expect_success && !status.success() {
        Status::error(format!("command exited with {}", status));
    }
    Status::pop();
    output
}

/// Run command in this process, thereby ending chef
pub fn exec(mut command: Command) -> ! {
    Status::push("Executing", display(&command));

    command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit());

    #[cfg(unix)]
    {
        let err = command.exec();
        Status::error(format!("could not execute command: {err}"));
    }

    #[cfg(not(unix))]
    {
        use std::process;

        let result = command.status();

        match result {
            Ok(status) => process::exit(status.code().unwrap_or(0)),
            Err(err) => Status::error(format!("could not execute command: {err}")),
        }
    }
}

pub fn display(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    let args = command
        .get_args()
        .map(|a| {
            let s = a.to_string_lossy();
            if s.contains(' ') {
                format!("\"{}\"", s)
            } else {
                s.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} {}", program, args)
}

#[allow(dead_code)]
impl CommandOptions {
    pub const fn new() -> Self {
        Self {
            expect_success: true,
            capture_output: false,
            show_output: true,
        }
    }
    pub const fn expect_success(&mut self, value: bool) -> &mut Self {
        self.expect_success = value;
        self
    }

    pub const fn capture_output(&mut self, value: bool) -> &mut Self {
        self.capture_output = value;
        self
    }

    pub const fn show_output(&mut self, value: bool) -> &mut Self {
        self.show_output = value;
        self
    }
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self::new()
    }
}
