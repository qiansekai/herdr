//! `ui.sidebar.wheel`: cycling focus instead of scrolling list contents.

use super::*;

fn endpoint_method(outcome: &ClientShellInput) -> Option<&crate::api::schema::Method> {
    outcome.actions.iter().find_map(|action| match action {
        ClientShellAction::Endpoint { request, .. } => Some(&request.method),
        _ => None,
    })
}

fn wheel(kind: MouseEventKind, column: u16, row: u16) -> RawInputEvent {
    RawInputEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

fn agent(pane_id: &str, workspace_id: &str, tab_id: &str, seq: u64) -> ClientShellAgent {
    ClientShellAgent {
        pane_id: pane_id.into(),
        workspace_id: workspace_id.into(),
        tab_id: tab_id.into(),
        name: Some(pane_id.into()),
        display_agent: None,
        agent: Some("claude".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: AgentStatus::Idle,
        state_change_seq: seq,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: false,
    }
}

/// A second workspace so wheel switching has somewhere to go. The default
/// `AgentPanelSortConfig::Spaces` keeps agent order identical to the snapshot.
fn snapshot_with_two_workspaces() -> ClientShellSnapshot {
    let mut snapshot = snapshot();
    snapshot.workspaces.push(ClientShellWorkspace {
        workspace_id: "ws_2".into(),
        active_tab_id: "tab_2".into(),
        new_workspace_cwd: "/repo-two".into(),
        number: 2,
        label: "second".into(),
        custom_label: false,
        branch: None,
        git_ahead_behind: None,
        tokens: Vec::new(),
        worktree: None,
        focused: false,
        agent_status: AgentStatus::Idle,
    });
    snapshot.tabs.push(ClientShellTab {
        tab_id: "tab_2".into(),
        workspace_id: "ws_2".into(),
        number: 1,
        label: "1".into(),
        custom_label: false,
        zoomed: false,
        focused: false,
        agent_status: AgentStatus::Idle,
    });
    snapshot.panes.push(ClientShellPane {
        pane_id: "pane_2".into(),
        workspace_id: "ws_2".into(),
        tab_id: "tab_2".into(),
        label: None,
        cwd: Some("/repo-two".into()),
        foreground_cwd: Some("/repo-two".into()),
        focused: false,
        right_click_passthrough: false,
    });
    snapshot.agents = vec![
        agent("pane_1", "ws_1", "tab_1", 1),
        agent("pane_2", "ws_2", "tab_2", 2),
    ];
    snapshot
}

fn shell_state(snapshot: ClientShellSnapshot) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state
}

#[test]
fn wheel_over_workspace_list_switches_workspace() {
    let mut state = shell_state(snapshot_with_two_workspaces());
    state.compose(106, 30).expect("sidebar frame");
    let body = state.hits.workspace_body;
    assert!(body.height > 0, "workspace list must be rendered");

    let down = state.handle_raw_events(vec![wheel(MouseEventKind::ScrollDown, body.x, body.y)]);
    assert!(
        matches!(
            endpoint_method(&down),
            Some(crate::api::schema::Method::WorkspaceFocus(target)) if target.workspace_id == "ws_2"
        ),
        "scroll down should focus the next workspace: {:?}",
        endpoint_method(&down)
    );

    // Two workspaces: previous wraps around to the same neighbour.
    let up = state.handle_raw_events(vec![wheel(MouseEventKind::ScrollUp, body.x, body.y)]);
    assert!(
        matches!(
            endpoint_method(&up),
            Some(crate::api::schema::Method::WorkspaceFocus(target)) if target.workspace_id == "ws_2"
        ),
        "scroll up should wrap to the last workspace: {:?}",
        endpoint_method(&up)
    );
}

#[test]
fn wheel_over_agent_list_switches_agent() {
    let mut state = shell_state(snapshot_with_two_workspaces());
    state.compose(106, 30).expect("sidebar frame");
    let body = state.hits.agent_body;
    assert!(body.height > 0, "agent list must be rendered");

    // Ordered agents are [pane_1, pane_2] with pane_1 focused.
    let down = state.handle_raw_events(vec![wheel(MouseEventKind::ScrollDown, body.x, body.y)]);
    assert!(
        matches!(
            endpoint_method(&down),
            Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "pane_2"
        ),
        "scroll down should focus the next agent: {:?}",
        endpoint_method(&down)
    );

    let up = state.handle_raw_events(vec![wheel(MouseEventKind::ScrollUp, body.x, body.y)]);
    assert!(
        matches!(
            endpoint_method(&up),
            Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "pane_2"
        ),
        "scroll up should wrap to the last agent: {:?}",
        endpoint_method(&up)
    );
}

#[test]
fn wheel_scroll_mode_keeps_list_scrolling() {
    let mut state = shell_state(snapshot_with_two_workspaces());
    state.config.sidebar_wheel = crate::config::SidebarWheelConfig::Scroll;
    state.compose(106, 30).expect("sidebar frame");

    let workspace_body = state.hits.workspace_body;
    let workspace_wheel = state.handle_raw_events(vec![wheel(
        MouseEventKind::ScrollDown,
        workspace_body.x,
        workspace_body.y,
    )]);
    assert!(
        endpoint_method(&workspace_wheel).is_none(),
        "scroll mode must not switch workspaces"
    );

    let agent_body = state.hits.agent_body;
    let agent_wheel = state.handle_raw_events(vec![wheel(
        MouseEventKind::ScrollDown,
        agent_body.x,
        agent_body.y,
    )]);
    assert!(
        endpoint_method(&agent_wheel).is_none(),
        "scroll mode must not switch agents"
    );
}
