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
    /// Derives a stable, deterministic UUID from an external metadata ID (e.g. `tmdb:12345`).
    /// All instances that know a movie by the same external ID will produce the same MovieId,
    /// enabling remote and local reviews to be linked to the same movie record.
    pub fn from_external(external_id: &crate::value_objects::ExternalMetadataId) -> Self {
        Self(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            external_id.value().as_bytes(),
        ))
    }
}

uuid_id!(ReviewId);
uuid_id!(UserId);
uuid_id!(ImportSessionId);
uuid_id!(ImportProfileId);
uuid_id!(WatchlistEntryId);
uuid_id!(WatchEventId);
uuid_id!(WebhookTokenId);
uuid_id!(WrapUpId);
uuid_id!(GoalId);
uuid_id!(PersonId);

impl PersonId {
    /// Deterministic UUIDv5 from an external person ID string.
    /// "tmdb:12345" always maps to the same PersonId.
    pub fn from_external(external_id: &crate::models::person::ExternalPersonId) -> Self {
        Self(Uuid::new_v5(&Uuid::NAMESPACE_URL, external_id.value().as_bytes()))
    }
}
