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

impl MovieId {
    pub fn from_external(external_id: &crate::value_objects::ExternalMetadataId) -> Self {
        Self(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            external_id.value().as_bytes(),
        ))
    }
}

uuid_id!(ReviewId);
uuid_id!(UserId);

pub const SERVICE_ACTOR_UUID: Uuid =
    Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]);

impl UserId {
    pub const fn service_actor() -> Self {
        Self(SERVICE_ACTOR_UUID)
    }
    pub fn is_service_actor(&self) -> bool {
        self.0 == SERVICE_ACTOR_UUID
    }
}
uuid_id!(ImportSessionId);
uuid_id!(ImportProfileId);
uuid_id!(WatchlistEntryId);
uuid_id!(WatchEventId);
uuid_id!(WebhookTokenId);
uuid_id!(WrapUpId);
uuid_id!(GoalId);
uuid_id!(PersonId);

impl PersonId {
    pub fn from_external(external_id: &crate::models::person::ExternalPersonId) -> Self {
        Self(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            external_id.value().as_bytes(),
        ))
    }
}
