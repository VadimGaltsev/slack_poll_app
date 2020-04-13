use crate::data::{
    ChannelUser, DialogVariant, PollReportSource, Pool, SingleVariantSource, VotesResult,
};
use crate::poll_state::PollData;
use crate::ui_poll_view::{PollReport, PollView, SingleVariant};
use actix::{Actor, Addr, Message, SyncArbiter, SyncContext};
use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};
use dotenv::dotenv;
use std::env;

pub struct Database(pub(crate) Pool);

impl Actor for Database {
    type Context = SyncContext<Self>;
}

pub struct ReadPoll(pub i32);

pub struct ReadLastPoll;

pub struct FindUser(pub String);

pub struct WriteUser(pub String, pub String);

pub struct WriteVotes(pub i32, pub i32, pub i32, pub i32, pub i32);

pub struct ReadVotesForCurrentDay;

pub struct ReadPollVariant(pub i32);

pub struct ReadVotesForCurrentUser(pub String);

pub struct ReadDialogVariantsForLastDay;

pub struct WriteNewPoll(pub PollData);

pub struct UpdatePollTime(pub String);

pub struct GetPollReport;

impl Message for WriteNewPoll {
    type Result = Result<(), ()>;
}

impl Message for UpdatePollTime {
    type Result = Result<(), ()>;
}

impl Message for FindUser {
    type Result = Result<ChannelUser, ()>;
}

impl Message for WriteUser {
    type Result = Result<ChannelUser, ()>;
}

impl Message for ReadPoll {
    type Result = Result<PollView, ()>;
}

impl Message for ReadLastPoll {
    type Result = Result<PollView, ()>;
}

impl Message for ReadPollVariant {
    type Result = Result<SingleVariantSource, ()>;
}

impl Message for WriteVotes {
    type Result = Result<(), ()>;
}

impl Message for ReadVotesForCurrentDay {
    type Result = Result<Vec<Vec<VotesResult>>, ()>;
}

impl Message for ReadVotesForCurrentUser {
    type Result = Result<Vec<VotesResult>, ()>;
}

impl Message for ReadDialogVariantsForLastDay {
    type Result = Result<Vec<DialogVariant>, ()>;
}

impl Message for GetPollReport {
    type Result = Result<Vec<PollReportSource>, ()>;
}

pub fn create_connection() -> Addr<Database> {
    dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL env var doesnt exist and must be set");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    SyncArbiter::start(4, move || Database(pool.clone()))
}
