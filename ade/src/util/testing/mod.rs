#![allow(dead_code)]

pub(crate) mod desktop;
pub(crate) mod integration;
pub(crate) mod ipc;
pub(crate) mod launcher;
pub(crate) mod regression;
pub(crate) mod renderer;
pub(crate) mod services;
pub(crate) mod stress;
pub(crate) mod window;

pub(crate) fn run_all(desktop: &mut crate::core::desktop::Desktop) -> bool {
    let mut ok = true;
    ok &= desktop::test_window_creation(desktop);
    ok &= desktop::test_window_focus(desktop);
    ok &= desktop::test_start_menu(desktop);
    ok &= window::test_visual_flags();
    ok &= window::test_window_state();
    ok &= launcher::test_spawn(desktop);
    ok &= launcher::test_spawn_at(desktop);
    ok &= renderer::test_compositor_clear();
    ok &= renderer::test_compositor_layers();
    ok &= ipc::test_message_bus();
    ok &= ipc::test_service_registry();
    ok &= ipc::test_channels();
    ok &= ipc::test_permission_manager();
    ok &= ipc::test_register_defaults();
    ok &= ipc::test_exit_class();
    ok &= ipc::test_ipc_gate_granted(desktop);
    ok &= ipc::test_ipc_gate_denied(desktop);
    ok &= services::test_notifications(desktop);
    ok &= services::test_clipboard(desktop);
    ok &= services::test_session(desktop);
    ok &= integration::test_full_flow(desktop);
    ok
}

pub(crate) fn run_regression(desktop: &mut crate::core::desktop::Desktop) -> bool {
    regression::run_regression_suite(desktop)
}

pub(crate) fn run_stress(desktop: &mut crate::core::desktop::Desktop) -> bool {
    stress::run_stress_tests(desktop)
}
