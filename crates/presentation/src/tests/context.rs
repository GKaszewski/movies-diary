/// The prebuilt deps must point at the SAME port instances as the `Repositories`
/// they were built from. If build_deps is wired to a different Arc — or is called
/// before a repo is swapped — handlers in Plan B silently talk to the wrong
/// dependency, and no other test in Plan A would catch it.
///
/// `AppContext` no longer carries a `repos` field (Plan C2 deleted it), so this
/// test builds its own `Repositories` locally, keeps a clone of the Arcs it
/// cares about, and hands the original to `build_test_state_from` — the same
/// construction `test_app_state()` uses internally.
#[test]
fn prebuilt_deps_share_port_instances_with_repos() {
    use std::sync::Arc;

    let repos = crate::extractors::tests::test_repositories();
    let user = Arc::clone(&repos.user);
    let follow_graph = Arc::clone(&repos.follow_graph);

    let state = crate::extractors::tests::build_test_state_from(
        repos,
        Arc::new(crate::extractors::tests::Panic),
    );

    assert!(
        Arc::ptr_eq(&state.app_ctx.deps.users.get_local_profile.user, &user),
        "deps.users.get_local_profile.user must be the same Arc as the Repositories it was built from"
    );
    assert!(
        Arc::ptr_eq(&state.app_ctx.deps.social.query.follow_graph, &follow_graph),
        "deps.social.query.follow_graph must be the same Arc as the Repositories it was built from"
    );
    assert_eq!(
        state.app_ctx.deps.users.get_local_profile.instance, state.app_ctx.instance,
        "prebuilt deps must carry the context's InstanceIdentity"
    );
}
