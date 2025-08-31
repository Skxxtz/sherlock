use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=resources/");

    generate_resource_file();

    let status = Command::new("glib-compile-resources")
        .arg("--target=resources.gresources")
        .arg("--sourcedir=resources")
        .arg("resources/resources.gresources.xml")
        .status()
        .expect("Failed to execute glib-compile-resources");

    if !status.success() {
        panic!("glib-compile-resources failed");
    }
    println!("Resources compiled!!");
}

fn generate_resource_file() {
    let mut buf: Vec<String> = Vec::new();

    let current_file = Path::new(file!());
    let current_dir = current_file.parent().unwrap();
    let start: PathBuf = current_dir.join("resources");

    fn read_dir(dir: PathBuf, buf: &mut Vec<String>, base: &PathBuf) {
        if let Ok(entries) = dir.read_dir() {
            for item in entries.filter_map(Result::ok) {
                let path = item.path();

                // skip resources.gresources.xml
                if path.file_name().and_then(|n| n.to_str()) == Some("resources.gresources.xml") {
                    continue; // skip this one
                }

                if path.is_dir() {
                    read_dir(path, buf, base)
                } else if path.is_file() {
                    if let Ok(rel) = path.strip_prefix(base) {
                        buf.push(rel.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    read_dir(start.clone(), &mut buf, &start);

    let (icons, files): (Vec<String>, Vec<String>) =
        buf.into_iter().partition(|f| f.starts_with("icons/"));
    let files = files
        .into_iter()
        .map(|f| format!(r#"<file alias="{}">{}</file>"#, f, f))
        .collect::<Vec<String>>()
        .join("\n");
    let icons = icons
        .into_iter()
        .map(|f| format!(r#"<file alias="{}">{}</file>"#, f, f))
        .collect::<Vec<String>>()
        .join("\n");

    let file_wrapper = format!(
        r#"<gresource prefix="/dev/skxxtz/sherlock">{}</gresource>"#,
        files
    );
    let icon_wrapper = format!(
        r#"<gresource prefix="/org/gtk/libgtk/icons/">{}</gresource>"#,
        icons
    );

    let file = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<gresources>
    {}
    {}
</gresources>"#,
        file_wrapper, icon_wrapper
    );

    fs::write(&"resources/resources.gresources.xml", file)
        .expect("Failed to write resources.gresources.xml");
}
