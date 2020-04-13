mod dialogs;
mod local_datasource;
mod polls;
mod users;
mod votes;

use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};

pub use {dialogs::*, local_datasource::*, polls::*, users::*, votes::*};

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
