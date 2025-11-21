mod models;
mod storage;

use crate::models::Ledger;
use crate::storage::Storage;
use anyhow::{Context, Result, anyhow};
use chrono::{Local, NaiveDate};
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    BarChart, Block, Borders, Cell, Chart, Dataset, Paragraph, Row, Table, Tabs, Wrap,
};
use std::io::{Stdout, stdout};
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    let mut app = App::new()?;
    let res = run(&mut app);
    if let Err(err) = res {
        eprintln!("Application error: {err:?}");
        std::process::exit(1);
    }
    Ok(())
}

fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, app);

    disable_raw_mode()?;
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor()?;

    result
}

struct App {
    ledger: Ledger,
    storage: Storage,
    active_tab: usize,
    form: ActiveForm,
    show_suggestions: bool,
    last_message: String,
    last_save: Option<Instant>,
}

impl App {
    fn new() -> Result<Self> {
        let storage = Storage::new()?;
        let ledger = storage.load()?;
        Ok(Self {
            ledger,
            storage,
            active_tab: 0,
            form: ActiveForm::None,
            show_suggestions: true,
            last_message: "Loaded data".to_string(),
            last_save: None,
        })
    }

    fn save(&mut self) -> Result<()> {
        self.storage
            .save(&self.ledger)
            .context("saving ledger failed")?;
        self.last_save = Some(Instant::now());
        self.last_message = format!("Saved to {}", self.storage.path().display());
        Ok(())
    }
}

enum ActiveForm {
    None,
    Transaction(TxForm),
    Budget(BudgetForm),
}

#[derive(Clone)]
struct Field {
    label: &'static str,
    value: String,
}

struct TxForm {
    fields: Vec<Field>,
    index: usize,
}

impl TxForm {
    fn new() -> Self {
        let today = Local::now().naive_local().date();
        Self {
            fields: vec![
                Field {
                    label: "Description",
                    value: String::new(),
                },
                Field {
                    label: "Amount (+out / -in)",
                    value: String::new(),
                },
                Field {
                    label: "Category",
                    value: String::from("General"),
                },
                Field {
                    label: "Date (YYYY-MM-DD)",
                    value: today.to_string(),
                },
            ],
            index: 0,
        }
    }

    fn current_mut(&mut self) -> &mut Field {
        &mut self.fields[self.index]
    }

    fn next(&mut self) {
        if self.index + 1 < self.fields.len() {
            self.index += 1;
        }
    }

    fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    fn push_char(&mut self, c: char) {
        self.current_mut().value.push(c);
    }

    fn backspace(&mut self) {
        self.current_mut().value.pop();
    }

    fn try_submit(&self) -> Result<NewTransaction> {
        let description = self.fields[0].value.trim();
        let amount_str = self.fields[1].value.trim();
        let category = self.fields[2].value.trim();
        let date_str = self.fields[3].value.trim();

        if description.is_empty() {
            return Err(anyhow!("Description is required"));
        }
        if amount_str.is_empty() {
            return Err(anyhow!("Amount is required"));
        }
        let amount: f64 = amount_str
            .parse()
            .context("Amount must be a number (use negative for income)")?;
        let date = if date_str.is_empty() {
            Local::now().naive_local().date()
        } else {
            NaiveDate::parse_from_str(date_str, "%Y-%m-%d").context("Date must be YYYY-MM-DD")?
        };

        Ok(NewTransaction {
            description: description.to_string(),
            amount,
            category: if category.is_empty() {
                "General".to_string()
            } else {
                category.to_string()
            },
            date,
        })
    }
}

struct BudgetForm {
    fields: Vec<Field>,
    index: usize,
}

impl BudgetForm {
    fn new() -> Self {
        Self {
            fields: vec![
                Field {
                    label: "Category",
                    value: "General".to_string(),
                },
                Field {
                    label: "Monthly limit",
                    value: String::new(),
                },
            ],
            index: 0,
        }
    }

    fn current_mut(&mut self) -> &mut Field {
        &mut self.fields[self.index]
    }

    fn next(&mut self) {
        if self.index + 1 < self.fields.len() {
            self.index += 1;
        }
    }

    fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    fn push_char(&mut self, c: char) {
        self.current_mut().value.push(c);
    }

    fn backspace(&mut self) {
        self.current_mut().value.pop();
    }

    fn try_submit(&self) -> Result<NewBudget> {
        let category = self.fields[0].value.trim();
        let limit = self.fields[1].value.trim();
        if category.is_empty() {
            return Err(anyhow!("Category is required"));
        }
        let monthly_limit: f64 = limit
            .parse()
            .context("Monthly limit must be a number (no $ sign)")?;
        Ok(NewBudget {
            category: category.to_string(),
            monthly_limit,
        })
    }
}

struct NewTransaction {
    description: String,
    amount: f64,
    category: String,
    date: NaiveDate,
}

struct NewBudget {
    category: String,
    monthly_limit: f64,
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
            && handle_key(app, key)?
        {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match &mut app.form {
        ActiveForm::Transaction(form) => match key.code {
            KeyCode::Esc => {
                app.form = ActiveForm::None;
                app.last_message = "Cancelled transaction".into();
            }
            KeyCode::Tab => form.next(),
            KeyCode::BackTab => form.prev(),
            KeyCode::Enter => {
                if form.index + 1 < form.fields.len() {
                    form.next();
                } else {
                    match form.try_submit() {
                        Ok(tx) => {
                            app.ledger.add_transaction(
                                tx.description,
                                tx.amount,
                                tx.category,
                                tx.date,
                            );
                            app.form = ActiveForm::None;
                            app.last_message = "Transaction added".into();
                            app.save().ok(); // best effort
                        }
                        Err(err) => app.last_message = err.to_string(),
                    }
                }
            }
            KeyCode::Backspace => form.backspace(),
            KeyCode::Left => form.prev(),
            KeyCode::Right => form.next(),
            KeyCode::Char(c) => form.push_char(c),
            _ => {}
        },
        ActiveForm::Budget(form) => match key.code {
            KeyCode::Esc => {
                app.form = ActiveForm::None;
                app.last_message = "Cancelled budget edit".into();
            }
            KeyCode::Tab => form.next(),
            KeyCode::BackTab => form.prev(),
            KeyCode::Enter => {
                if form.index + 1 < form.fields.len() {
                    form.next();
                } else {
                    match form.try_submit() {
                        Ok(budget) => {
                            app.ledger
                                .add_or_update_budget(budget.category, budget.monthly_limit);
                            app.form = ActiveForm::None;
                            app.last_message = "Budget saved".into();
                            app.save().ok();
                        }
                        Err(err) => app.last_message = err.to_string(),
                    }
                }
            }
            KeyCode::Backspace => form.backspace(),
            KeyCode::Left => form.prev(),
            KeyCode::Right => form.next(),
            KeyCode::Char(c) => form.push_char(c),
            _ => {}
        },
        ActiveForm::None => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('h') => {
                if app.active_tab > 0 {
                    app.active_tab -= 1;
                }
            }
            KeyCode::Char('l') => {
                if app.active_tab < 2 {
                    app.active_tab += 1;
                }
            }
            KeyCode::Char('a') => app.form = ActiveForm::Transaction(TxForm::new()),
            KeyCode::Char('b') => app.form = ActiveForm::Budget(BudgetForm::new()),
            KeyCode::Char('s') => {
                app.save()?;
            }
            KeyCode::Char('g') => app.show_suggestions = !app.show_suggestions,
            KeyCode::Char('r') => {
                app.ledger = app.storage.load()?;
                app.last_message = "Reloaded data".into();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            _ => {}
        },
    }

    Ok(false)
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(f.size());

    let top = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(layout[0]);

    render_header(f, top[0], app);

    let tab_titles = ["Overview", "Transactions", "Budgets"]
        .iter()
        .map(|t| Line::from(*t))
        .collect::<Vec<_>>();
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .select(app.active_tab);
    f.render_widget(tabs, top[1]);

    match app.active_tab {
        0 => render_overview(f, layout[1], &app.ledger),
        1 => render_transactions(f, layout[1], &app.ledger),
        _ => render_budgets(f, layout[1], &app.ledger, app.show_suggestions),
    }

    render_footer(f, layout[2], app);
}

fn render_header(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "centsh",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  | data "),
        Span::styled(
            app.storage.path().to_string_lossy(),
            Style::default().fg(Color::Gray),
        ),
    ]))
    .wrap(Wrap { trim: true });
    f.render_widget(header, area);
}

fn render_overview(f: &mut ratatui::Frame, area: Rect, ledger: &Ledger) {
    let overview = ledger.current_month_overview();
    let cat_spend = ledger.category_spending_current_month();
    let budgets = ledger.budgets_by_category();
    let cashflow = ledger.spending_last_n_months(6);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

    let stats_block = Block::default().title("This month").borders(Borders::ALL);
    let stats_lines = vec![
        Line::from(format!(
            "Income: {}",
            format_currency(overview.total_income)
        )),
        Line::from(format!(
            "Spending: {}",
            format_currency(overview.total_outgoing)
        )),
        Line::from(vec![Span::raw("Net: "), styled_net(overview.net)]),
        Line::from(" "),
        Line::from("Budgets:"),
    ];

    let mut budget_lines = stats_lines;
    let mut rows: Vec<Line> = budgets
        .iter()
        .map(|(cat, limit)| {
            let spent = cat_spend
                .iter()
                .find(|(c, _)| c == cat)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);
            let pct = if *limit > 0.0 {
                (spent / limit * 100.0).min(999.0)
            } else {
                0.0
            };
            Line::from(format!(
                "- {cat}: {} / {} ({pct:.0}%)",
                format_currency(spent),
                format_currency(*limit)
            ))
        })
        .collect();
    if rows.is_empty() {
        rows.push(Line::from("No budgets yet. Press b to add one."));
    }
    budget_lines.extend(rows);
    let stats = Paragraph::new(budget_lines).block(stats_block);
    f.render_widget(stats, chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)].as_ref())
        .split(chunks[1]);

    render_category_chart(f, right_chunks[0], cat_spend);
    render_cashflow_chart(f, right_chunks[1], cashflow);
}

fn render_category_chart(f: &mut ratatui::Frame, area: Rect, cat_spend: Vec<(String, f64)>) {
    let data: Vec<(&str, u64)> = cat_spend
        .iter()
        .map(|(cat, amt)| (cat.as_str(), amt.max(0.0) as u64))
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .title("Category spend (this month)")
                .borders(Borders::ALL),
        )
        .bar_width(8)
        .data(&data)
        .value_style(Style::default().fg(Color::Yellow))
        .label_style(Style::default().fg(Color::White));
    f.render_widget(chart, area);
}

fn render_cashflow_chart(f: &mut ratatui::Frame, area: Rect, cashflow: Vec<(String, f64)>) {
    let data: Vec<(f64, f64)> = cashflow
        .iter()
        .enumerate()
        .map(|(i, (_, v))| (i as f64, *v))
        .collect();

    let labels: Vec<Span> = cashflow
        .iter()
        .map(|(label, _)| Span::raw(label.clone()))
        .collect();

    let dataset = vec![
        Dataset::default()
            .name("Net by month")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .data(&data),
    ];

    let chart = Chart::new(dataset)
        .block(Block::default().title("Cashflow").borders(Borders::ALL))
        .x_axis(
            ratatui::widgets::Axis::default()
                .bounds([0.0, data.len().max(1) as f64])
                .labels(labels),
        )
        .y_axis(
            ratatui::widgets::Axis::default()
                .bounds([
                    data.iter().map(|(_, y)| *y).fold(0.0, f64::min) - 50.0,
                    data.iter().map(|(_, y)| *y).fold(0.0, f64::max) + 50.0,
                ])
                .labels(vec![Span::raw("-"), Span::raw("0"), Span::raw("+")]),
        );
    f.render_widget(chart, area);
}

fn render_transactions(f: &mut ratatui::Frame, area: Rect, ledger: &Ledger) {
    let header = Row::new(vec!["Date", "Description", "Category", "Amount"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = ledger
        .transactions
        .iter()
        .take(18)
        .map(|tx| {
            Row::new(vec![
                Cell::from(tx.date.to_string()),
                Cell::from(tx.description.clone()),
                Cell::from(tx.category.clone()),
                Cell::from(styled_amount(tx.amount)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Percentage(40),
        Constraint::Length(14),
        Constraint::Length(12),
    ];
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title("Recent transactions")
            .borders(Borders::ALL),
    );

    f.render_widget(table, area);
}

fn render_budgets(f: &mut ratatui::Frame, area: Rect, ledger: &Ledger, show_suggestions: bool) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(area);

    let rows: Vec<Row> = ledger
        .budgets
        .iter()
        .map(|b| {
            Row::new(vec![
                Cell::from(b.category.clone()),
                Cell::from(format_currency(b.monthly_limit)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        &[Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .header(
        Row::new(vec!["Category", "Monthly limit"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().title("Budgets").borders(Borders::ALL));
    f.render_widget(table, chunks[0]);

    let suggestion_block = Block::default()
        .title("Auto-budgets (90d trend)")
        .borders(Borders::ALL);

    if show_suggestions {
        let suggestions = ledger.suggested_budgets();
        let lines: Vec<Line> = suggestions
            .into_iter()
            .map(|s| {
                Line::from(format!(
                    "{}: {} ({})",
                    s.category,
                    if s.suggested_limit > 0.0 {
                        format_currency(s.suggested_limit)
                    } else {
                        "add target".into()
                    },
                    s.reason
                ))
            })
            .collect();
        let paragraph = Paragraph::new(lines)
            .block(suggestion_block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, chunks[1]);
    } else {
        let paragraph = Paragraph::new("Press g to show auto-budget ideas").block(suggestion_block);
        f.render_widget(paragraph, chunks[1]);
    }
}

fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &App) {
    if let ActiveForm::Transaction(form) = &app.form {
        render_form(f, area, "Add transaction", form.fields.clone(), form.index);
        return;
    }
    if let ActiveForm::Budget(form) = &app.form {
        render_form(f, area, "Add budget", form.fields.clone(), form.index);
        return;
    }

    let last_saved = app
        .last_save
        .map(|_| "Saved recently".to_string())
        .unwrap_or_default();
    let footer = Paragraph::new(Line::from(vec![
        Span::raw(
            "q quit  a add txn  b add budget  h/l tabs  s save  g toggle auto-budget  r reload  ",
        ),
        Span::styled(last_saved, Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(&app.last_message, Style::default().fg(Color::Yellow)),
    ]))
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, area);
}

fn render_form(f: &mut ratatui::Frame, area: Rect, title: &str, fields: Vec<Field>, index: usize) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let label = if i == index {
            Span::styled(
                field.label,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(field.label)
        };
        lines.push(Line::from(vec![
            label,
            Span::raw(": "),
            Span::raw(field.value.clone()),
        ]));
    }
    lines.push(Line::from("Enter: next/submit   Tab: next   Esc: cancel"));
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn format_currency(value: f64) -> String {
    if value.is_sign_negative() {
        format!("-${:.2}", value.abs())
    } else {
        format!("${:.2}", value)
    }
}

fn styled_amount(amount: f64) -> Span<'static> {
    let color = if amount >= 0.0 {
        Color::Red
    } else {
        Color::Green
    };
    Span::styled(format_currency(amount), Style::default().fg(color))
}

fn styled_net(net: f64) -> Span<'static> {
    let color = if net >= 0.0 { Color::Green } else { Color::Red };
    Span::styled(format_currency(net), Style::default().fg(color))
}
