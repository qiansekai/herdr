//! Context menu entries added by this fork: Paste, Enter, and Open folder.

use super::*;

fn endpoint_method(outcome: &ClientShellInput) -> Option<&crate::api::schema::Method> {
    outcome.actions.iter().find_map(|action| match action {
        ClientShellAction::Endpoint { request, .. } => Some(&request.method),
        _ => None,
    })
}

fn shell_state() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state
}

fn labels(state: &ClientShellState) -> Vec<&'static str> {
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("context menu overlay expected");
    };
    menu.items().into_iter().map(|item| item.label).collect()
}

#[test]
fn pane_context_menu_lists_paste_and_enter_first() {
    let mut state = shell_state();
    state.open_pane_context_menu("pane_1".into(), 0, 0);
    let labels = labels(&state);
    assert_eq!(labels.first(), Some(&"Paste"));
    assert_eq!(labels.get(1), Some(&"Enter"));
    assert!(labels.contains(&"Rename pane"));
}

#[test]
fn workspace_context_menu_lists_open_folder_first() {
    let mut state = shell_state();
    state.open_workspace_context_menu("ws_1".into(), 0, 0);
    let labels = labels(&state);
    assert_eq!(labels.first(), Some(&"Open folder"));
    assert!(labels.contains(&"Rename"));
}

#[test]
fn pane_context_menu_enter_sends_enter_to_that_pane() {
    let mut state = shell_state();
    state.open_pane_context_menu("pane_1".into(), 0, 0);
    let index = labels(&state)
        .iter()
        .position(|label| *label == "Enter")
        .expect("enter item");

    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);

    assert!(
        matches!(
            endpoint_method(&outcome),
            Some(crate::api::schema::Method::PaneSendKeys(params))
                if params.pane_id == "pane_1" && params.keys == vec!["enter".to_string()]
        ),
        "enter should target the clicked pane: {:?}",
        endpoint_method(&outcome)
    );
    assert!(state.overlay.is_none(), "menu closes after activation");
}

#[test]
fn paste_text_into_pane_targets_the_clicked_pane() {
    let state = shell_state();
    let mut outcome = ClientShellInput::default();
    state.paste_text_into_pane("pane_1".into(), "multi\nline".into(), &mut outcome);

    let pasted = outcome.requests.iter().find_map(|message| match message {
        ClientMessage::ClientShellPaneInput { pane_id, events } => {
            Some((pane_id.clone(), events.clone()))
        }
        _ => None,
    });
    let Some((pane_id, events)) = pasted else {
        panic!("paste should target a pane: {:?}", outcome.requests);
    };
    assert_eq!(pane_id, "pane_1");
    assert!(
        matches!(
            events.as_slice(),
            [crate::protocol::ClientPaneInputEvent::Paste(text)] if text == "multi\nline"
        ),
        "{events:?}"
    );
}

#[test]
fn workspace_context_menu_open_folder_ignores_unknown_workspace() {
    let mut state = shell_state();
    state.overlay = Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
        target: ClientContextMenuTarget::Workspace {
            workspace_id: "missing".into(),
            is_git: false,
            is_linked_worktree: false,
            has_worktree_children: false,
            collapsed: false,
        },
        x: 0,
        y: 0,
        highlighted: 0,
    }));

    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(0, &mut outcome);

    assert!(
        state.overlay.is_none(),
        "menu closes even when no directory is available"
    );
}
