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
