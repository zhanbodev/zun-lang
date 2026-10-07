use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthConfigError {
    MissingJwtSecret,
    InvalidDefaultScheme(String),
    DefaultSchemeRequiresJwt,
    UnsupportedJwtAlgorithm(String),
    InvalidSessionCookieSameSite(String),
    SameSiteNoneRequiresSecure,
    InvalidSessionRotation(String),
}

impl fmt::Display for AuthConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingJwtSecret => write!(f, "JWT secret is required when JWT auth is enabled"),
            Self::InvalidDefaultScheme(scheme) => {
                write!(
                    f,
                    "auth default scheme must be 'session' or 'bearer', got '{scheme}'"
                )
            }
            Self::DefaultSchemeRequiresJwt => {
                write!(
                    f,
                    "auth default scheme 'bearer' requires [server.auth.jwt].enabled = true"
                )
            }
            Self::UnsupportedJwtAlgorithm(algorithm) => {
                write!(f, "unsupported JWT algorithm '{algorithm}'")
            }
            Self::InvalidSessionCookieSameSite(value) => {
                write!(
                    f,
                    "auth session cookie_same_site must be 'lax', 'strict', or 'none', got '{value}'"
                )
            }
            Self::SameSiteNoneRequiresSecure => {
                write!(
                    f,
                    "auth session cookie_same_site = 'none' requires cookie_secure = true (SameSite=None must be Secure)"
                )
            }
            Self::InvalidSessionRotation(value) => {
                write!(
                    f,
                    "auth session rotation must be 'off', 'on_login', or 'always', got '{value}'"
                )
            }
        }
    }
}

impl std::error::Error for AuthConfigError {}
