#[macro_export]
macro_rules! imports {
    () => {
        use crate::actions_response::InteractResponse;
        use crate::application::SlackApplication;
        use crate::data::*;
        use crate::slack_ui::{create_poll_view, update_message_response};
        use crate::ui_poll_view::{PollView, SingleVariant};
        use actions_response::ActionResponse;
        use actix::Addr;
        use actix_http::http::Method;
        use actix_web::web::{Data, Form};
        use actix_web::{
            middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
        };
        use dotenv::dotenv;
        use slacker::Future;
        use slacker::{PostMessageResponse, Slacker};
        use std::collections::HashMap;
        use std::env;
    };
}
