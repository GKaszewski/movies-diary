use domain::models::UserRole;

pub struct RegisterCommand {
    pub email: String,
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

pub struct RegisterAndLoginCommand {
    pub email: String,
    pub username: String,
    pub password: String,
}
