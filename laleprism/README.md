# LALE Prism - Desktop Application

WCET Analysis and Real-Time Scheduling Visualization for Embedded Systems

## Prerequisites

### Required
- **Rust** (latest stable): https://rustup.rs/
- **Node.js** (v18+): https://nodejs.org/
- **npm** or **yarn**

### Platform-Specific Requirements

#### Linux
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

#### Windows
- Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## Quick Start

### 1. Clone and Setup

```bash
# Navigate to project root
cd /path/to/lale

# Install frontend dependencies
cd laleprism/frontend
npm install

# Return to laleprism directory
cd ..
```

### 2. Run the Application

#### Linux (Recommended)
```bash
# Use the provided launcher script (fixes Wayland issues)
./run.sh
```

#### All Platforms
```bash
# Development mode
cargo tauri dev

# Or set environment variables manually on Linux
WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 cargo tauri dev
```

This will:
1. Start the Vite dev server (frontend)
2. Compile the Rust backend
3. Launch the desktop application
4. Enable hot-reload for frontend changes

### 3. Build for Production

```bash
# From laleprism directory
cargo tauri build
```

The binary will be created in:
- **Linux**: `target/release/bundle/appimage/laleprism_*.AppImage`
- **macOS**: `target/release/bundle/macos/LALE Prism.app`
- **Windows**: `target/release/bundle/msi/laleprism_*.msi`

## Development Workflow

### Frontend Development

```bash
cd laleprism/frontend

# Install dependencies
npm install

# Run Vite dev server only (for UI development)
npm run dev

# Build frontend
npm run build

# Lint
npm run lint
```

### Backend Development

```bash
cd laleprism

# Check Rust code
cargo check

# Run tests
cargo test

# Build backend only
cargo build

# Build with release optimizations
cargo build --release
```

### Full Application

```bash
cd laleprism

# Development mode (hot-reload)
cargo tauri dev

# Production build
cargo tauri build

# Clean build artifacts
cargo clean
cd frontend && npm run clean
```

## Project Structure

```
laleprism/
├── src/                    # Rust backend
│   ├── main.rs            # Tauri entry point
│   ├── commands.rs        # Tauri commands (IPC)
│   ├── analysis.rs        # WCET analysis wrapper
│   ├── demangler.rs       # Symbol demangling
│   └── storage.rs         # Schedule persistence
├── frontend/              # React frontend
│   ├── src/
│   │   ├── services/      # Tauri API wrapper
│   │   ├── components/    # React components
│   │   └── pages/         # Application pages
│   └── package.json
├── Cargo.toml            # Rust dependencies
├── tauri.conf.json       # Tauri configuration
└── build.rs              # Build script
```

## Configuration

### Tauri Configuration

Edit `tauri.conf.json` to customize:
- Window size and behavior
- Application metadata
- Plugin permissions
- Build settings

### Frontend Configuration

Edit `frontend/vite.config.ts` for:
- Dev server settings
- Build optimizations
- Plugin configuration

## Troubleshooting

### "Failed to load module" errors

```bash
cd laleprism/frontend
rm -rf node_modules package-lock.json
npm install
```

### Rust compilation errors

```bash
cd laleprism
cargo clean
cargo update
cargo build
```

### Tauri CLI not found

```bash
cargo install tauri-cli --version "^2.0.0"
```

### WebView2 missing (Windows)

Download and install: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Permission denied (Linux)

```bash
chmod +x target/release/bundle/appimage/laleprism_*.AppImage
```

## Features

### Current (v0.1.0)
- ✅ WCET analysis for LLVM IR files
- ✅ 13 platform models (ARM Cortex-M/R/A, RISC-V)
- ✅ Symbol demangling (Rust/C++/C)
- ✅ Schedule generation (RMA/EDF)
- ✅ Interactive Gantt chart visualization
- ✅ Task table with detailed information
- ✅ Schedule persistence
- ✅ Cross-platform support

### Planned
- ⏳ WCET distribution charts
- ⏳ CPU utilization gauge
- ⏳ Schedule comparison
- ⏳ PDF export
- ⏳ Batch analysis

## Usage

### 1. Select LLVM IR Directory
Click "Select Directory" and choose a folder containing `.ll` files.

### 2. Configure Analysis
- Select target platform (e.g., ARM Cortex-M4)
- Choose scheduling policy (RMA or EDF)
- Configure tasks (manual or auto-generate)

### 3. Run Analysis
Click "Analyze" to start WCET analysis and schedule generation.

### 4. View Results
- Interactive Gantt chart showing task timeline
- Task table with demangled function names
- Schedulability status and metrics

### 5. Save Schedule
Save analysis results for later review or comparison.

## API Documentation

### Tauri Commands

All commands are available through the TypeScript service layer:

```typescript
import { tauriService } from './services/tauri';

// Analyze directory
const report = await tauriService.analyzeDirectory(config);

// List platforms
const platforms = await tauriService.listPlatforms();

// Demangle symbol
const demangled = await tauriService.demangleName(symbol);

// Save schedule
const id = await tauriService.saveSchedule(report, name);

// Load schedule
const report = await tauriService.loadSchedule(id);

// List schedules
const schedules = await tauriService.listSchedules();
```

## Performance

- **Analysis Speed**: <5s for 1000 functions
- **Memory Usage**: <200MB typical
- **Binary Size**: ~5-10MB (vs 100MB+ for Electron)
- **Startup Time**: <1s

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test --workspace`
5. Submit a pull request

## License

MIT OR Apache-2.0

## Support

- GitHub Issues: https://github.com/vertexclique/lale/issues
- Documentation: https://docs.lale.bot (coming soon)

## Acknowledgments

- Built with [Tauri](https://tauri.app/)
- Uses [lale](../lale) WCET analysis library
- UI based on [TailAdmin](https://tailadmin.com/)
