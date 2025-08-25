use std::process::Command;
use std::path::Path;

fn run_npm_command(args: &[&str], current_dir: &Path) -> std::io::Result<std::process::Output> {
    if cfg!(target_os = "windows") {
        let mut cmd_args = vec!["cmd", "/C", "npm"];
        cmd_args.extend_from_slice(args);
        Command::new("cmd")
            .args(&["/C", "npm"])
            .args(args)
            .current_dir(current_dir)
            .output()
    } else {
        Command::new("npm")
            .args(args)
            .current_dir(current_dir)
            .output()
    }
}

fn main() {
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/package.json");
    println!("cargo:rerun-if-changed=ui/index.html");

    let ui_dir = Path::new("ui");

    if !ui_dir.exists() {
        panic!("UI directory does not exist: {:?}", ui_dir);
    }

    if !ui_dir.join("node_modules").exists() {
        println!("cargo:warning=Installing ui dependencies...");
        let output = run_npm_command(&["install"], ui_dir)
            .expect("Failed to run npm install");
        if !output.status.success() {
            panic!("npm install failed: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    println!("cargo:warning=Building ui...");
    let output = run_npm_command(&["run", "build"], ui_dir)
        .expect("Failed to run npm run build");

    if !output.status.success() {
        panic!("npm run build failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    println!("cargo:warning=UI build completed successfully.");
}