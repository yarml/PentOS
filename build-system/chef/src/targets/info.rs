use {
    crate::{
        crates::{self, Crate},
        target::{Target, run_policy::AlwaysRun},
    },
    std::{fs, io::BufRead, path::PathBuf},
};

pub fn lines() -> LinesTarget {
    LinesTarget
}

pub struct LinesTarget;

impl Target for LinesTarget {
    fn spec(&self) -> bool {
        print_line_counts(crates::all_crates());
        false
    }

    fn run_policy(&self) -> Box<dyn crate::target::run_policy::RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<std::rc::Rc<dyn Target>> {
        vec![]
    }
}

fn count_lines_in_file(path: &PathBuf) -> usize {
    let Ok(file) = fs::File::open(path) else {
        return 0;
    };
    std::io::BufReader::new(file).lines().count()
}

fn count_rs_lines_in_dir(dir: &PathBuf) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .map(|p| {
            if p.is_dir() {
                count_rs_lines_in_dir(&p)
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                count_lines_in_file(&p)
            } else {
                0
            }
        })
        .sum()
}

fn count_crate_lines(krate: &Crate) -> usize {
    let mut total = 0;

    let src = krate.path.join("src");
    if src.is_dir() {
        total += count_rs_lines_in_dir(&src);
    }

    let build_rs = krate.path.join("build.rs");
    if build_rs.is_file() {
        total += count_lines_in_file(&build_rs);
    }

    let build_dir = krate.path.join("build");
    if build_dir.is_dir() {
        total += count_rs_lines_in_dir(&build_dir);
    }

    total
}

fn print_line_counts(crates: impl Iterator<Item = &'static Crate>) {
    let mut counts: Vec<(String, usize)> = crates
        .map(|k| {
            let count = count_crate_lines(k);
            (k.name.clone(), count)
        })
        .collect();

    counts.sort_by_key(|(_, count)| *count);

    let total: usize = counts.iter().map(|(_, c)| c).sum();

    let name_width = counts.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let max_count = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
    let count_width = max_count.to_string().len().max(total.to_string().len());

    for (name, count) in &counts {
        println!("{name:<name_width$} {count:>count_width$}");
    }

    println!("{:<name_width$} {total:>count_width$}", "total");
}
