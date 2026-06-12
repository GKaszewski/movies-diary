use crate::value_objects::UserId;

#[derive(Clone, Debug)]
pub struct UserSettings {
    user_id: UserId,
    federate_goals: bool,
    federate_reviews: bool,
    federate_watchlist: bool,
}

impl UserSettings {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            federate_goals: true,
            federate_reviews: true,
            federate_watchlist: true,
        }
    }

    pub fn from_persistence(
        user_id: UserId,
        federate_goals: bool,
        federate_reviews: bool,
        federate_watchlist: bool,
    ) -> Self {
        Self {
            user_id,
            federate_goals,
            federate_reviews,
            federate_watchlist,
        }
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn federate_goals(&self) -> bool {
        self.federate_goals
    }

    pub fn set_federate_goals(&mut self, value: bool) {
        self.federate_goals = value;
    }

    pub fn federate_reviews(&self) -> bool {
        self.federate_reviews
    }

    pub fn set_federate_reviews(&mut self, value: bool) {
        self.federate_reviews = value;
    }

    pub fn federate_watchlist(&self) -> bool {
        self.federate_watchlist
    }

    pub fn set_federate_watchlist(&mut self, value: bool) {
        self.federate_watchlist = value;
    }
}
