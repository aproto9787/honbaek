use crate::config::AppPaths;
use crate::domain::InspectState;
use crate::ipc::{IpcRequest, IpcResponse};
use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::io::{self, IsTerminal};
use std::time::Duration;

pub fn watch(paths: &AppPaths, once: bool) -> Result<()> {
    let state = inspect(paths)?;
    if once || !io::stdout().is_terminal() {
        print_watch_snapshot(&state);
        return Ok(());
    }

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut result = run_watch_loop(paths);
    if let Err(error) = disable_raw_mode()
        && result.is_ok()
    {
        result = Err(error.into());
    }
    if let Err(error) = io::stdout().execute(LeaveAlternateScreen)
        && result.is_ok()
    {
        result = Err(error.into());
    }
    result
}

pub fn print_inspect(state: &InspectState) {
    println!("혼백강령 inspect");
    println!("魂 instances: {}", state.hons.len());
    for hon in &state.hons {
        println!(
            "- 魂 {} {} profile={} active={}",
            hon.name, hon.id, hon.profile, hon.active
        );
    }
    println!(
        "魄 provider={} model={} configured={}",
        state.baek.provider, state.baek.model, state.baek.configured
    );
    println!(
        "心 intent={:?} priority={} self_check={}",
        state.sim.current_intent, state.sim.priority, state.sim.self_check
    );
    println!(
        "身 filesystem={} shell={} network={}",
        state.shin.filesystem, state.shin.shell, state.shin.network
    );
    println!(
        "命 identity={} continuity_events={}",
        state.myeong.identity, state.myeong.continuity_events
    );
    println!("怪異 records: {}", state.kaeyi.len());
    for kaeyi in state.kaeyi.iter().take(5) {
        println!(
            "- 怪異 {} [{} {}] {} source={}",
            kaeyi.id, kaeyi.severity, kaeyi.state, kaeyi.title, kaeyi.source_kind
        );
    }
    println!("tasks:");
    for task in &state.tasks {
        println!("- {} {} {}", task.id, task.status, task.prompt);
        if let Some(result) = &task.result {
            println!("  result: {result}");
        }
    }
    println!("recent events:");
    for event in state.events.iter().take(8) {
        println!(
            "- {} {} {}",
            event.concept.hanja(),
            event.kind,
            event.message
        );
    }
    println!("tool calls: {}", state.tool_calls.len());
    println!("provider usage records: {}", state.provider_usage.len());
}

fn print_watch_snapshot(state: &InspectState) {
    println!("혼백강령 watch");
    println!("timeline:");
    for event in state.events.iter().take(12) {
        println!(
            "- {} [{}] {}",
            event.concept.hanja(),
            event.kind,
            event.message
        );
    }
    println!("current task:");
    if let Some(task) = state.tasks.first() {
        println!("- {} {} {}", task.id, task.status, task.prompt);
    } else {
        println!("- none");
    }
    println!("last action:");
    if let Some(call) = state.tool_calls.first() {
        println!("- {} ok={} {}", call.tool, call.ok, call.input);
    } else {
        println!("- none");
    }
    println!("provider usage: {}", state.provider_usage.len());
    println!("failure recovery: journaled events available");
    println!("怪異 summary:");
    if let Some(kaeyi) = state.kaeyi.first() {
        println!(
            "- {} [{} {}] {} source={}",
            kaeyi.id, kaeyi.severity, kaeyi.state, kaeyi.title, kaeyi.source_kind
        );
    } else {
        println!("- none");
    }
}

fn inspect(paths: &AppPaths) -> Result<InspectState> {
    match crate::ipc::send(&paths.socket, &IpcRequest::Inspect)? {
        IpcResponse::Inspect { state } => Ok(*state),
        IpcResponse::Error { message } => anyhow::bail!("{message}"),
        _ => anyhow::bail!("daemon returned unexpected response"),
    }
}

fn run_watch_loop(paths: &AppPaths) -> Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    loop {
        let state = inspect(paths)?;
        terminal.draw(|frame| draw(frame, &state))?;
        if should_exit_watch()? {
            return Ok(());
        }
    }
}

fn should_exit_watch() -> Result<bool> {
    if !event::poll(Duration::from_millis(500))? {
        return Ok(false);
    }
    match event::read()? {
        Event::Key(key)
            if key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))) =>
        {
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, state: &InspectState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Length(8),
        ])
        .split(frame.area());

    let title = Paragraph::new("혼백강령 watch — 魂 魄 心 身 命")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let events: Vec<ListItem<'_>> = state
        .events
        .iter()
        .take(10)
        .map(|event| {
            ListItem::new(Line::from(format!(
                "{} [{}] {}",
                event.concept.hanja(),
                event.kind,
                event.message
            )))
        })
        .collect();
    frame.render_widget(
        List::new(events).block(Block::default().title("timeline").borders(Borders::ALL)),
        chunks[1],
    );

    let current = state
        .tasks
        .first()
        .map(|task| {
            format!(
                "task={} status={} prompt={}",
                task.id, task.status, task.prompt
            )
        })
        .unwrap_or_else(|| "no active task".to_string());
    frame.render_widget(
        Paragraph::new(current).block(Block::default().title("current").borders(Borders::ALL)),
        chunks[2],
    );

    let kaeyi_lines = render_kaeyi_lines(state);
    frame.render_widget(
        Paragraph::new(kaeyi_lines).block(Block::default().title("怪異").borders(Borders::ALL)),
        chunks[3],
    );

    let last_action = state
        .tool_calls
        .first()
        .map(|call| format!("last action: {} ok={} {}", call.tool, call.ok, call.input))
        .unwrap_or_else(|| "last action: none".to_string());
    let tool_calls = format!("tool calls: {}", state.tool_calls.len());
    let provider_usage = state
        .provider_usage
        .first()
        .map(|usage| {
            format!(
                "provider usage: {} records, latest {} {} in={} out={}",
                state.provider_usage.len(),
                usage.provider,
                usage.model,
                usage.prompt_tokens,
                usage.completion_tokens
            )
        })
        .unwrap_or_else(|| "provider usage: 0 records".to_string());
    let failure_recovery = state
        .tasks
        .iter()
        .find(|task| matches!(task.status, crate::domain::TaskStatus::Failed))
        .map(|task| format!("failure recovery: failed task {} journaled", task.id))
        .unwrap_or_else(|| "failure recovery: journaled events available".to_string());
    let status = vec![
        Line::from(last_action),
        Line::from(tool_calls),
        Line::from(provider_usage),
        Line::from(failure_recovery),
    ];
    frame.render_widget(
        Paragraph::new(status).block(Block::default().title("status").borders(Borders::ALL)),
        chunks[4],
    );
}

fn render_kaeyi_lines(state: &InspectState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(format!("records: {}", state.kaeyi.len()))];
    if let Some(kaeyi) = state.kaeyi.first() {
        lines.push(Line::from(format!(
            "latest: {} [{} {}]",
            kaeyi.title, kaeyi.severity, kaeyi.state
        )));
        lines.push(Line::from(format!("source: {}", kaeyi.source_kind)));
    } else {
        lines.push(Line::from("latest: none"));
    }
    lines
}
