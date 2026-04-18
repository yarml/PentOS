use {
    crate::{result::ResultExt, status::Status},
    std::process::{Command, Output, Stdio},
};

pub fn exec(mut command: Command, show_output: bool) -> Output {
    Status::push("Running", display(&command));

    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .or_fatal("could not execute command");

    if show_output {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            eprintln!("{}", Status::indent(line));
        }
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            eprintln!("{}", Status::indent(line));
        }
    }

    if !output.status.success() {
        Status::error(format!("command exited with {}", output.status));
    }

    Status::pop();
    output
}

fn display(command: &Command) -> String {
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
