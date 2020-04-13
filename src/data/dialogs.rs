use crate::data::{Database, PollViewSource, ReadDialogVariantsForLastDay};
use crate::schema::poll::dsl::poll;
use crate::schema::dialog_variants;
use actix::{Actor, Handler};
use diesel::{
    insert_into, r2d2, update, BelongingToDsl, ExpressionMethods, Identifiable, Insertable,
    PgConnection, QueryDsl, Queryable, RunQueryDsl,
};

#[derive(Clone, Debug, Queryable, Associations, Identifiable, PartialEq)]
#[belongs_to(PollViewSource, foreign_key = "day_id")]
#[table_name = "dialog_variants"]
pub struct DialogVariant {
    pub id: i32,
    pub day_id: i32,
    pub variant_text: String,
    pub max_score: i32,
}

#[derive(Clone, Debug, Insertable, PartialEq)]
#[table_name = "dialog_variants"]
pub struct DialogVariantWrite {
    pub day_id: i32,
    pub variant_text: String,
    pub max_score: i32,
}

impl Handler<ReadDialogVariantsForLastDay> for Database {
    type Result = Result<Vec<DialogVariant>, ()>;

    fn handle(&mut self, _: ReadDialogVariantsForLastDay, _: &mut Self::Context) -> Self::Result {
        use crate::schema::poll;
        use crate::schema::dialog_variants::dsl::*;
        let connection = self.0.get().unwrap();
        let current_day = poll::table
            .order(poll::id.desc())
            .first::<PollViewSource>(&connection)
            .expect("Cannot load poll");
        DialogVariant::belonging_to(&current_day)
            .load::<DialogVariant>(&connection)
            .map_err(|e| println!("Cannot find dialog variants {}", e))
    }
}
