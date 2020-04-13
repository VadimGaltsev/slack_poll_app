use crate::data::{
    ChannelUser, Database, DialogVariantWrite, GetPollReport, ReadLastPoll, ReadPoll,
    ReadPollVariant, UpdatePollTime, VotesResult, WriteNewPoll,
};
use crate::diesel::query_dsl::methods::DistinctOnDsl;
use crate::diesel::GroupedBy;
use crate::schema::votes_results::all_columns;
use crate::schema::{channel_users, poll, poll_variant, votes_results};
use crate::ui_poll_view::{PollReport, PollView, SingleVariant};
use actix::{Actor, Handler};
use chrono::NaiveDateTime;
use diesel::dsl::{avg, max};
use diesel::expression::dsl::count;
use diesel::pg::expression::array_comparison::any;
use diesel::sql_types::{BigInt, Double, Float, Integer, Text};
use diesel::{
    insert_into, r2d2, select, sql_query, update, BelongingToDsl, ExpressionMethods, Identifiable,
    Insertable, PgConnection, QueryDsl, Queryable, RunQueryDsl,
};
use std::str::FromStr;

#[derive(Clone, Debug, Queryable, Associations, Identifiable, PartialEq)]
#[belongs_to(PollViewSource, foreign_key = "day_id")]
#[table_name = "poll_variant"]
pub struct SingleVariantSource {
    pub id: i32,
    pub day_id: i32,
    pub title: Option<String>,
    pub variant: Option<String>,
    pub start_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
}

impl Default for SingleVariantSource {
    fn default() -> Self {
        SingleVariantSource {
            id: 0,
            day_id: 0,
            title: None,
            variant: None,
            start_date: NaiveDateTime::from_timestamp(0, 0),
            end_date: None,
        }
    }
}

#[derive(Clone, Debug, Insertable)]
#[table_name = "poll_variant"]
pub struct SingleVariantWrite {
    pub day_id: i32,
    pub title: String,
    pub variant: String,
    pub start_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Insertable)]
#[table_name = "poll"]
pub struct PollViewWrite {
    pub channel: String,
    pub is_closed: bool,
    pub time: Option<String>,
}

#[derive(Clone, Debug, Queryable, Identifiable, PartialEq)]
#[table_name = "poll"]
pub struct PollViewSource {
    pub id: i32,
    pub channel: String,
    pub is_closed: bool,
    pub time: Option<String>,
}

impl Into<PollViewWrite> for &PollView {
    fn into(self) -> PollViewWrite {
        PollViewWrite {
            channel: self.channel.clone(),
            is_closed: false,
            time: None,
        }
    }
}

#[derive(Clone, Debug, QueryableByName)]
pub struct PollReportSource {
    #[sql_type = "Text"]
    pub team: String,
    #[sql_type = "Text"]
    pub channel: String,
    #[sql_type = "Text"]
    pub total_votes: String,
    #[sql_type = "Double"]
    pub score: f64,
}

impl Into<Vec<SingleVariantWrite>> for &PollView {
    fn into(self) -> Vec<SingleVariantWrite> {
        let variants = &self.variants;
        variants
            .iter()
            .map(|element| SingleVariantWrite {
                day_id: self.id.unwrap(),
                title: element.title.clone(),
                variant: element.variant.clone(),
                start_date: element.start_date,
                end_date: None,
            })
            .collect()
    }
}

impl Into<PollView>
    for (
        PollViewSource,
        Vec<SingleVariantSource>,
        Vec<ChannelUser>,
        Vec<VotesResult>,
    )
{
    fn into(self) -> PollView {
        let users = &self.2;
        let votes = &self.3;
        PollView {
            id: Some(self.0.id),
            variants: self
                .1
                .iter()
                .map(|variant| {
                    let images = votes
                        .iter()
                        .filter(|element| element.poll_variant_id == variant.id);

                    SingleVariant {
                        id: Some(variant.id),
                        title: variant.title.clone().unwrap_or(String::new()),
                        variant: variant.variant.clone().unwrap_or(Default::default()),
                        images: images
                            .clone()
                            .flat_map(|votes| {
                                users
                                    .into_iter()
                                    .filter(move |user| user.id == votes.user_id)
                                    .map(|user| {
                                        user.user_thumbnail.clone().unwrap_or(Default::default())
                                    })
                            })
                            .collect(),
                        votes: if images.clone().count() == 0 {
                            None
                        } else {
                            Some(images.count() as i32)
                        },
                        start_date: variant.start_date,
                    }
                })
                .collect(),
            channel: self.0.channel,
            is_closed: self.0.is_closed,
            time: self.0.time,
        }
    }
}

impl Handler<ReadPollVariant> for Database {
    type Result = Result<SingleVariantSource, ()>;

    fn handle(&mut self, msg: ReadPollVariant, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll;
        use crate::schema::poll_variant::dsl::*;

        let conn = &self.0.get().unwrap();
        let poll: PollViewSource =
            diesel::QueryDsl::order(poll::table, poll::id.desc())
                .first::<PollViewSource>(conn)
                .expect("Cannot find last poll");
        diesel::QueryDsl::filter(SingleVariantSource::belonging_to(&poll), id.eq(msg.0))
            .first::<SingleVariantSource>(conn)
            .map_err(|e| println!("Cannot read variant for day {}", e))
    }
}

impl Handler<ReadLastPoll> for Database {
    type Result = Result<PollView, ()>;

    fn handle(&mut self, _: ReadLastPoll, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll::*;
        use crate::schema::poll_variant::dsl::*;

        let conn = &self.0.get().unwrap();
        let poll: PollViewSource =
            diesel::QueryDsl::order(poll::table, poll::id.desc())
                .first::<PollViewSource>(conn)
                .expect("Cannot find last poll");
        let variants = SingleVariantSource::belonging_to(&poll)
            .load::<SingleVariantSource>(conn)
            .expect("No variants for given id");
        let users = channel_users::table
            .load::<ChannelUser>(conn)
            .unwrap_or(Default::default());
        let votes = diesel::QueryDsl::distinct_on(
            VotesResult::belonging_to(&users),
            (votes_results::user_id, votes_results::poll_variant_id),
        )
        .load::<VotesResult>(conn)
        .unwrap_or(Default::default());
        println!("{:?},  --- {:?}", variants, votes);
        Ok((poll, variants, users, votes).into())
    }
}

impl Handler<ReadPoll> for Database {
    type Result = Result<PollView, ()>;

    fn handle(&mut self, _: ReadPoll, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll::*;
        use crate::schema::poll_variant::dsl::*;

        let conn = &self.0.get().unwrap();
        let poll: PollViewSource =
            diesel::QueryDsl::order(poll::table, poll::id.desc())
                .first::<PollViewSource>(conn)
                .expect("Cannot find last poll");
        let variants = SingleVariantSource::belonging_to(&poll)
            .load::<SingleVariantSource>(conn)
            .expect("No days for given id");
        let users = channel_users::table
            .load::<ChannelUser>(conn)
            .unwrap_or(Default::default());
        let votes = diesel::QueryDsl::distinct_on(
            VotesResult::belonging_to(&users),
            votes_results::user_id,
        )
        .load::<VotesResult>(conn)
        .unwrap_or(Default::default());
        Ok((poll, variants, users, votes).into())
    }
}

impl Handler<UpdatePollTime> for Database {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: UpdatePollTime, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll::*;
        let conn = &self.0.get().unwrap();
        let poll: PollViewSource =
            diesel::QueryDsl::order(poll::table, poll::id.desc())
                .first::<PollViewSource>(conn)
                .expect("Cannot find last poll");
        update(table.filter(id.eq(poll.id)))
            .set(time.eq(Some(msg.0)))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| println!("Cannot update day time {}", e))
    }
}

impl Handler<GetPollReport> for Database {
    type Result = Result<Vec<PollReportSource>, ()>;

    fn handle(&mut self, msg: GetPollReport, _: &mut Self::Context) -> Self::Result {
        let conn = &self.0.get().unwrap();
        let limit = std::env::var("MIN_VOTES_COUNT").unwrap_or(Default::default());
        let limit_num = i32::from_str(limit.as_str()).unwrap_or(0);
        let poll: PollViewSource =
            diesel::QueryDsl::order(poll::table, poll::id.desc())
                .first::<PollViewSource>(conn)
                .expect("Cannot find last poll");
        sql_query(std::env::var("SQL_COUNTER").unwrap_or(String::default()))
            .bind::<Integer, _>(poll.id)
            .bind::<Integer, _>(limit_num)
            .load::<PollReportSource>(conn)
            .map_err(|e| println!("Cannot create report cause {}", e))
    }
}

impl Handler<WriteNewPoll> for Database {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: WriteNewPoll, ctx: &mut Self::Context) -> Self::Result {
        let poll_channel = msg.0.poll_channel;
        let dialog_variants = msg.0.dialog_variants;
        let poll_variants = msg.0.poll_variants;
        let connection = &self.0.get().unwrap();
        let poll = insert_into(crate::schema::poll::table)
            .values(PollViewWrite {
                channel: poll_channel,
                is_closed: false,
                time: None,
            })
            .get_result::<PollViewSource>(connection)
            .expect("Cannot write poll");
        let dialog_variants_write = dialog_variants
            .into_iter()
            .map(|e| DialogVariantWrite {
                day_id: poll.id,
                variant_text: e.variant_text,
                max_score: e.max_score.last().unwrap(),
            })
            .collect::<Vec<_>>();
        insert_into(crate::schema::dialog_variants::table)
            .values(dialog_variants_write)
            .execute(connection)
            .expect("Cannot write dialog variants");
        let poll_write_variants = poll_variants
            .into_iter()
            .map(|e| SingleVariantWrite {
                day_id: poll.id,
                title: e.title,
                variant: e.variant,
                start_date: e.start_date,
                end_date: None,
            })
            .collect::<Vec<_>>();
        insert_into(crate::schema::poll_variant::table)
            .values(poll_write_variants)
            .execute(connection)
            .map_err(|e| println!("Cannot write poll variants {}", e))
            .map(|_| ())
    }
}
