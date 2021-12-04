use super::image::Image;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Hash)]
pub struct UserId(String);

// impl Id for UserId {
//     #[inline]
//     fn id(&self) -> &str {
//         &self.0
//     }

//     #[inline]
//     fn _type(&self) -> Type {
//         Type::User
//     }

//     #[inline]
//     fn _type_static() -> Type
//     where
//         Self: Sized,
//     {
//         Type::User
//     }

//     #[inline]
//     unsafe fn from_id_unchecked(id: &str) -> Self {
//         UserId(id.to_owned())
//     }

//     /// Parse Spotify id from string slice. Spotify doesn't specify what a User
//     /// ID might look like, so this will allow any kind of value.
//     fn from_id(id: &str) -> Result<Self, IdError>
//     where
//         Self: Sized,
//     {
//         // Safe, we've just checked that the Id is valid.
//         Ok(unsafe { Self::from_id_unchecked(id) })
//     }
// }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicUser {
    pub display_name: Option<String>,
    // pub external_urls: HashMap<String, String>,
    // pub followers: Option<Followers>,
    // pub href: String,
    pub id: UserId,
    #[serde(default = "Vec::new")]
    pub images: Vec<Image>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateUser {
    // pub country: Option<Country>,
    pub display_name: Option<String>,
    // pub email: Option<String>,
    // pub external_urls: HashMap<String, String>,
    // pub explicit_content: Option<ExplicitContent>,
    // pub followers: Option<Followers>,
    // pub href: String,
    pub id: UserId,
    pub images: Option<Vec<Image>>,
    // pub product: Option<SubscriptionLevel>,
}
