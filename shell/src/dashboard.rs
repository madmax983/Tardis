#[cfg(feature = "nova")]
/// The TUI Dashboard module.
pub mod tui {
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::{Backend, CrosstermBackend},
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        symbols,
        text::{Line, Span, Text},
        widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, Paragraph},
        Frame, Terminal,
    };
    use std::{
        io,
        sync::{Arc, Mutex},
        time::Duration,
    };
    #[cfg(feature = "nova")]
    use tardis_chronos::experimental::prophecy::Prophet;
    use tardis_gallifrey::domain::Entity;
    use tardis_gallifrey::experimental::heatmap::TemporalHeatmap;
    use tardis_gallifrey::GallifreyService;
    use tardis_telemetry::{gallifrey::TelemetryStore, types::MetricValue};

    /// Shared data for the dashboard.
    #[derive(Debug, Default)]
    struct DashboardData {
        prophecies: Vec<Entity>,
        last_update: Option<DateTime<Utc>>,
    }

    /// The interactive dashboard.
    #[derive(Debug)]
    pub struct Dashboard {
        _gallifrey: Arc<dyn GallifreyService>,
        heatmap: TemporalHeatmap,
        telemetry: Option<Arc<TelemetryStore>>,
        data: Arc<Mutex<DashboardData>>,
        update_handle: Option<tokio::task::JoinHandle<()>>,
    }

    impl Drop for Dashboard {
        fn drop(&mut self) {
            if let Some(handle) = &self.update_handle {
                handle.abort();
            }
        }
    }

    impl Dashboard {
        /// Create a new dashboard.
        ///
        /// # Errors
        ///
        /// Returns an error if history cannot be scanned.
        pub fn new(
            gallifrey: Arc<dyn GallifreyService>,
            telemetry: Option<Arc<TelemetryStore>>,
            #[cfg(feature = "nova")] prophet: Option<Arc<Prophet>>,
        ) -> Result<Self> {
            // Fetch history for heatmap
            let all_history = Arc::new(Mutex::new(Vec::new()));
            let all_history_clone = all_history.clone();

            gallifrey.scan_history(Box::new(move |history| {
                if let Ok(mut guard) = all_history_clone.lock() {
                    guard.extend_from_slice(history);
                }
            }))?;

            let history_vec = {
                let guard = all_history.lock().map_err(|e| anyhow::anyhow!("Mutex error: {e}"))?;
                guard.clone()
            };

            let heatmap = TemporalHeatmap::new(&history_vec, 50, 20);
            let data = Arc::new(Mutex::new(DashboardData::default()));

            // Spawn background task for prophecies
            #[cfg(feature = "nova")]
            let update_handle = if let Some(p) = prophet {
                let data_clone = data.clone();
                Some(tokio::spawn(async move {
                    loop {
                        // Mock model handle for now
                        let model = tardis_common::id::ModelHandle::new(0);
                        // Predict 10 minutes into the future
                        if let Ok(predictions) = p
                            .foresee(
                                "System running normally. Telemetry active.",
                                Duration::from_secs(600),
                                model,
                            )
                            .await
                        {
                            if let Ok(mut lock) = data_clone.lock() {
                                lock.prophecies = predictions;
                                lock.last_update = Some(Utc::now());
                            }
                        }
                        // Refresh every 30 seconds
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                }))
            } else {
                None
            };

            #[cfg(not(feature = "nova"))]
            let update_handle = None;

            Ok(Self {
                _gallifrey: gallifrey,
                heatmap,
                telemetry,
                data,
                update_handle,
            })
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

        #[allow(
            clippy::too_many_lines,
            clippy::cast_precision_loss,
            clippy::manual_range_contains,
            clippy::cast_lossless
        )]
        fn ui(&self, f: &mut Frame) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3), // Title
                        Constraint::Min(0),    // Content (Heatmap + Lower Panel)
                        Constraint::Length(3), // Footer
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Title
            let title = Paragraph::new(Text::styled(
                "🌟 Tardis Holodeck 🌟",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            ))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Split Content into Top (Heatmap) and Bottom (Telemetry + Prophecies)
            let content_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(chunks[1]);

            // --- Top: Heatmap ---
            let heatmap_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                .split(content_chunks[0]);

            let heatmap_str = self.heatmap.render_ascii();
            let heatmap_widget = Paragraph::new(heatmap_str).block(
                Block::default()
                    .title("Bi-Temporal History")
                    .borders(Borders::ALL),
            );
            f.render_widget(heatmap_widget, heatmap_chunks[0]);

            let total_events: usize = self.heatmap.grid.iter().flatten().sum();
            let stats_text = vec![
                Line::from(Span::styled(
                    "Archive Stats",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )),
                Line::from(""),
                Line::from(format!("Entities: {total_events}")),
                Line::from(format!(
                    "Valid: {} - {}",
                    self.heatmap.valid_range.0.format("%H:%M"),
                    self.heatmap.valid_range.1.format("%H:%M")
                )),
            ];
            let stats_widget = Paragraph::new(stats_text)
                .block(Block::default().title("Archive").borders(Borders::ALL));
            f.render_widget(stats_widget, heatmap_chunks[1]);

            // --- Bottom: Telemetry & Prophecies ---
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                .split(content_chunks[1]);

            // Telemetry Chart
            let mut datasets = Vec::new();
            let mut data_points = Vec::new();

            let mut chart_title = "Live Telemetry (Simulated)";

            if let Some(telemetry) = &self.telemetry {
                // Try to read real metrics
                let now = Utc::now();
                let from = now - chrono::Duration::seconds(100);

                // Look for standard metrics. If none, fallback to simulated.
                // We don't have a way to know metric names without scanning.
                // But let's assume "cpu_usage" exists if telemetry is active.
                let samples = telemetry.metric_history("cpu_usage", from, now);

                if !samples.is_empty() {
                    chart_title = "Live Telemetry (cpu_usage)";
                    let base_time = from.timestamp_millis() as f64 / 1000.0;
                    for s in samples {
                        let time_sec = s.timestamp_ns as f64 / 1_000_000_000.0;
                        // Normalize time to 0-100 range relative to window
                        let rel_time = time_sec - base_time;
                        if rel_time >= 0.0 && rel_time <= 100.0 {
                            let val = match s.value {
                                MetricValue::Counter(v) => v as f64,
                                MetricValue::Gauge(v) => v as f64,
                                MetricValue::Histogram { sum, count, .. } => {
                                    if count > 0 {
                                        sum / count as f64
                                    } else {
                                        0.0
                                    }
                                }
                            };
                            data_points.push((rel_time, val));
                        }
                    }
                }
            }

            if data_points.is_empty() {
                // Fallback mock data
                for i in 0..100 {
                    data_points.push((i as f64, (i as f64 / 10.0).sin()));
                }
            }

            let dataset = Dataset::default()
                .name("System Pulse")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Green))
                .data(&data_points);

            datasets.push(dataset);

            let chart = Chart::new(datasets)
                .block(Block::default().title(chart_title).borders(Borders::ALL))
                .x_axis(Axis::default().title("Time").bounds([0.0, 100.0]))
                .y_axis(Axis::default().title("Amplitude").bounds([-1.0, 1.0]));

            f.render_widget(chart, bottom_chunks[0]);

            // Prophecies List
            let mut items = Vec::new();
            if let Ok(lock) = self.data.lock() {
                if lock.prophecies.is_empty() {
                    items.push(ListItem::new("Consulting the Oracle..."));
                } else {
                    for p in &lock.prophecies {
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("🔮 ", Style::default().fg(Color::Magenta)),
                            Span::raw(format!("{}: {}", p.entity_type, p.name)),
                        ])));
                    }
                }
            } else {
                items.push(ListItem::new("Oracle Unreachable"));
            }

            let list = List::new(items).block(
                Block::default()
                    .title("Prophecies (Next 10m)")
                    .borders(Borders::ALL),
            );

            f.render_widget(list, bottom_chunks[1]);

            // Footer
            let footer =
                Paragraph::new("Press 'q' to exit").block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        }
    }
}
