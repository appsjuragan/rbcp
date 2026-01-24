use std::env;

#[derive(Debug, Clone)]
pub struct CopyOptions {
    pub source: String,
    pub destination: String,
    pub patterns: Vec<String>,
    
    pub recursive: bool,
    pub include_empty: bool,
    pub restartable: bool,
    pub backup_mode: bool,
    pub purge: bool,
    pub mirror: bool,
    pub move_files: bool,
    pub move_dirs: bool,
    pub attributes_add: String,
    pub attributes_remove: String,
    pub threads: usize,
    pub retries: usize,
    pub wait_time: u64,
    pub log_file: Option<String>,
    pub list_only: bool,
    pub show_progress: bool,
    pub log_file_names: bool,
    pub empty_files: bool,
    pub child_only: bool,
    pub shred_files: bool,
}

impl Default for CopyOptions {
    fn default() -> Self {
        CopyOptions {
            source: String::new(),
            destination: String::new(),
            patterns: Vec::new(),
            recursive: false,
            include_empty: false,
            restartable: false,
            backup_mode: false,
            purge: false,
            mirror: false,
            move_files: false,
            move_dirs: false,
            attributes_add: String::new(),
            attributes_remove: String::new(),
            threads: 1,
            retries: 1_000_000,
            wait_time: 30,
            log_file: None,
            list_only: false,
            show_progress: true,
            log_file_names: true,
            empty_files: false,
            child_only: false,
            shred_files: false,
        }
    }
}

impl CopyOptions {
    pub fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        
        if args.len() < 3 {
            return Err("Not enough arguments".to_string());
        }

        let mut options = CopyOptions::default();
        let mut positional_args = Vec::new();

        // Skip the program name
        for arg in args.iter().skip(1) {
            if arg.starts_with('/') {
                // It's a flag
                let upper_arg = arg.to_uppercase();
                match upper_arg.as_str() {
                    "/S" => options.recursive = true,
                    "/E" => {
                        options.recursive = true;
                        options.include_empty = true;
                    },
                    "/Z" => options.restartable = true,
                    "/B" => options.backup_mode = true,
                    "/PURGE" => options.purge = true,
                    "/MIR" => {
                        options.purge = true;
                        options.recursive = true;
                        options.include_empty = true;
                    },
                    "/MOV" => options.move_files = true,
                    "/MOVE" => {
                        options.move_files = true;
                        options.move_dirs = true;
                    },
                    "/L" => options.list_only = true,
                    "/NP" => options.show_progress = false,
                    "/NFL" => options.log_file_names = false,
                    "/EMPTY" => options.empty_files = true,
                    "/CHILDONLY" => options.child_only = true,
                    "/SHRED" => options.shred_files = true,
                    _ => {
                        if let Some(stripped) = upper_arg.strip_prefix("/A+:") {
                            options.attributes_add = stripped.to_string();
                        } else if let Some(stripped) = upper_arg.strip_prefix("/A-:") {
                            options.attributes_remove = stripped.to_string();
                        } else if upper_arg.starts_with("/MT") {
                            let threads = if upper_arg.len() > 4 && upper_arg.chars().nth(3) == Some(':') {
                                upper_arg[4..].parse::<usize>().unwrap_or(8)
                            } else {
                                8
                            };
                            options.threads = threads;
                        } else if let Some(stripped) = upper_arg.strip_prefix("/R:") {
                            let retries = stripped.parse::<usize>().unwrap_or(1_000_000);
                            options.retries = retries;
                        } else if let Some(stripped) = upper_arg.strip_prefix("/W:") {
                            let wait = stripped.parse::<u64>().unwrap_or(30);
                            options.wait_time = wait;
                        } else if upper_arg.starts_with("/LOG:") {
                            options.log_file = Some(arg[5..].to_string()); // Use original case for filename
                        }
                    }
                }
            } else {
                // It's a positional argument (Source, Dest, or Pattern)
                positional_args.push(arg.clone());
            }
        }

        if positional_args.len() < 2 {
            return Err("Missing source or destination".to_string());
        }

        options.source = positional_args[0].clone();
        options.destination = positional_args[1].clone();

        // Any remaining positional args are patterns
        if positional_args.len() > 2 {
            for pattern in positional_args.iter().skip(2) {
                options.patterns.push(pattern.clone());
            }
        } else {
            // Default pattern if none specified
            options.patterns.push("*.*".to_string());
        }

        Ok(options)
    }

    pub fn to_string_flags(&self) -> String {
        let mut result = Vec::new();

        if self.recursive {
            if self.include_empty {
                result.push("/E".to_string());
            } else {
                result.push("/S".to_string());
            }
        }

        if self.restartable {
            result.push("/Z".to_string());
        }

        if self.backup_mode {
            result.push("/B".to_string());
        }

        if self.mirror {
            result.push("/MIR".to_string());
        } else if self.purge {
            result.push("/PURGE".to_string());
        }

        if self.move_dirs {
            result.push("/MOVE".to_string());
        } else if self.move_files {
            result.push("/MOV".to_string());
        }

        if !self.attributes_add.is_empty() {
            result.push(format!("/A+:{}", self.attributes_add));
        }

        if !self.attributes_remove.is_empty() {
            result.push(format!("/A-:{}", self.attributes_remove));
        }

        if self.threads != 1 {
            result.push(format!("/MT:{}", self.threads));
        }

        if self.retries != 1_000_000 {
            result.push(format!("/R:{}", self.retries));
        }

        if self.wait_time != 30 {
            result.push(format!("/W:{}", self.wait_time));
        }

        if self.list_only {
            result.push("/L".to_string());
        }

        if !self.show_progress {
            result.push("/NP".to_string());
        }

        if !self.log_file_names {
            result.push("/NFL".to_string());
        }

        if self.empty_files {
            result.push("/EMPTY".to_string());
        }

        if self.child_only {
            result.push("/CHILDONLY".to_string());
        }

        if self.shred_files {
            result.push("/SHRED".to_string());
        }

        result.join(" ")
    }
}

pub fn print_usage(program_name: &str) {
    println!("Usage: {} <source> <destination> [<file_pattern>...] [options]", program_name);
    println!("Options:");
    println!("  /S         - Copy subdirectories, but not empty ones");
    println!("  /E         - Copy subdirectories, including empty ones");
    println!("  /Z         - Copy files in restartable mode (slower but more robust)");
    println!("  /B         - Copy files in Backup mode (overrides permissions)");
    println!("  /PURGE     - Delete destination files/folders that no longer exist in source");
    println!("  /MIR       - Mirror directory tree (like /PURGE plus all subdirectories)");
    println!("  /MOV       - Move files (delete from source after copying)");
    println!("  /MOVE      - Move files and directories (delete from source after copying)");
    println!("  /A+:[RASHCNETO] - Add specified attributes to copied files");
    println!("  /A-:[RASHCNETO] - Remove specified attributes from copied files");
    println!("  /MT[:n]    - Multithreaded copying with n threads (default is 8)");
    println!("  /R:n       - Number of retries on failed copies (default is 1 million)");
    println!("  /W:n       - Wait time between retries in seconds (default is 30)");
    println!("  /LOG:file  - Output log to file");
    println!("  /L         - List only - don't copy, timestamp or delete any files");
    println!("  /NP        - No progress - don't display % copied");
    println!("  /NFL       - No file list - don't log file names");
    println!("  /EMPTY     - Create empty (zero-byte) copies of files");
    println!("  /CHILDONLY - Process only direct child folders of source path");
    println!("  /SHRED     - Securely overwrite files before deletion");
}
