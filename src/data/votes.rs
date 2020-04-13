use crate::data::ChannelUser;
use crate::data::{
    Database, PollViewSource, ReadVotesForCurrentDay, ReadVotesForCurrentUser, SingleVariantSource,
    WriteVotes,
};
use crate::diesel::GroupedBy;
use crate::schema::{channel_users, poll, poll_variant, votes_results};
use crate::ui_poll_view::SingleVariant;
use actix::Handler;
use diesel::query_dsl::filter_dsl::FilterDsl;
use diesel::query_dsl::methods::OrderDsl;
use diesel::{
    insert_into, r2d2, update, BelongingToDsl, ExpressionMethods, Identifiable, Insertable,
    PgConnection, QueryDsl, Queryable, RunQueryDsl,
};

#[derive(Clone, Debug, Queryable, Associations, Identifiable, PartialEq)]
#[belongs_to(SingleVariantSource, foreign_key = "poll_variant_id")]
#[belongs_to(ChannelUser, foreign_key = "user_id")]
#[belongs_to(PollViewSource, foreign_key = "day_id")]
pub struct VotesResult {
    pub id: i32,
    pub user_id: i32,
    pub day_id: i32,
    pub poll_variant_id: i32,
    pub dialog_variant_id: i32,
    pub score: i32,
}

#[derive(Clone, Debug, Insertable, PartialEq)]
#[table_name = "votes_results"]
pub struct VotesResultWrite {
    pub user_id: i32,
    pub day_id: i32,
    pub poll_variant_id: i32,
    pub dialog_variant_id: i32,
    pub score: i32,
}

impl Into<VotesResultWrite> for WriteVotes {
    fn into(self) -> VotesResultWrite {
        VotesResultWrite {
            user_id: self.0,
            day_id: self.1,
            poll_variant_id: self.2,
            dialog_variant_id: self.3,
            score: self.4,
        }
    }
}

impl Handler<WriteVotes> for Database {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: WriteVotes, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll_variant::dsl::*;
        use crate::schema::votes_results::dsl::*;
        let connection = self.0.get().unwrap();
        diesel::insert_into(votes_results)
            .values::<VotesResultWrite>(msg.into())
            .execute(&connection)
            .map_err(|_| println!("Cannot write result"))
            .map(|_| ())
    }
}

impl Handler<ReadVotesForCurrentUser> for Database {
    type Result = Result<Vec<VotesResult>, ()>;

    fn handle(&mut self, msg: ReadVotesForCurrentUser, _: &mut Self::Context) -> Self::Result {
        use crate::schema;
        use crate::schema::votes_results::dsl::*;

        let connection = self.0.get().unwrap();
        let current_day: PollViewSource =
            diesel::QueryDsl::order(schema::poll::table, schema::poll::id.desc())
                .first::<PollViewSource>(&connection)
                .expect("Cannot find last poll");
        let user = diesel::QueryDsl::filter(
            schema::channel_users::dsl::channel_users,
            schema::channel_users::user_slack_id.eq(msg.0),
        )
        .first::<ChannelUser>(&connection)
        .unwrap_or(Default::default());

        diesel::QueryDsl::filter(VotesResult::belonging_to(&current_day), user_id.eq(user.id))
            .load::<VotesResult>(&connection)
            .map_err(|e| println!("Cannot read votes from table {}", e))
    }
}

impl Handler<ReadVotesForCurrentDay> for Database {
    type Result = Result<Vec<Vec<VotesResult>>, ()>;

    fn handle(&mut self, _: ReadVotesForCurrentDay, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll::dsl::*;
        use crate::schema::poll_variant::dsl::*;
        use crate::schema::votes_results::dsl::*;
        let connection = self.0.get().unwrap();
        let current_day: PollViewSource = poll
            .load(&connection)
            .unwrap()
            .into_iter()
            .last()
            .unwrap();
        let variants: Vec<SingleVariantSource> = SingleVariantSource::belonging_to(&current_day)
            .load::<SingleVariantSource>(&connection)
            .expect("Error while loading variants to poll");
        let votes: Vec<Vec<VotesResult>> = VotesResult::belonging_to(&variants)
            .load::<VotesResult>(&connection)
            .expect("Error while loading votes to poll")
            .grouped_by(&variants);
        Ok(votes)
    }
}
