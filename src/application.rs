use crate::actions_response::{ActionResponse, BlockAction};
use crate::data::{
    create_connection, Database, FindUser, GetPollReport, ReadDialogVariantsForLastDay,
    ReadLastPoll, ReadPollVariant, ReadVotesForCurrentUser, SingleVariantSource, UpdatePollTime,
    WriteNewPoll, WriteUser, WriteVotes,
};
use crate::poll_state::PollData;
use crate::slack_ui::{
    create_poll_menu, create_poll_report_view, create_poll_view, show_answered_request_view,
    show_not_ready_request_view, update_message_response,
};
use crate::ui_poll_view::{DialogView, DialogViewVariant, SingleVariant};
use crate::DIALOG_VARIANT_CREATE_ID;
use actix::Addr;
use actix_web::web::Form;
use actix_web::{Error, HttpRequest, HttpResponse, Responder};
use chrono::{Local, NaiveDateTime};
use futures::Future;
use serde_json::{Map, Value};
use slacker::{
    BlockElement, Dialog, DialogElement, DialogOpen, DialogOptionGroup, GetUserInfo, LayoutBlock,
    MessageVisibility, PostMessage, PostMessageResponse, Slacker, UserInfoResponse, View, ViewOpen,
    ViewPush, ViewUpdate,
};
use std::collections::HashMap;
use std::intrinsics::transmute;
use std::mem::swap;
use std::str::FromStr;
use std::sync::atomic::AtomicPtr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

#[derive(Clone)]
pub struct SlackApplication {
    data: Addr<Database>,
    state: Arc<Mutex<Option<PollData>>>,
    slacker: Slacker,
    pub user_admin: String,
}

impl SlackApplication {
    pub fn new() -> Self {
        SlackApplication {
            data: create_connection(),
            state: Arc::new(Mutex::new(Option::Some(PollData::default()))),
            slacker: Slacker::new(
                std::env::var("API_KEY").unwrap().as_str(),
                std::env::var("WORK_SPACE").unwrap().as_str(),
            ),
            user_admin: std::env::var("USER_ADMIN").unwrap_or_default(),
        }
    }

    pub fn process_dialog_submission(&self, block_action: BlockAction) {
        let slacker = self.clone().slacker;
        let slacker_client = self.clone().slacker;
        let data = self.clone().data;
        let old_view = self.clone().state;
        let callback_id = i32::from_str(&block_action.callback_id.clone()).unwrap();
        let user_id = block_action.user.id.clone();
        let dialog_submission = data
            .send(FindUser(block_action.user.id.clone()))
            .map_err(|e| println!("Cannot find user {}", e))
            .and_then(|result| {
                println!("Get from data base");
                result
            })
            .or_else(move |_| {
                println!("Get from slack api user");
                slacker_client
                    .get(GetUserInfo(user_id))
                    .map_err(|e| println!("Cannot load user info {}", e))
                    .map(|user| user.into())
            })
            .and_then(move |result| {
                let user_info = result;
                let user_image = user_info.user_thumbnail.unwrap();
                let user_id = user_info.user_slack_id.unwrap();
                let database = data.clone();
                data.send(WriteUser(user_id.clone(), user_image.clone()))
                    .map_err(|e| println!("Cannot write user {}", e))
                    .and_then(move |user| {
                        let user_id = user.unwrap().id;
                        let answers = block_action.submission;
                        data.send(ReadDialogVariantsForLastDay)
                            .map(|variants| variants.unwrap())
                            .map_err(|_| ())
                            .and_then(move |variants| {
                                let mut futures = vec![];
                                for variant in variants {
                                    let write_vote = data
                                        .send(WriteVotes(
                                            user_id,
                                            variant.day_id,
                                            callback_id,
                                            variant.id,
                                            i32::from_str(&answers[&variant.variant_text]).unwrap(),
                                        ))
                                        .map_err(|_| ());
                                    futures.push(write_vote)
                                }
                                futures::future::join_all(futures)
                            })
                            .map(|_| println!("Result written"))
                            .and_then(move |_| {
                                database
                                    .send(ReadLastPoll)
                                    .map_err(|_| println!("Cannot read poll"))
                            })
                            .and_then(move |result| {
                                //todo change to data base poll time
                                let result = result.unwrap();
                                update_message_response(
                                    slacker,
                                    result.time.clone().unwrap(),
                                    result,
                                )
                            })
                    })
            });
        actix::spawn(dialog_submission);
    }

    pub fn process_poll_request(&self, trigger_id: String) {
        let app_data = self.clone();
        let task = app_data
            .slacker
            .post(create_poll_menu(trigger_id))
            .map_err(|e| println!("Error while request poll {}", e))
            .map(move |poll| {
                println!("View response {:?}", poll);
            });
        actix::spawn(task);
    }

    pub fn close_poll_and_create_report_request(&self, trigger_id: String) {
        println!("Run report");
        let app_data = self.clone();
        let slacker = self.slacker.clone();
        let task = self
            .data
            .send(GetPollReport)
            .map_err(|e| println!("Cannot read report {:?}", e))
            .map(|e| create_poll_report_view(e.unwrap()))
            .and_then(move |e| {
                slacker
                    .post(e)
                    .map_err(|e| println!("Cannot post report {}", e))
            })
            .map(|result| println!("{:?}", result));
        actix::spawn(task);
    }

    pub fn post_last_poll_to_channel(&self, trigger_id: String) {
        let state = self.state.clone();
        let database = self.data.clone();
        let slacker = self.slacker.clone();
        let post_poll = self
            .data
            .send(ReadLastPoll)
            .map_err(|e| println!("Cannot read last poll"))
            .and_then(move |poll| {
                slacker
                    .post(create_poll_view(poll.unwrap()))
                    .map_err(|e| println!("Error while request poll {}", e))
            })
            .and_then(move |resp| {
                println!("View response {:?}", resp);
                let mut post = state.lock().unwrap();
                database
                    .send(UpdatePollTime(resp.ts))
                    .map_err(|e| println!("Cannot write poll {}", e))
            })
            .map(|_| ());
        actix::spawn(post_poll);
    }

    pub fn post_dialog_on_request(&self, block_action: BlockAction) {
        let client = self.slacker.clone();
        println!("{:?}", block_action.actions);
        let action_id = block_action.actions.first().unwrap().action_id.clone();
        let id = action_id.clone();
        let dialog_with_poll =
            self.create_dialog_for_poll(action_id.clone(), block_action.trigger_id.clone());

        let answer = self
            .data
            .send(ReadVotesForCurrentUser(block_action.user.id.clone()))
            .map_err(|e| println!("Cannot read votes for current user {}", e))
            .map(move |votes| {
                votes
                    .expect("Cannot find votes in database")
                    .iter()
                    .find(|e| e.poll_variant_id == i32::from_str(id.as_str()).unwrap())
                    .is_none()
            })
            .join(
                self.data
                    .send(ReadPollVariant(
                        i32::from_str(action_id.clone().as_str()).unwrap_or(1),
                    ))
                    .map_err(|e| println!("Cannot find this variant {}", e)),
            )
            .and_then(move |is_available| {
                println!("Start choose {:?}", is_available);
                let start_time = is_available.1.unwrap();
                println!("Start choose {:?}", start_time);
                let now = Local::now().naive_local();
                println!("Start choose {:?}", now);
                if is_available.0 && start_time.start_date < now {
                    println!("Ok");
                    dialog_with_poll
                } else if !is_available.0 {
                    println!("Voted");
                    show_answered_request_view(client, block_action)
                } else {
                    println!("Time");
                    show_not_ready_request_view(client, block_action, start_time.start_date)
                }
            });
        actix::spawn(answer);
    }

    fn create_dialog_for_poll(
        &self,
        action_id: String,
        trigger_id: String,
    ) -> Box<dyn Future<Item = (), Error = ()>> {
        let client = self.slacker.clone();
        Box::new(
            self.data
                .send(ReadDialogVariantsForLastDay)
                .map(|variants| Into::<DialogView>::into(variants.unwrap()))
                .map_err(|e| println!("Cannot read from database dialog variants {}", e))
                .join(
                    self.data
                        .send(ReadPollVariant(i32::from_str(action_id.as_str()).unwrap()))
                        .map_err(|e| println!("Cannot read from database day variants {}", e)),
                )
                .and_then(move |result| {
                    let view = result.0;
                    let variant = result.1.unwrap();
                    let mut dialog;
                    if variant.title.as_ref().unwrap_or(&Default::default()).len() >= 24 {
                        dialog = Dialog::new_dialog_with_callback(
                            format!(
                                "{}...",
                                variant
                                    .title
                                    .clone()
                                    .unwrap_or(Default::default())
                                    .drain(0..=20)
                                    .collect::<String>()
                            )
                            .as_str(),
                            &action_id,
                            "Подтвердить",
                        );
                    } else {
                        dialog = Dialog::new_dialog_with_callback(
                            format!("{}", variant.title.clone().unwrap_or(Default::default()))
                                .as_str(),
                            &action_id,
                            "Подтвердить",
                        );
                    }
                    for variant in view.variants {
                        let mut options = vec![];
                        for i in variant.max_score {
                            options.push(i.to_string())
                        }
                        dialog =
                            dialog.add_element(DialogElement::new_select_element_with_options(
                                &variant.variant_text,
                                &variant.variant_text,
                                options,
                            ));
                    }
                    client
                        .post(DialogOpen::new(&trigger_id, dialog))
                        .map_err(|e| println!("Cannot post request to dialog {}", e))
                })
                .map(|e| println!("{:?}", e)),
        )
    }

    pub fn add_variant_to_poll(&self, mut old_view: View) {
        let count = old_view
            .blocks
            .iter()
            .filter(|e| {
                if let LayoutBlock::Input { .. } = e {
                    true
                } else {
                    false
                }
            })
            .count();
        let next_id = count / 3 + 1;
        old_view.blocks.insert(
            old_view.blocks.len() - 2,
            LayoutBlock::new_plain_single_line_text_input(
                format!("Заголовок #{}", next_id).as_str(),
                format!("title_text_{}", next_id),
                "Можно в markdown",
            ),
        );
        old_view.blocks.insert(
            old_view.blocks.len() - 2,
            LayoutBlock::new_plain_text_input(
                format!("Вариант #{}", next_id).as_str(),
                format!("variant_text_{}", next_id),
            ),
        );
        old_view.blocks.insert(
            old_view.blocks.len() - 2,
            LayoutBlock::new_plain_single_line_text_input(
                format!("Дата начала голосования #{}", next_id).as_str(),
                format!("start_variant_poll_date_{}", next_id),
                "2015-09-18T23:56:04",
            ),
        );
        let mut id = old_view.id.clone();
        let mut submit = old_view.submit.clone();
        let mut update_view = ViewUpdate::new(old_view);
        update_view.view_id = id;
        update_view = update_view.add_submit(submit.unwrap());
        let update = self
            .slacker
            .post(update_view)
            .map(|result| println!("Post update view result {:?}", result))
            .map_err(|e| println!("Cannot update poll creator {}", e));
        actix::spawn(update);
    }

    pub fn process_channel_change(&self, channel_id: String) {
        use std::sync::LockResult;
        match self.state.lock() {
            Result::Ok(mut guard) => guard.as_mut().unwrap().poll_channel = channel_id,
            Result::Err(err) => err.into_inner().as_mut().unwrap().poll_channel = channel_id,
        }
    }

    pub fn show_dialog_create(&self, trigger_id: String) {
        let blocks = vec![
            LayoutBlock::new_plain_single_line_text_input(
                "Критерий #1",
                "dialog_variant_text_1".to_owned(),
                "Критерий оценки голоса",
            ),
            LayoutBlock::new_plain_single_line_text_input(
                "Максимальная оценка #1",
                "dialog_variant_max_score_1".to_owned(),
                "1-100",
            ),
            LayoutBlock::new_action(vec![BlockElement::new_button(
                "Добавить критерий",
                "dialog_variant_add".to_owned(),
            )])
            .build(),
        ];
        let view_push = ViewPush::new_with_id(
            trigger_id,
            DIALOG_VARIANT_CREATE_ID,
            "Варианты оценок",
            blocks,
        )
        .add_submit("Accept");
        let push_view = self
            .slacker
            .post(view_push)
            .map_err(|e| println!("Error while push view {}", e))
            .map(|resp| println!("Response {:?}", resp));
        actix::spawn(push_view);
    }

    pub fn add_variant_to_dialog(&self, mut old_view: View) {
        println!("Update dialog view");
        let count = old_view
            .blocks
            .iter()
            .filter(|e| {
                if let LayoutBlock::Input { .. } = e {
                    true
                } else {
                    false
                }
            })
            .count();
        let next_id = count / 2 + 1;
        old_view.blocks.insert(
            old_view.blocks.len() - 1,
            LayoutBlock::new_plain_single_line_text_input(
                format!("Критерий #{}", next_id).as_str(),
                format!("dialog_variant_text_{}", next_id),
                "Критерий оценки голоса",
            ),
        );
        old_view.blocks.insert(
            old_view.blocks.len() - 1,
            LayoutBlock::new_plain_single_line_text_input(
                format!("Максимальная оценка #{}", next_id).as_str(),
                format!("dialog_variant_max_score_{}", next_id),
                "1-100",
            ),
        );
        let mut id = old_view.id.clone();
        let mut submit = old_view.submit.clone();
        let mut update_view = ViewUpdate::new(old_view);
        update_view.view_id = id;
        update_view = update_view.add_submit(submit.unwrap());
        let update = self
            .slacker
            .post(update_view)
            .map(|result| println!("Post update view result {:?}", result))
            .map_err(|e| println!("Cannot update poll creator {}", e));
        actix::spawn(update);
    }

    //todo убрать дубли если все работает
    pub fn save_dialog_info(&self, block_action: BlockAction) {
        let mut view = block_action.view.unwrap();
        let mut values = view.state.unwrap().values;
        let mut state = self.state.lock().unwrap();
        let variants = &mut state.as_mut().unwrap().dialog_variants;
        let mut peekable = view
            .blocks
            .iter()
            .filter(|e| {
                if let LayoutBlock::Input { .. } = e {
                    true
                } else {
                    false
                }
            })
            .peekable();
        while let (Option::Some(variant), Option::Some(date)) = (peekable.next(), peekable.peek()) {
            if let (
                LayoutBlock::Input { block_id, .. },
                LayoutBlock::Input {
                    block_id: date_id, ..
                },
            ) = (variant, date)
            {
                let variant = values[block_id.as_ref().unwrap()][block_id.as_ref().unwrap()]
                    .as_object_mut()
                    .unwrap_or(&mut Map::new())
                    .remove("value");
                let start_date = values[date_id.as_ref().unwrap()][date_id.as_ref().unwrap()]
                    .as_object_mut()
                    .unwrap_or(&mut Map::new())
                    .remove("value");
                if let (Option::Some(data), Option::Some(score)) =
                    (variant.to_owned(), start_date.to_owned())
                {
                    variants.push(DialogViewVariant {
                        variant_text: data.as_str().unwrap().to_owned(),
                        max_score: 1..=i32::from_str(score.as_str().unwrap()).unwrap_or(1),
                    })
                }
            }
            peekable.next();
        }
    }

    pub fn save_poll_info(&self, block_action: BlockAction) {
        let mut view = block_action.view.unwrap();
        println!("{:?}", view);
        let mut values = view.state.unwrap().values;
        let mut lock = self.state.lock().unwrap();
        let mut state = lock.as_mut().unwrap();
        let poll_variants = &mut state.poll_variants;
        let mut peekable = view
            .blocks
            .iter()
            .filter(|e| {
                if let LayoutBlock::Input { .. } = e {
                    true
                } else {
                    false
                }
            })
            .peekable();
        while let (Option::Some(title), Option::Some(variant), Option::Some(date)) =
            (peekable.next(), peekable.next(), peekable.peek())
        {
            println!("{:?} {:?} {:?}", title, variant, date);
            if let (
                LayoutBlock::Input {
                    block_id: title_id, ..
                },
                LayoutBlock::Input { block_id, .. },
                LayoutBlock::Input {
                    block_id: date_id, ..
                },
            ) = (title, variant, date)
            {
                println!("{:?} {:?} {:?}", title_id, block_id, date_id);
                let title = values[title_id.as_ref().unwrap()][title_id.as_ref().unwrap()]
                    .as_object_mut()
                    .unwrap_or(&mut Map::new())
                    .remove("value");
                let variant = values[block_id.as_ref().unwrap()][block_id.as_ref().unwrap()]
                    .as_object_mut()
                    .unwrap_or(&mut Map::new())
                    .remove("value");
                let start_date = values[date_id.as_ref().unwrap()][date_id.as_ref().unwrap()]
                    .as_object_mut()
                    .unwrap_or(&mut Map::new())
                    .remove("value");
                if let (Option::Some(title), Option::Some(variant), Option::Some(date)) =
                    (title.to_owned(), variant.to_owned(), start_date.to_owned())
                {
                    poll_variants.push(SingleVariant {
                        id: None,
                        title: title.as_str().unwrap_or(Default::default()).to_owned(),
                        variant: variant.as_str().unwrap_or(Default::default()).to_owned(),
                        images: vec![],
                        votes: None,
                        start_date: NaiveDateTime::from_str(date.as_str().unwrap())
                            .unwrap_or(NaiveDateTime::from_timestamp(0, 0)),
                    })
                }
            }
            peekable.next();
        }
        drop(lock);
        let state = self
            .state
            .lock()
            .unwrap()
            .replace(PollData::default())
            .unwrap();
        println!("{:?}", state);
        let self_state = self.state.clone();
        let database = self.data.clone();
        let slacker = self.slacker.clone();
        let write_time_access = self.data.clone();
        let write_poll = self
            .data
            .send(WriteNewPoll(state))
            .map_err(|e| println!("Cannot write poll {}", e))
            .map(|e| ());
        actix::spawn(write_poll);
    }
}
