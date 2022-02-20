use mongodb::{bson::oid::ObjectId, Collection};
use rocket::{
    http::Status,
    request::FromParam,
    serde::{Deserialize, Serialize},
};

pub const USERS_COLLECTION: &'static str = "users";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub name: String,
}

pub struct UserConn(pub Collection<User>);

pub const COOKIE_USER_ID: &'static str = "user_id";
pub const COOKIE_USER_NAME: &'static str = "name";

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIdParam(pub ObjectId);

impl FromParam<'_> for UserIdParam {
    type Error = Status;

    fn from_param(param: &'_ str) -> Result<Self, Self::Error> {
        match ObjectId::parse_str(param) {
            Ok(id) => Ok(Self(id)),
            Err(error) => {
                error!("Error while parsing user id: {error:?}");
                Err(Status::BadRequest)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateUserIdRequest {
    pub name: String,
}
