#[macro_use]
extern crate actix_web;
#[macro_use]
extern crate diesel;

use actix_http::error;
use actix_rt::System;
use actix_web::FromRequest;
use std::str::FromStr;

mod actions_response;
mod application;
mod data;
mod imports;
mod poll_state;
mod schema;
mod slack_ui;
mod ui_poll_view;

imports!();

const VARIANT_ADD: &str = "variant_add";
const CHANNEL_CHOOSE: &str = "channel_choose";
const DIALOG_SETUP: &str = "dialog_setup";
const DIALOG_VARIANT_ADD: &str = "dialog_variant_add";
pub const VIEW_POLL_CREATE_ID: &str = "view_poll_create";
pub const DIALOG_VARIANT_CREATE_ID: &str = "dialog_variant_create";

#[post("/dialog")]
fn dialog_response(
    request: HttpRequest,
    payload: Form<HashMap<String, String>>,
    application: Data<SlackApplication>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let action: ActionResponse =
        serde_json::from_str::<ActionResponse>(&payload.get("payload").unwrap()).unwrap();
    println!("{:?}", action);
    match action {
        ActionResponse::BlockActions { block_action } => {
            let action_id = block_action.actions.first().unwrap().action_id.clone();
            match action_id.as_str() {
                VARIANT_ADD => application.add_variant_to_poll(block_action.view.unwrap()),
                CHANNEL_CHOOSE => application.process_channel_change(
                    block_action
                        .actions
                        .first()
                        .unwrap()
                        .selected_channel
                        .clone()
                        .unwrap(),
                ),
                DIALOG_SETUP => application.show_dialog_create(block_action.trigger_id),
                DIALOG_VARIANT_ADD => application.add_variant_to_dialog(block_action.view.unwrap()),
                _ => application.post_dialog_on_request(block_action),
            }
        }
        ActionResponse::ViewSubmission { block_action } => {
            let view = block_action.view.as_ref().unwrap();
            println!("{:?}", view.callback_id);
            match view.callback_id.as_ref().unwrap_or(&"".to_owned()).as_str() {
                DIALOG_VARIANT_CREATE_ID => application.save_dialog_info(block_action),
                VIEW_POLL_CREATE_ID => application.save_poll_info(block_action),
                _ => (),
            }
        }
        ActionResponse::DialogSubmission { block_action } => {
            application.process_dialog_submission(block_action)
        }
        _ => (),
    };
    HttpResponse::Ok().respond_to(&request)
}

#[post("/post_poll")]
fn post_poll_response(
    request: HttpRequest,
    payload: Form<HashMap<String, String>>,
    application: Data<SlackApplication>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("{:?}", payload);
    if !application.user_admin.is_empty()
        && application.user_admin != payload[&"user_id".to_owned()]
    {
        return HttpResponse::Forbidden().respond_to(&request);
    }
    application.post_last_poll_to_channel(payload[&"trigger_id".to_owned()].clone());
    HttpResponse::Ok().respond_to(&request)
}

#[post("/create_poll")]
fn create_poll_response(
    request: HttpRequest,
    payload: Form<HashMap<String, String>>,
    application: Data<SlackApplication>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("{:?}", payload);
    if !application.user_admin.is_empty()
        && application.user_admin != payload[&"user_id".to_owned()]
    {
        return HttpResponse::Forbidden().respond_to(&request);
    }
    application.process_poll_request(payload[&"trigger_id".to_owned()].clone());
    HttpResponse::Ok().respond_to(&request)
}

#[post("/close_and_post_report")]
fn close_poll_and_post_report_response(
    request: HttpRequest,
    payload: Form<HashMap<String, String>>,
    application: Data<SlackApplication>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("{:?}", payload);
    if !application.user_admin.is_empty()
        && application.user_admin != payload[&"user_id".to_owned()]
    {
        return HttpResponse::Forbidden().respond_to(&request);
    }
    application.close_poll_and_create_report_request(payload[&"trigger_id".to_owned()].clone());
    HttpResponse::Ok().respond_to(&request)
}

fn main() -> Result<(), std::io::Error> {
    std::env::set_var("RUST_LOG", "actix_web=debug,actix_server=debug");
    let _ = System::new("Poll_application");
    let application = Data::new(SlackApplication::new());
    env_logger::init();
    let app = move || {
        App::new()
            .register_data(application.clone())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api/slack")
                    .data(web::Form::<HashMap<String, String>>::configure(|cfg| {
                        cfg.limit(10240000)
                    }))
                    .service(create_poll_response)
                    .service(dialog_response)
                    .service(post_poll_response)
                    .service(close_poll_and_post_report_response),
            )
    };

    let workers = std::env::var("WORKERS").unwrap_or(String::default());
    let workers_count = usize::from_str(workers.as_str()).unwrap_or(num_cpus::get());
    HttpServer::new(app)
        .workers(workers_count)
        .bind(format!(
            "127.0.0.1:{}",
            std::env::var("PORT").unwrap_or("8888".to_owned())
        ))?
        .run()
}
