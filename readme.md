# RBCP v2.0 - Robust Copy with Modern GUI

A powerful, high-performance file copy utility built with Rust and Tauri v2, featuring a stunning emerald-themed glassmorphism UI.

![RBCP v2.0](docs/screenshot.png)

## ✨ Features

### Core Functionality
- **🚀 Multi-threaded Copying**: Leverages parallel processing for maximum speed
- **📊 Real-time Progress**: Accurate percentage with pre-scan counting
- **🔄 Smart Resumption**: Continue interrupted transfers
- **🗑️ Secure Deletion**: DOD 5220.22-M compliant file shredding
- **📁 Windows Explorer Behavior**: Intuitive directory copying (preserves root folder)
- **🔍 Pattern Matching**: Flexible file filtering with glob patterns
- **🪞 Mirror Mode**: Synchronize source and destination
- **♻️ Move Operations**: Cut and paste functionality

### GUI Enhancements (v2.0)
- **🎨 Emerald Green Theme**: Beautiful glassmorphism design with dark/light mode
- **⚡ Startup Loader**: Smooth loading animation
- **🔔 Smart Warnings**: Conflict detection with native-style dialogs
- **📈 Dynamic Status**: Real-time state updates (Ready → Scanning → Copying → Finished)
- **💾 Directory Memory**: Remembers last used paths
- **🚫 Infinite Loop Guard**: Prevents copying directory into itself
- **📏 Responsive Layout**: Adapts to window resizing
- **📂 Multi-file Selection**: Select folders or multiple files at once

## 🖥️ Screenshots

### Main Interface
- Clean, modern UI with emerald accents
- Real-time progress ring with percentage
- Live transfer speed and object count
- Activity log with timestamps

### Overwrite Dialog
- Native Windows-style conflict resolution
- Options: Skip All, Overwrite All, or Cancel
- Only appears when actual conflicts exist

## 🛠️ Installation

### Pre-built Binary
Download the latest release from the [Releases](https://github.com/yourusername/rbcp/releases) page.

### Build from Source

#### Prerequisites
- Rust 1.70+ ([rustup.rs](https://rustup.rs))
- Node.js 18+ (for Tauri)
- Windows 10+ / Linux / macOS

#### Build Steps
```bash
# Clone the repository
git clone https://github.com/yourusername/rbcp.git
cd rbcp

# Build release version
cargo build --release

# GUI executable will be in:
# ./target/release/rbcp-gui.exe (Windows)
# ./target/release/rbcp-gui (Linux/macOS)
```

## 📖 Usage

### GUI Mode

1. Launch `rbcp-gui.exe`
2. **Select Source**: Click 📁 for folder or 📄 for files
3. **Select Destination**: Choose target directory
4. **Configure Options** (optional):
   - Recursive: Include subdirectories
   - Mirror: Sync source to destination
   - Move: Delete source after copy
   - Secure Delete: Shred moved files
5. Click **Start Copy**

### CLI Mode

```bash
# Basic copy
rbcp source_dir dest_dir

# Recursive copy with patterns
rbcp source dest -r -p "*.txt" "*.md"

# Mirror directories
rbcp source dest --mirror

# Multi-threaded copy
rbcp source dest -t 16

# Secure move
rbcp source dest --move --shred
```

#### Common Options
```
-r, --recursive         Copy subdirectories
-t, --threads <N>       Number of threads (default: 8)
-p, --patterns <PAT>    File patterns to match
--mirror                Mirror mode (sync with deletion)
--move                  Move instead of copy
--shred                 Secure file deletion
--force                 Overwrite without prompt
```

## 🎯 Advanced Features

### Pattern Matching
Supports glob patterns for flexible file filtering:
```bash
# Copy only images
rbcp source dest -p "*.jpg" "*.png" "*.gif"

# Exclude specific files
rbcp source dest -p "*" "!*.tmp"

# Copy by name pattern
rbcp source dest -p "report_*.pdf"
```

### Conflict Resolution
When files/folders exist at destination:
- **Skip All**: Preserve existing files
- **Overwrite All**: Replace all conflicts
- **Cancel**: Abort operation

### Progress Tracking
The engine performs a fast pre-scan to:
1. Count total files and bytes
2. Enable accurate progress percentage
3. Show meaningful "X of Y objects" counter

## 🏗️ Architecture

### Tech Stack
- **Core**: Rust (rbcp-core library)
- **GUI Framework**: Tauri v2
- **Frontend**: Vanilla HTML/CSS/JavaScript
- **Parallelism**: Rayon
- **File Operations**: std::fs + custom optimizations

### Project Structure
```
rbcp/
├── rbcp-core/          # Core copy engine (library)
│   ├── src/
│   │   ├── engine.rs   # Main copy orchestration
│   │   ├── copy.rs     # File/directory operations
│   │   ├── progress.rs # Progress tracking
│   │   ├── args.rs     # Configuration
│   │   └── stats.rs    # Statistics
│   └── Cargo.toml
├── src-tauri/          # Tauri backend
│   ├── src/
│   │   ├── main.rs
│   │   └── commands.rs # Tauri commands
│   └── tauri.conf.json
├── ui/                 # Frontend
│   ├── index.html
│   ├── style.css
│   └── main.js
└── README.md
```

## 🔧 Development

### Running in Dev Mode
```bash
# Start Tauri dev server
cd src-tauri
cargo tauri dev
```

### Code Formatting
```bash
# Format all Rust code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Running Tests
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

## 📝 Configuration

### GUI Settings (Persistent)
The GUI automatically remembers:
- Last source directory
- Last destination directory
- Theme preference (dark/light)

Settings are stored in browser localStorage.

## 🐛 Known Issues & Limitations

- **Windows Only**: Some features are Windows-specific
- **Large Operations**: Very large file counts (>1M files) may take time to scan
- **Network Drives**: Performance may vary on network paths

## 🗺️ Roadmap

- [ ] Linux/macOS support
- [ ] Bandwidth throttling
- [ ] Resume interrupted transfers
- [ ] Cloud storage integration
- [ ] Scheduled/automated copies
- [ ] File deduplication
- [ ] Compression support

## 🤝 Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and formatting
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by `robocopy` and `rsync`
- Built with [Tauri](https://tauri.app)
- UI design influenced by modern glassmorphism trends

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/rbcp/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/rbcp/discussions)

---

**Made with ❤️ and Rust**
