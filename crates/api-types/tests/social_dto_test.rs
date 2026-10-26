use api_types::social::FollowStateDto;

#[test]
fn follow_state_dto_serializes_to_lowercase_strings() {
    assert_eq!(
        serde_json::to_string(&FollowStateDto::None).unwrap(),
        "\"none\""
    );
    assert_eq!(
        serde_json::to_string(&FollowStateDto::Pending).unwrap(),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&FollowStateDto::Accepted).unwrap(),
        "\"accepted\""
    );
    assert_eq!(
        serde_json::to_string(&FollowStateDto::Rejected).unwrap(),
        "\"rejected\""
    );
}
