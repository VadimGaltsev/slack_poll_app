use crate::actions_response::BlockAction;
use crate::data::PollReportSource;
use crate::poll_state::PollData;
use crate::ui_poll_view::PollView;
use crate::VIEW_POLL_CREATE_ID;
use chrono::NaiveDateTime;
use futures::Future;
use serde_json::Value;
use slacker::{
    BlockElement, Dialog, DialogElement, DialogOpen, DialogOptionGroup, LayoutBlock,
    MessageVisibility, PostMessage, PostMessageResponse, SlackRequest, Slacker, TextObject,
    UpdateMessage, ViewOpen,
};

//todo change to data base poll time
pub fn update_message_response(
    slacker: Slacker,
    ts: String,
    poll_view: PollView,
) -> Box<dyn Future<Item = (), Error = ()>> {
    let update = UpdateMessage::new("Poll", &poll_view.channel, &ts)
        .with_blocks(create_poll_view(poll_view).into());
    println!("{:?}", update);
    let request = slacker
        .post(update)
        .and_then(|r| {
            println!("{:?}", r);
            Ok(())
        })
        .map_err(|e| println!("{:?}", e));
    Box::new(request)
}

pub fn create_poll_view(
    poll_view: PollView,
) -> impl SlackRequest<PostMessageResponse> + Into<Vec<LayoutBlock>> {
    let mut poll_request = PostMessage::new("Голосование")
        .channel_str(&poll_view.channel)
        .set_response_type(MessageVisibility::InChannel)
        .add_block(LayoutBlock::new_section(TextObject::new_mrkdwn_text(
            "*Голосование*",
        )))
        .add_block(LayoutBlock::new_divider());
    for variant in poll_view.variants {
        let mut images = Vec::new();
        variant
            .images
            .into_iter()
            .rev()
            .take(4)
            .for_each(|url| images.push(BlockElement::new_image(url, "Cannot load".into())));
        poll_request = poll_request.add_block(LayoutBlock::new_section(
            TextObject::new_mrkdwn_text(format!("*{}*", &variant.title).as_str()),
        ));
        poll_request = poll_request.add_block(
            LayoutBlock::new_section(TextObject::new_mrkdwn_text(&variant.variant)).set_accessory(
                BlockElement::new_button(
                    TextObject::new_plain_text("Голосовать"),
                    variant.id.unwrap().to_string(),
                ),
            ),
        );
        let mut context = LayoutBlock::new_context(Vec::<BlockElement>::new());
        if !images.is_empty() {
            context = context.set_elements(images);
        }
        context = context.add_element(if let Some(count) = variant.votes {
            BlockElement::new_text_element(format!("{} votes", count).as_str())
        } else {
            BlockElement::new_text_element("No votes")
        });
        poll_request = poll_request.add_block(context)
    }
    poll_request
}

pub fn create_poll_menu(trigger_id: String) -> impl SlackRequest<PostMessageResponse> {
    let blocks = vec![
        LayoutBlock::new_section("Канал для голосования").build(),
        LayoutBlock::new_action(vec![BlockElement::new_channel_select(
            "Выберите канал",
            "channel_choose".to_owned(),
        )])
        .build(),
        LayoutBlock::new_plain_single_line_text_input(
            "Заголовок #1",
            "title_text_1".to_owned(),
            "Можно в markdown",
        ),
        LayoutBlock::new_plain_text_input("Вариант #1", "variant_text_1".to_owned()),
        LayoutBlock::new_plain_single_line_text_input(
            "Дата начала голосования #1",
            "start_variant_poll_date_1".to_owned(),
            "2015-09-18T23:56:04",
        ),
        LayoutBlock::new_action(vec![BlockElement::new_button(
            "Добавить вариант",
            "variant_add".to_owned(),
        )])
        .build(),
        LayoutBlock::new_action(vec![BlockElement::new_button(
            "Добавить критерии",
            "dialog_setup".to_owned(),
        )])
        .build(),
    ];
    ViewOpen::new_with_id(
        trigger_id,
        VIEW_POLL_CREATE_ID,
        "Создать голосование",
        blocks,
    )
    .add_submit("Next")
}

pub fn show_answered_request_view(
    client: Slacker,
    block_action: BlockAction,
) -> Box<dyn Future<Item = (), Error = ()>> {
    Box::new(
        client
            .post(
                ViewOpen::new(
                    block_action.trigger_id,
                    "Sorry",
                    vec![LayoutBlock::new_section("Ваш голос уже учтён! Спасибо!")],
                )
                .add_submit("Понятно"),
            )
            .map_err(|e| println!("Cannot post message {}", e))
            .map(|post| println!("Response {:?}", post)),
    )
}

pub fn show_not_ready_request_view(
    client: Slacker,
    block_action: BlockAction,
    start_date: NaiveDateTime,
) -> Box<dyn Future<Item = (), Error = ()>> {
    Box::new(
        client
            .post(
                ViewOpen::new(
                    block_action.trigger_id,
                    "Sorry",
                    vec![LayoutBlock::new_section(
                        format!(
                            "Голосование еще не началось! Ожидаем {}!",
                            start_date.format("%Y-%m-%d %H:%M:%S")
                        )
                        .as_str(),
                    )],
                )
                .add_submit("Понятно"),
            )
            .map_err(|e| println!("Cannot post message {}", e))
            .map(|post| println!("Response {:?}", post)),
    )
}

pub fn create_poll_report_view(
    poll_view: Vec<PollReportSource>,
) -> impl SlackRequest<PostMessageResponse> + Into<Vec<LayoutBlock>> {
    let mut poll_request = PostMessage::new("*Результаты голосования*")
        .channel_str(&poll_view.first().unwrap().channel)
        .add_block(LayoutBlock::new_section(TextObject::new_mrkdwn_text(
            "*Результаты голосования*",
        )))
        .add_block(LayoutBlock::new_divider());

    for report in poll_view.into_iter().enumerate() {
        poll_request =
            poll_request.add_block(LayoutBlock::new_section(TextObject::new_mrkdwn_text(
                format!("{}{}", convert_to_word(report.0 as i32 + 1), report.1.team).as_str(),
            )));
        poll_request = poll_request.add_block(LayoutBlock::new_context(vec![
            BlockElement::new_mrkdwn_text_element(
                format!("*{}* votes", report.1.total_votes).as_str(),
            ),
            BlockElement::new_mrkdwn_text_element(
                format!("*{:.2}* points", report.1.score).as_str(),
            ),
        ]));
    }
    poll_request
}

fn convert_to_word(num: i32) -> String {
    match num {
        1 => "🏆*Первое место:*\n",
        2 => "🌟*Второе место:*\n",
        3 => "🌟*Третье место:*\n",
        _ => "",
    }
    .to_owned()
}
