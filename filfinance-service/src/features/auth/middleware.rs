//! Facade des middlewares HTTP auth et synchronisation profil.

mod authorization;
mod bearer;
mod profile;

pub use authorization::{require_admin, require_listener, require_moderator};
#[cfg(test)]
pub(in crate::features::auth) use bearer::bearer_token_from_headers;
pub use profile::{authenticate_access_token, sync_current_user_profile};
