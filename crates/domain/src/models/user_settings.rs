use crate::value_objects::UserId;

#[derive(Clone, Debug)]
pub struct UserSettings {
    user_id: UserId,
    federate_goals: bool,
}

impl UserSettings {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            federate_goals: false,
        }
    }

    pub fn from_persistence(user_id: UserId, federate_goals: bool) -> Self {
        Self {
            user_id,
            federate_goals,
        }
    }

    pub fn set_federate_goals(&mut self, value: bool) {
        self.federate_goals = value;
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn federate_goals(&self) -> bool {
        self.federate_goals
    }
}
