extern crate walkdir;

use walkdir::WalkDir;

#[derive(Debug)]
pub struct Pubspec {
    pub name: String,
    pub path: String,
}

pub fn find_pubspecs(root_dir: &str) -> Vec<String> {
    let mut pubspecs = vec![];

    let walker = WalkDir::new(root_dir)
        .into_iter()
        // filter hidden files/directories
        .filter_entry(|e| {
            !e.file_name()
                .to_str()
                .map(|s| s.starts_with("."))
                .unwrap_or(false)
        })
        // skip errors (e.g. non permission directories)
        .filter_map(|e| e.ok());

    for entry in walker {
        let filename = entry.file_name().to_str().unwrap_or("").to_lowercase();
        let is_pubspec = filename == "pubspec.yaml" || filename == "pubspec.yml";
        if is_pubspec {
            if let Some(path) = entry.path().to_str() {
                pubspecs.push(path.to_owned());
            }
        }
    }

    pubspecs
}
