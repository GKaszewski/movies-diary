use crate::value_objects::{Email, PasswordHash, UserId, Username};

#[derive(Clone, Debug, Default)]
pub enum UserRole {
    #[default]
    Standard,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileField {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default)]
pub struct UserProfile {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_path: Option<String>,
    pub banner_path: Option<String>,
    pub also_known_as: Option<String>,
    pub profile_fields: Vec<ProfileField>,
}

#[derive(Clone, Debug)]
pub struct User {
    id: UserId,
    email: Email,
    username: Username,
    password_hash: PasswordHash,
    role: UserRole,
    profile: UserProfile,
}

impl User {
    pub fn new(
        email: Email,
        username: Username,
        password_hash: PasswordHash,
        role: UserRole,
    ) -> Self {
        Self {
            id: UserId::generate(),
            email,
            username,
            password_hash,
            role,
            profile: UserProfile::default(),
        }
    }

    pub fn from_persistence(
        id: UserId,
        email: Email,
        username: Username,
        password_hash: PasswordHash,
        role: UserRole,
        profile: UserProfile,
    ) -> Self {
        Self {
            id,
            email,
            username,
            password_hash,
            role,
            profile,
        }
    }

    pub fn update_password(&mut self, new_hash: PasswordHash) {
        self.password_hash = new_hash;
    }

    pub fn update_profile(&mut self, profile: UserProfile) {
        self.profile = profile;
    }

    pub fn email(&self) -> &Email {
        &self.email
    }
    pub fn username(&self) -> &Username {
        &self.username
    }
    pub fn id(&self) -> &UserId {
        &self.id
    }
    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }
    pub fn role(&self) -> &UserRole {
        &self.role
    }
    pub fn display_name(&self) -> Option<&str> {
        self.profile.display_name.as_deref()
    }
    pub fn bio(&self) -> Option<&str> {
        self.profile.bio.as_deref()
    }
    pub fn avatar_path(&self) -> Option<&str> {
        self.profile.avatar_path.as_deref()
    }
    pub fn banner_path(&self) -> Option<&str> {
        self.profile.banner_path.as_deref()
    }
    pub fn also_known_as(&self) -> Option<&str> {
        self.profile.also_known_as.as_deref()
    }
    pub fn profile_fields(&self) -> &[ProfileField] {
        &self.profile.profile_fields
    }
}

#[derive(Clone, Debug)]
pub struct UserSummary {
    pub user_id: UserId,
    email: Email,
    username: Username,
    display_name: Option<String>,
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub avatar_path: Option<String>,
}

impl UserSummary {
    pub fn new(
        user_id: UserId,
        email: Email,
        username: Username,
        display_name: Option<String>,
        total_movies: i64,
        avg_rating: Option<f64>,
        avatar_path: Option<String>,
    ) -> Self {
        Self {
            user_id,
            email,
            username,
            display_name,
            total_movies,
            avg_rating,
            avatar_path,
        }
    }
    pub fn email(&self) -> &str {
        self.email.value()
    }
    pub fn username(&self) -> &str {
        self.username.value()
    }
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }
}
