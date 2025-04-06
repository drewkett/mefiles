# MeFiles

A lightweight terminal-based file browser with Neovim integration, built in Rust.

## Features

- Navigate directories with keyboard controls
- Open files directly in Neovim
- Display file metadata (size, modification time)
- Toggle hidden files
- Clean, intuitive TUI interface

## Screenshots

```
 Current directory: /Users/andrew/Documents 
┌Files────────────────────────────────────────────────────────────────────────┐
│📁 ../                                     DIR                                │
│📁 Projects/                               DIR         2023-05-12 14:23:45    │
│📁 Work/                                   DIR         2023-06-18 09:10:22    │
│📄 notes.md                                12.5 KiB    2023-07-01 16:45:30    │
│📄 todo.txt                                1.2 KiB     2023-07-05 08:30:15    │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌Help─────────────────────────────────────────────────────────────────────────┐
│↑/↓: Navigate  Enter: Open dir/file  Backspace: Up  h: Toggle hidden  q: Quit │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Requirements

- Rust (1.60 or newer)
- Neovim (for file editing)

## Building from Source

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/mefiles.git
   cd mefiles
   ```

2. Build the project:
   ```
   cargo build --release
   ```

3. The compiled binary will be available at `target/release/mefiles`

## Usage

Run the application with:

```
mefiles [PATH] [OPTIONS]
```

### Arguments

- `PATH`: Starting directory (defaults to current directory)

### Options

- `-a, --all`: Show hidden files
- `-h, --help`: Print help
- `-V, --version`: Print version

### Keyboard Controls

- `↑/↓`: Navigate up/down the file list
- `Enter`: Open selected directory or file (files open in Neovim)
- `Backspace`: Navigate to parent directory
- `h`: Toggle display of hidden files
- `q`: Quit the application

## Examples

Start in the current directory:
```
mefiles
```

Start in your home directory and show hidden files:
```
mefiles ~ --all
```

## License

MIT
