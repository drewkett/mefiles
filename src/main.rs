use chrono::{DateTime, Local};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use humansize::{format_size, BINARY};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

/// Interactive file browser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Starting directory (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Show hidden files
    #[arg(short = 'a', long)]
    all: bool,
}

struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: String,
}

struct App {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected_index: usize,
    show_hidden: bool,
}

impl App {
    fn new(path: PathBuf, show_hidden: bool) -> Self {
        let mut app = App {
            current_dir: path,
            entries: Vec::new(),
            selected_index: 0,
            show_hidden,
        };
        app.refresh_entries();
        app
    }

    fn refresh_entries(&mut self) {
        self.entries.clear();
        self.selected_index = 0;

        // Add parent directory entry (..) if not at root
        if self.current_dir.parent().is_some() {
            self.entries.push(FileEntry {
                name: String::from(".."),
                path: self.current_dir.join(".."),
                is_dir: true,
                size: 0,
                modified: String::new(),
            });
        }

        // Get all entries in the current directory
        let entries = fs::read_dir(&self.current_dir).unwrap_or_else(|_| {
            // If we can't read the directory, try to go up one level
            if let Some(parent) = self.current_dir.parent() {
                self.current_dir = parent.to_path_buf();
                fs::read_dir(&self.current_dir).unwrap()
            } else {
                panic!("Cannot read directory: {:?}", self.current_dir);
            }
        });

        // Process each entry
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();

            // Skip hidden files/dirs if show_hidden is false
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }

            let metadata = match fs::metadata(&path) {
                Ok(meta) => meta,
                Err(_) => {
                    // Skip entries we can't get metadata for
                    continue;
                }
            };

            let is_dir = metadata.is_dir();
            let size = if is_dir { 0 } else { metadata.len() };
            let modified = format_modified_time(&metadata);

            self.entries.push(FileEntry {
                name: file_name,
                path,
                is_dir,
                size,
                modified,
            });
        }

        // Sort: directories first, then files, both alphabetically
        self.entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
    }

    fn navigate_to(&mut self, path: &Path) {
        if path.is_dir() {
            // Canonicalize the path to resolve any ".." components
            if let Ok(canonical_path) = fs::canonicalize(path) {
                self.current_dir = canonical_path;
            } else {
                // Fallback to the original path if canonicalization fails
                self.current_dir = path.to_path_buf();
            }
            self.refresh_entries();
        }
    }

    fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            // Canonicalize the parent path to resolve any ".." components
            if let Ok(canonical_path) = fs::canonicalize(parent) {
                self.current_dir = canonical_path;
            } else {
                // Fallback to the original parent path if canonicalization fails
                self.current_dir = parent.to_path_buf();
            }
            self.refresh_entries();
        }
    }

    fn toggle_hidden_files(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh_entries();
    }
}

fn format_modified_time(metadata: &fs::Metadata) -> String {
    metadata
        .modified()
        .map(|time| {
            let datetime: DateTime<Local> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|_| String::from("Unknown"))
}

fn open_in_neovim<B: ratatui::backend::Backend + std::io::Write>(
    path: &Path,
    terminal: &mut Terminal<B>,
) -> io::Result<()> {
    // Restore terminal to normal state before launching neovim
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Launch neovim with the selected file
    let status = Command::new("nvim")
        .arg(path)
        .status()
        .expect("Failed to execute neovim");

    // Check if neovim exited successfully
    if !status.success() {
        eprintln!("Neovim exited with error: {}", status);
    }

    // Restore terminal to app state
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.hide_cursor()?;

    // Force a terminal refresh
    terminal.clear()?;

    Ok(())
}

fn run_app<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(1), // Status bar
                    Constraint::Min(0),    // File list
                    Constraint::Length(3), // Info panel
                ])
                .split(f.area());

            // Status bar
            let status = format!(" Current directory: {} ", app.current_dir.display());
            let status_bar =
                Paragraph::new(status).style(Style::default().bg(Color::Blue).fg(Color::White));
            f.render_widget(status_bar, chunks[0]);

            // File list
            let items: Vec<ListItem> = app
                .entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let name = if entry.is_dir {
                        format!("📁 {}/", entry.name)
                    } else {
                        format!("📄 {}", entry.name)
                    };

                    let size = if entry.is_dir {
                        String::from("DIR")
                    } else {
                        format_size(entry.size, BINARY)
                    };

                    let content = format!("{:<40} {:<12} {}", name, size, entry.modified);

                    let style = if i == app.selected_index {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else if entry.is_dir {
                        Style::default().fg(Color::Blue)
                    } else {
                        Style::default()
                    };

                    ListItem::new(Span::styled(content, style))
                })
                .collect();

            let files_list =
                List::new(items).block(Block::default().borders(Borders::ALL).title("Files"));
            f.render_widget(files_list, chunks[1]);

            // Info panel
            let help_text =
                "↑/↓: Navigate  Enter: Open dir/file  Backspace: Up  h: Toggle hidden  q: Quit";
            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(help, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('h') => app.toggle_hidden_files(),
                KeyCode::Up => {
                    if app.selected_index > 0 {
                        app.selected_index -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.selected_index < app.entries.len().saturating_sub(1) {
                        app.selected_index += 1;
                    }
                }
                KeyCode::Enter => {
                    if app.selected_index < app.entries.len() {
                        let is_dir = app.entries[app.selected_index].is_dir;
                        let path = app.entries[app.selected_index].path.clone();

                        if is_dir {
                            // Navigate to directory
                            app.navigate_to(&path);
                        } else {
                            // Open file in neovim
                            open_in_neovim(&path, terminal)?;
                        }
                    }
                }
                KeyCode::Backspace => app.navigate_up(),
                _ => {}
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = App::new(args.path, args.all);

    // Run the app
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
