use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use slacker::{PostMessage, PostMessageResponse, TextObject, View};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct InteractResponse {
    payload: ActionResponse,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionResponse {
    BlockActions {
        #[serde(flatten)]
        block_action: BlockAction,
    },
    DialogSubmission {
        #[serde(flatten)]
        block_action: BlockAction,
    },
    ViewSubmission {
        #[serde(flatten)]
        block_action: BlockAction,
    },
    MessageActions {
        #[serde(flatten)]
        block_action: BlockAction,
    },
}

impl Default for ActionResponse {
    fn default() -> Self {
        ActionResponse::BlockActions {
            block_action: Default::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct BlockAction {
    pub team: Team,
    pub user: User,
    pub api_app_id: String,
    pub token: String,
    pub container: Container,
    pub trigger_id: String,
    pub callback_id: String,
    pub channel: Channel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<PostMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<View>,
    pub response_url: String,
    pub actions: Vec<Action>,
    pub submission: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct Team {
    id: String,
    domain: String,
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct User {
    pub id: String,
    username: String,
    name: String,
    team_id: String,
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct Container {
    r#type: String,
    message_ts: String,
    attachment_id: String,
    channel_id: String,
    is_ephemeral: bool,
    is_app_unfurl: bool,
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct Channel {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct Action {
    pub action_id: String,
    pub name: String,
    pub selected_options: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_channel: Option<String>,
    pub block_id: String,
    pub text: TextObject,
    pub value: String,
    pub r#type: String,
    pub action_ts: String,
}

#[cfg(test)]
mod test {
    use crate::actions_response::{ActionResponse, InteractResponse};

    #[test]
    fn test() {
        let resp = InteractResponse {
            payload: ActionResponse::BlockActions {
                block_action: Default::default(),
            },
        };
        let string = serde_json::to_string(&resp).unwrap();
        println!("{}", string);
    }
}
