use uuid::Uuid;

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }
            pub fn value(&self) -> Uuid {
                self.0
            }
        }
    };
}

uuid_id!(MovieId);
uuid_id!(ReviewId);
uuid_id!(UserId);
uuid_id!(ImportSessionId);
uuid_id!(ImportProfileId);
uuid_id!(WatchlistEntryId);
uuid_id!(WatchEventId);
uuid_id!(WebhookTokenId);
uuid_id!(WrapUpId);
uuid_id!(GoalId);
