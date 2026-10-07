use super::gc::LazyGcWindow;

#[test]
fn lazy_gc_window_only_runs_when_due() {
    let mut window = LazyGcWindow::new(60);
    #[allow(clippy::disallowed_methods)]
    {
        window.set_next_gc_at_for_test(100);
    }

    assert!(!window.is_due(99));
    assert!(window.is_due(100));

    window.mark_ran(100);

    assert!(!window.is_due(159));
    assert!(window.is_due(160));
}
