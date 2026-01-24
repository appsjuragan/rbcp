# rbcp: Robust Copy Utility

`rbcp` is a high-performance, cross-platform command-line file copying utility inspired by Microsoft's Robocopy. It is designed for reliability, speed, and flexibility, offering features like directory mirroring, multithreaded copying, and secure file deletion.

## Features

*   **Modern GUI**: A beautiful, responsive graphical interface for easy operation.
*   **Cross-Platform**: Runs natively on Windows, Linux, and macOS.
*   **Multithreaded Performance**: Utilizes multiple CPU cores for faster file transfers (`/MT`).
*   **Directory Mirroring**: Synchronize source and destination directories perfectly (`/MIR`).
*   **Robustness**: Automatic retries for failed copies (`/R`, `/W`) and restartable mode (`/Z`).
*   **Secure Deletion**: Optional secure shredding of deleted files (`/SHRED`).
*   **Advanced Filtering**: Include/exclude files based on patterns and attributes.
*   **Real-time Progress**: Visual progress bars, speed display (MB/s), and detailed logs.
*   **Single File Support**: Copy individual files or entire folders seamlessly.

## Graphical User Interface (GUI)

Launch `rbcp` without any command-line arguments to open the GUI.

### GUI Features:
*   **Dark/Light Mode**: Toggle between themes for comfortable viewing.
*   **Browse Menu**: Easily select folders or individual files as source.
*   **Real-time Speed**: Monitor transfer rates in MB/s.
*   **Overwrite Confirmation**: Safety check when destination directory already exists.
*   **Log View**: Integrated real-time log viewer with toggle.
*   **Minimize to Tray**: Keep the application running in the background.

## Installation

### From Source

1.  Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.
2.  Clone the repository:
    ```bash
    git clone https://github.com/yourusername/rbcp.git
    cd rbcp
    ```
3.  Build the project:
    ```bash
    cargo build --release
    ```
4.  The executable will be at `target/release/rbcp`.

## Usage Manual

### Basic Syntax

```
rbcp <source> <destination> [file_pattern...] [options]
```

*   **source**: Path to the source directory.
*   **destination**: Path to the destination directory.
*   **file_pattern**: (Optional) One or more wildcard patterns to filter files (e.g., `*.jpg`, `data*`). Defaults to `*.*`.

### Command Line Options

#### Copy Options

| Option | Description |
| :--- | :--- |
| `/S` | Copy subdirectories, but not empty ones. |
| `/E` | Copy subdirectories, including empty ones. |
| `/Z` | **Restartable Mode**: Flushes data after every write. Slower but ensures data integrity if interrupted. |
| `/B` | **Backup Mode**: (Windows only) Attempts to bypass file permission issues. |
| `/MT[:n]` | **Multithreading**: Run with `n` threads. If `n` is omitted, defaults to 8. If `/MT` is not used, runs single-threaded. |
| `/EMPTY` | **Empty Files**: Create zero-byte placeholders instead of copying actual file content. Useful for testing structure. |

#### Selection & Filtering

| Option | Description |
| :--- | :--- |
| `/CHILDONLY` | **Child Only**: Process only the immediate children of the source directory, ignoring the root files. |
| `/A+:[RASHCNETO]` | **Add Attributes**: Add the specified attributes to copied files (Read-only, Archive, System, Hidden, etc.). |
| `/A-:[RASHCNETO]` | **Remove Attributes**: Remove the specified attributes from copied files. |

#### Move & Delete Options

| Option | Description |
| :--- | :--- |
| `/PURGE` | **Purge**: Delete files/directories in the destination that no longer exist in the source. |
| `/MIR` | **Mirror**: Equivalent to `/E` + `/PURGE`. Makes the destination an exact copy of the source. |
| `/MOV` | **Move Files**: Delete files from the source after they are successfully copied. |
| `/MOVE` | **Move All**: Delete files and directories from the source after copying. |
| `/SHRED` | **Secure Delete**: When deleting files (via `/PURGE`, `/MIR`, or `/MOV`), overwrite them with random data before deletion to prevent recovery. |

#### Logging & Control

| Option | Description |
| :--- | :--- |
| `/L` | **List Only**: List files that *would* be copied or deleted, but do not make any changes. |
| `/LOG:file` | **Log File**: Write status output to the specified file (overwrites existing). |
| `/NP` | **No Progress**: Do not display the percentage progress bar (recommended for log files). |
| `/NFL` | **No File List**: Do not log the names of files being copied. |
| `/R:n` | **Retries**: Number of times to retry a failed copy (default: 1 million). |
| `/W:n` | **Wait**: Wait time (in seconds) between retries (default: 30). |

## Examples

### 1. Simple Backup
Copy all files from `C:\Work` to `D:\Backup`, including subdirectories.
```bash
rbcp C:\Work D:\Backup /E
```

### 2. Mirroring (Exact Sync)
Make `D:\Backup` exactly match `C:\Work`. **Warning**: This deletes files in `D:\Backup` that are not in `C:\Work`.
```bash
rbcp C:\Work D:\Backup /MIR
```

### 3. Fast Multithreaded Copy
Copy using 16 threads for maximum speed.
```bash
rbcp C:\Images D:\Images /E /MT:16
```

### 4. Move and Securely Delete
Move files to an archive and securely shred the originals.
```bash
rbcp C:\Sensitive D:\Archive /MOVE /SHRED
```

### 5. Network Transfer with Retries
Copy over a flaky network connection, retrying 5 times with a 10-second wait.
```bash
rbcp \\Server\Share C:\Local /Z /R:5 /W:10
```

### 6. Filter Specific Files
Copy only `.jpg` and `.png` files.
```bash
rbcp C:\Photos D:\Backup *.jpg *.png /S
```

## Return Codes

`rbcp` does not currently use standard Robocopy return codes (bitmaps). It returns `0` on success and non-zero on critical errors. Check the logs for detailed failure counts.

## License

MIT License
