use crate::data::DialogVariant;
use actix::Message;
use chrono::{Date, NaiveDateTime, TimeZone};
use std::ops::{Range, RangeInclusive};

#[derive(Clone, Debug)]
pub struct PollView {
    pub id: Option<i32>,
    pub variants: Vec<SingleVariant>,
    pub channel: String,
    pub is_closed: bool,
    pub time: Option<String>,
}

impl Message for PollView {
    type Result = Result<String, String>;
}

#[derive(Clone, Debug)]
pub struct SingleVariant {
    pub id: Option<i32>,
    pub title: String,
    pub variant: String,
    pub images: Vec<String>,
    pub votes: Option<i32>,
    pub start_date: NaiveDateTime,
}

#[derive(Clone, Debug)]
pub struct PollReport {
    pub results: Vec<PollResult>,
}

#[derive(Clone, Debug)]
pub struct PollResult {
    pub place: String,
    pub variant_title: String,
    pub votes_count: i32,
    pub points_count: f32,
}

#[derive(Clone, Debug)]
pub struct DialogView {
    pub title: String,
    pub variants: Vec<DialogViewVariant>,
}

#[derive(Clone, Debug)]
pub struct DialogViewVariant {
    pub variant_text: String,
    pub max_score: RangeInclusive<i32>,
}

impl Into<DialogViewVariant> for DialogVariant {
    fn into(mut self) -> DialogViewVariant {
        if self.max_score > 100 {
            self.max_score = 100;
        }
        DialogViewVariant {
            variant_text: self.variant_text,
            max_score: 1..=self.max_score,
        }
    }
}

impl Into<DialogView> for Vec<DialogVariant> {
    fn into(self) -> DialogView {
        DialogView {
            title: String::new(),
            variants: self.into_iter().map(Into::into).collect(),
        }
    }
}

impl DialogView {
    pub fn set_title(mut self, title: String) -> Self {
        self.title = title;
        self
    }
}

impl SingleVariant {
    pub fn new(title: &str, variant: &str, images: Vec<&str>) -> SingleVariant {
        SingleVariant {
            id: None,
            title: title.to_owned(),
            variant: variant.to_owned(),
            images: images.into_iter().map(|e| e.to_owned()).collect(),
            votes: None,
            start_date: NaiveDateTime::from_timestamp(1, 1),
        }
    }

    pub fn add_image(mut self, image: &str) -> SingleVariant {
        self.images.push(image.to_owned());
        self
    }
}

impl PollView {
    pub fn new(id: i32, variants: Vec<SingleVariant>, channel: &str) -> PollView {
        PollView {
            id: Some(id),
            variants,
            channel: channel.to_owned(),
            is_closed: false,
            time: None,
        }
    }
}
