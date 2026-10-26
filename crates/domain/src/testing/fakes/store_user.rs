use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{FederationFlags, ProfileField, User, UserSettings, UserSummary},
    ports::{
        UserFederationSettingsQuery, UserProfileFieldsRepository, UserRepository,
        UserSettingsRepository,
    },
    value_objects::{Email, SocialIdentity, UserId, Username},
};

use super::InMemoryStore;

#[async_trait]
impl UserRepository for InMemoryStore {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let store = self.users.read().unwrap();
        let gone = self.tombstoned.read().unwrap();
        Ok(store
            .values()
            .filter(|u| !gone.contains(&u.id().value()))
            .find(|u| u.email().value() == email.value())
            .cloned())
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let store = self.users.read().unwrap();
        let gone = self.tombstoned.read().unwrap();
        Ok(store
            .values()
            .filter(|u| !gone.contains(&u.id().value()))
            .find(|u| u.username().value() == username.value())
            .cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        let mut users = self.users.write().unwrap();
        if users
            .values()
            .any(|u| u.email().value() == user.email().value())
        {
            return Err(DomainError::ValidationError(
                "Email already registered".into(),
            ));
        }
        if users
            .values()
            .any(|u| u.username().value() == user.username().value())
        {
            return Err(DomainError::ValidationError(
                "Username already taken".into(),
            ));
        }
        users.insert(user.id().value(), user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        if self.tombstoned.read().unwrap().contains(&id.value()) {
            return Ok(None);
        }
        Ok(self.users.read().unwrap().get(&id.value()).cloned())
    }

    async fn find_tombstoned_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        if !self.tombstoned.read().unwrap().contains(&id.value()) {
            return Ok(None);
        }
        Ok(self.users.read().unwrap().get(&id.value()).cloned())
    }

    async fn mark_deleted(
        &self,
        user_id: &UserId,
        events: &[crate::events::DomainEvent],
    ) -> Result<(), DomainError> {
        let id = user_id.value();
        let users = self.users.read().unwrap();
        let mut gone = self.tombstoned.write().unwrap();
        let Some(user) = users.get(&id).filter(|_| !gone.contains(&id)) else {
            return Err(DomainError::NotFound("User not found".into()));
        };
        if matches!(user.role(), crate::models::UserRole::Admin)
            && !users.values().any(|u| {
                matches!(u.role(), crate::models::UserRole::Admin)
                    && u.id().value() != id
                    && !gone.contains(&u.id().value())
            })
        {
            return Err(DomainError::Conflict(
                "the last active admin cannot be deleted".into(),
            ));
        }
        self.retired_usernames
            .write()
            .unwrap()
            .insert(user.username().value().to_lowercase());
        gone.insert(id);
        drop(gone);
        drop(users);
        self.refresh_sessions
            .write()
            .unwrap()
            .retain(|s| s.user_id != *user_id);
        self.record_events(events);
        Ok(())
    }

    async fn is_username_retired(&self, username: &Username) -> Result<bool, DomainError> {
        Ok(self
            .retired_usernames
            .read()
            .unwrap()
            .contains(&username.value().to_lowercase()))
    }

    async fn count_active_admins(&self) -> Result<u64, DomainError> {
        let gone = self.tombstoned.read().unwrap();
        Ok(self
            .users
            .read()
            .unwrap()
            .values()
            .filter(|u| matches!(u.role(), crate::models::UserRole::Admin))
            .filter(|u| !gone.contains(&u.id().value()))
            .count() as u64)
    }

    async fn list_with_stats(&self) -> Result<Vec<UserSummary>, DomainError> {
        let store = self.users.read().unwrap();
        let gone = self.tombstoned.read().unwrap();
        let mut users: Vec<_> = store
            .values()
            .filter(|u| !gone.contains(&u.id().value()))
            .collect();
        users.sort_by(|a, b| a.email().value().cmp(b.email().value()));
        Ok(users
            .into_iter()
            .map(|u| {
                UserSummary::new(
                    u.id().clone(),
                    u.email().clone(),
                    u.username().clone(),
                    u.display_name().map(str::to_string),
                    0,
                    None,
                    u.avatar_path().map(str::to_string),
                )
            })
            .collect())
    }

    async fn update_profile(
        &self,
        user_id: &UserId,
        profile: &crate::models::UserProfile,
        events: &[crate::events::DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.users.write().unwrap();
        if let Some(user) = store.get_mut(&user_id.value()) {
            user.update_profile(profile.clone());
        }
        drop(store);
        self.record_events(events);
        Ok(())
    }
}

#[async_trait]
impl UserProfileFieldsRepository for InMemoryStore {
    async fn get_fields(&self, user_id: &UserId) -> Result<Vec<ProfileField>, DomainError> {
        let store = self.profile_fields.read().unwrap();
        Ok(store.get(&user_id.value()).cloned().unwrap_or_default())
    }

    async fn set_fields(
        &self,
        user_id: &UserId,
        fields: Vec<ProfileField>,
        events: &[crate::events::DomainEvent],
    ) -> Result<(), DomainError> {
        self.profile_fields
            .write()
            .unwrap()
            .insert(user_id.value(), fields);
        self.record_events(events);
        Ok(())
    }
}

#[async_trait]
impl UserSettingsRepository for InMemoryStore {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError> {
        let store = self.user_settings.read().unwrap();
        Ok(store
            .get(&user_id.value())
            .cloned()
            .unwrap_or_else(|| UserSettings::new(user_id.clone())))
    }

    async fn save(&self, settings: &UserSettings) -> Result<(), DomainError> {
        self.user_settings
            .write()
            .unwrap()
            .insert(settings.user_id().value(), settings.clone());
        Ok(())
    }
}

#[async_trait]
impl UserFederationSettingsQuery for InMemoryStore {
    async fn get_federation_flags(&self, user_id: &UserId) -> Result<FederationFlags, DomainError> {
        let store = self.user_settings.read().unwrap();
        let settings = store
            .get(&user_id.value())
            .cloned()
            .unwrap_or_else(|| UserSettings::new(user_id.clone()));
        Ok(FederationFlags {
            goals: settings.federate_goals(),
            reviews: settings.federate_reviews(),
            watchlist: settings.federate_watchlist(),
        })
    }
}

impl InMemoryStore {
    pub fn delete_user(&self, user_id: &UserId) {
        let id = user_id.value();
        self.users.write().unwrap().remove(&id);
        self.tombstoned.write().unwrap().remove(&id);
        self.watchlist
            .write()
            .unwrap()
            .retain(|(user, _), _| *user != id);
        self.goals
            .write()
            .unwrap()
            .retain(|_, g| g.user_id().value() != id);
        self.user_settings.write().unwrap().remove(&id);
        self.profile_fields.write().unwrap().remove(&id);
        self.webhook_tokens
            .write()
            .unwrap()
            .retain(|t| t.user_id().value() != id);
        self.watch_events
            .write()
            .unwrap()
            .retain(|e| e.user_id().value() != id);
        let me = SocialIdentity::Local(user_id.clone());
        self.follows
            .write()
            .unwrap()
            .retain(|(follower, target, _)| *follower != id && *target != me);
        self.blocked
            .write()
            .unwrap()
            .retain(|(blocker, _)| *blocker != id);
    }
}
