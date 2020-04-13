use crate::data::{Database, FindUser, WriteUser};
use crate::schema::{channel_users, poll, poll_variant, votes_results};
use actix::Handler;
use diesel::query_dsl::filter_dsl::FilterDsl;
use diesel::{
    insert_into, r2d2, update, BelongingToDsl, ExpressionMethods, Identifiable, Insertable,
    PgConnection, Queryable, RunQueryDsl,
};
use slacker::UserInfoResponse;

#[derive(Clone, Debug, Queryable, PartialEq, Identifiable)]
#[table_name = "channel_users"]
pub struct ChannelUser {
    pub id: i32,
    pub user_slack_id: Option<String>,
    pub user_thumbnail: Option<String>,
}

impl Default for ChannelUser {
    fn default() -> Self {
        ChannelUser {
            id: -1,
            user_slack_id: None,
            user_thumbnail: None,
        }
    }
}

#[derive(Clone, Debug, Insertable, PartialEq, AsChangeset)]
#[table_name = "channel_users"]
pub struct ChannelUserWrite {
    pub user_slack_id: String,
    pub user_thumbnail: Option<String>,
}

impl Handler<FindUser> for Database {
    type Result = Result<ChannelUser, ()>;

    fn handle(&mut self, msg: FindUser, _: &mut Self::Context) -> Self::Result {
        use crate::schema::channel_users::dsl::*;
        let connection = self.0.get().unwrap();
        channel_users
            .filter(user_slack_id.eq(msg.0))
            .first::<ChannelUser>(&connection)
            .map_err(|_| ())
    }
}

impl Into<ChannelUserWrite> for WriteUser {
    fn into(self) -> ChannelUserWrite {
        ChannelUserWrite {
            user_slack_id: self.0,
            user_thumbnail: if self.1.is_empty() {
                None
            } else {
                Some(self.1)
            },
        }
    }
}

impl Into<ChannelUser> for UserInfoResponse {
    fn into(self) -> ChannelUser {
        let user_info = self.user.unwrap();
        ChannelUser {
            id: 0,
            user_slack_id: Some((&user_info).id.clone()),
            user_thumbnail: Some(user_info.profile.image_24),
        }
    }
}

impl Handler<WriteUser> for Database {
    type Result = Result<ChannelUser, ()>;

    fn handle(&mut self, msg: WriteUser, _: &mut Self::Context) -> Self::Result {
        use crate::schema::channel_users::dsl::*;
        let connection = self.0.get().unwrap();
        let user_image = &msg.1.clone();
        insert_into(channel_users)
            .values::<ChannelUserWrite>(msg.into())
            .on_conflict(user_slack_id)
            .do_update()
            .set(user_thumbnail.eq(user_image))
            .get_result::<ChannelUser>(&connection)
            .map_err(|e| println!("Cannot write user {}", e))
    }
}
