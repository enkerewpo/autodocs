use clap::{Command, arg};
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use sha2::Digest;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct Filter {
    target: String,
    include: Vec<String>,
    exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TranslationConfig {
    repo: String,
    branch: String,
    filter: Filter,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FileEntry {
    path: String,
    hash: String,
    translation_timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TranslationMeta {
    commit: String,
    files: Vec<FileEntry>,
}

fn cli() -> Command {
    Command::new("deepdocs").about("DeepDocs CLI").subcommand(
        Command::new("run")
            .about("Run the auto-translation using the config file")
            .arg(arg!(<CONFIG> "The YAML config file to use"))
            .arg_required_else_help(true),
    )
}

fn prase_target_suffix(target: &str) -> Vec<String> {
    // "*.md *.txt" -> "md txt"
    let mut suffix = target.replace("*", "");
    suffix = suffix.replace(".", "");
    suffix = suffix.trim().replace(" ", "");
    suffix.split(" ").map(|s| s.to_string()).collect()
}

fn run(config: TranslationConfig) {
    println!("Running the auto-translation with the following config:");
    println!("{:?}", config);
    // step1: clone the repo into workspace, default at ./workspace
    // create a workspace folder if not exists
    if !std::path::Path::new("./workspace").exists() {
        println!("Creating workspace folder at ./workspace");
        std::fs::create_dir("./workspace").unwrap();
    }
    // clone the repo
    // the translated snapshots will be places under ./workspace/<repo_name>-translated
    let repo = config.repo;
    let branch = config.branch;
    let workspace = "./workspace";
    let repo_name = repo.split("/").last().unwrap();
    let repo_name = repo_name.split(".").next().unwrap();
    let repo_path = format!("{}/{}", workspace, repo_name);
    let translated_repo_path = format!("{}/{}-translated", workspace, repo_name);
    // translation metadata stored in the {workspace}/{repo_name}.meta.json
    let meta_path = format!("{}/{}.meta.json", workspace, repo_name);
    // we need to implement the "SYNC" logic:
    // 1. if {repo_name} folder exists, pull the latest changes
    // 2. else clone the repo
    // 3. iterate all files in the repo except .git folder and using the filter to decide whether to translate
    // 4. mapping(copying) the files to the translated_repo_path with part of the files translated
    //      - we first check the metadata file to see if the file has been translated before(sha256 hash same), if so, we skip
    //      - else we update the metadata file with the new file entry, if the metadata file does not exist, we create one
    //      - we translate the file content and write to the translated_repo_path
    // 5. update metadata file with the latest commit hash, all relative paths according to the workspace root that are translated, we store a SHA256 hash of the original file content, sync timestamp, etc.

    if std::path::Path::new(&repo_path).exists() {
        println!("Pulling the latest changes from the repo: {}", repo);
        let output = std::process::Command::new("git")
            .arg("pull")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to pull the latest changes from the repo");
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("{}", String::from_utf8_lossy(&output.stderr));
    } else {
        println!("Cloning the repo: {}", repo);
        let output = std::process::Command::new("git")
            .arg("clone")
            .arg("--branch")
            .arg(branch)
            .arg(repo)
            .arg(repo_path.clone())
            .output()
            .expect("Failed to clone the repo");
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // read the metadata file
    let mut meta = if std::path::Path::new(&meta_path).exists() {
        let meta = std::fs::read_to_string(&meta_path);
        if let Err(e) = meta {
            println!("Error reading the metadata file @ {}: {}", meta_path, e);
            return;
        }
        let meta = serde_json::from_str(&meta.unwrap());
        match meta {
            Ok(meta) => meta,
            Err(e) => {
                println!("Error parsing the metadata file: {}", e);
                return;
            }
        }
    } else {
        TranslationMeta {
            commit: "".to_string(),
            files: vec![],
        }
    };

    // update comit hash
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get the latest commit hash");
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    meta.commit = commit.clone();

    // println!("Metadata: {:?}", meta);

    // iterate all files in the repo
    let mut q = vec![repo_path.clone()];
    let mut files = vec![];
    while !q.is_empty() {
        let path = q.pop().unwrap();
        let entries = std::fs::read_dir(&path);
        if let Err(e) = entries {
            println!("Error reading the directory @ {}: {}", path, e);
            return;
        }
        for entry in entries.unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                // skip the .git folder
                if path.ends_with(".git") {
                    continue;
                }
                q.push(path.to_str().unwrap().to_string());
            } else {
                let path = path.to_str().unwrap().to_string();
                files.push(path);
            }
        }
    }
    // println!("Files: {:?}", files);
    // filter the files
    let filter = config.filter;
    let suffix = prase_target_suffix(&filter.target);
    let mut filtered_files = vec![];
    for file in &files {
        // support suffix filter for now
        let mut include = false;
        for s in &suffix {
            if file.ends_with(s) {
                include = true;
                break;
            }
        }
        if !include {
            continue;
        }
        let mut exclude = false;
        for e in &filter.exclude {
            if file.contains(e) {
                exclude = true;
                break;
            }
        }
        if exclude {
            continue;
        }
        filtered_files.push(file.clone());
    }
    // println!("Filtered files: {:?}", filtered_files);
    // first copy all files that not need to be translated(not in filtered_files)
    for file in &files {
        if !filtered_files.contains(&file) {
            let translated_path = file.replace(&repo_path, &translated_repo_path);
            let translated_dir = translated_path.rsplitn(2, "/").last().unwrap();
            if !std::path::Path::new(&translated_dir).exists() {
                std::fs::create_dir_all(&translated_dir).unwrap();
            }
            // read binary and write binary
            let content = std::fs::read(&file).unwrap();
            std::fs::write(&translated_path, content).unwrap();
        }
    }
    // update the metadata file
    let mut translated_count = 0;
    for f in &filtered_files {
        let hash = format!("{:x}", sha2::Sha256::digest(&std::fs::read(&f).unwrap()));
        let mut translated = false;
        for file in &meta.files {
            if file.path == *f && file.hash == *hash {
                translated = true;
                break;
            }
        }
        if translated {
            translated_count += 1;
            continue;
        }
        let translated_path = f.replace(&repo_path, &translated_repo_path);
        let translated_dir = translated_path.rsplitn(2, "/").last().unwrap();
        if !std::path::Path::new(&translated_dir).exists() {
            std::fs::create_dir_all(&translated_dir).unwrap();
        }
        let content = std::fs::read_to_string(&f).unwrap();
        // translate the content
        let translated_content = "test_translation".to_string();
        println!("Translating file, src={}, dst={}", f, translated_path);
        std::fs::write(&translated_path, translated_content).unwrap();
        let file_entry = FileEntry {
            path: f.clone(),
            hash,
            translation_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        meta.files.push(file_entry);
    }
    let meta = serde_json::to_string_pretty(&meta);
    if let Err(e) = meta {
        println!("Error serializing the metadata: {}", e);
        return;
    }
    let meta = meta.unwrap();
    let mut file = std::fs::File::create(&meta_path).unwrap();
    if let Err(e) = file.write_all(meta.as_bytes()) {
        println!("Error writing the metadata file @ {}: {}", meta_path, e);
        return;
    }
    println!(
        "Translation finished, new files translated: {}, total files translated: {}, already translated files: {}",
        filtered_files.len() - translated_count,
        filtered_files.len(),
        translated_count
    );
}

fn main() {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("run", run_matches)) => {
            let config_file = run_matches.get_one::<String>("CONFIG").unwrap();
            let config = std::fs::read_to_string(config_file);
            if let Err(e) = config {
                println!("Error reading the config file @ {}: {}", config_file, e);
                return;
            }
            let config = serde_yaml::from_str(&config.unwrap());
            match config {
                Ok(config) => run(config),
                Err(e) => {
                    println!("Error parsing the config file: {}", e);
                }
            }
        }
        _ => {
            println!("No subcommand provided");
        }
    }
}
