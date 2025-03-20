pub fn add_nasm_lib(libname: &str, assemblies: &[&str]) {
    for assembly in assemblies {
        println!("cargo::rerun-if-changed={assembly}");
    }
    let full_lib_name = format!("lib{}.a", libname);
    nasm_rs::compile_library_args(&full_lib_name, assemblies, &["-felf64"])
        .expect("Could not compile NASM library");
    println!("cargo::rustc-link-lib={libname}");
}
