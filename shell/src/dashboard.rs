#[cfg(feature = "nova")]
/// The TUI Dashboard module.
pub mod tui {
    use anyhow::Result;
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::{Backend, CrosstermBackend},
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, Borders, Paragraph},
        Frame, Terminal,
    };
    use std::{io, sync::Arc, time::Duration};
    use tardis_gallifrey::{experimental::heatmap::TemporalHeatmap, Gallifrey};

    /// The interactive dashboard.
    #[derive(Debug)]
    pub struct Dashboard {
        _gallifrey: Arc<Gallifrey>,
        heatmap: TemporalHeatmap,
    }

    impl Dashboard {
        /// Create a new dashboard.
        ///
        /// # Errors
        ///
        /// Returns an error if history cannot be scanned.
        pub fn new(gallifrey: Arc<Gallifrey>) -> Result<Self> {
            // Fetch history for heatmap
            let mut all_history = Vec::new();

            // We need to access knowledge store directly.
            // scan_history is available because we enabled 'nova' feature in shell/Cargo.toml
            // which enables 'tardis-gallifrey/nova' which enables 'scan_history'.
            gallifrey.knowledge().scan_history(|history| {
                all_history.extend_from_slice(history);
            })?;

            // 50x20 resolution for the heatmap
            let heatmap = TemporalHeatmap::new(&all_history, 50, 20);

            Ok(Self { _gallifrey: gallifrey, heatmap })
        }

        /// Run the dashboard loop.
        ///
        /// # Errors
        ///
        /// Returns an error if terminal setup/teardown fails.
        pub fn run(&mut self) -> Result<()> {
            // Setup terminal
            enable_raw_mode()?;
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;

            // Run loop
            let res = self.run_app(&mut terminal);

            // Restore terminal
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            if let Err(err) = res {
                println!("Error running dashboard: {err:?}");
            }

            Ok(())
        }

        fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
            loop {
                terminal
                    .draw(|f| self.ui(f))
                    .map_err(|e| io::Error::other(e.to_string()))?;

                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(key) = event::read()? {
                        if let KeyCode::Char('q') = key.code {
                            return Ok(());
                        }
                    }
                }
            }
        }

        fn ui(&self, f: &mut Frame) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3), // Title
                        Constraint::Min(0),    // Content
                        Constraint::Length(3), // Footer
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Title
            let title = Paragraph::new(Text::styled(
                "🌟 Tardis Time Stream 🌟",
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
            ))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Content (Heatmap + Stats)
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                .split(chunks[1]);

            // Heatmap
            let heatmap_str = self.heatmap.render_ascii();
            let heatmap_widget = Paragraph::new(heatmap_str)
                .block(Block::default().title("Bi-Temporal Activity").borders(Borders::ALL));
            f.render_widget(heatmap_widget, main_chunks[0]);

            // Stats
            let total_events: usize = self.heatmap.grid.iter().flatten().sum();

            let stats_text = vec![
                Line::from(Span::styled("System Status", Style::default().add_modifier(Modifier::UNDERLINED))),
                Line::from(""),
                Line::from(format!("Entities: {total_events}")), // Rough proxy for activity
                Line::from(format!("Valid Time: {} to {}", self.heatmap.valid_range.0.format("%H:%M"), self.heatmap.valid_range.1.format("%H:%M"))),
                Line::from(format!("Trans Time: {} to {}", self.heatmap.transaction_range.0.format("%H:%M"), self.heatmap.transaction_range.1.format("%H:%M"))),
            ];
            let stats_widget = Paragraph::new(stats_text)
                .block(Block::default().title("Stats").borders(Borders::ALL));
            f.render_widget(stats_widget, main_chunks[1]);

            // Footer
            let footer = Paragraph::new("Press 'q' to exit")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        }
    }
}
