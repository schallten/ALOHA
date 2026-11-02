use fluxer_builders::{EmbedBuilder, MessagePayload};
use fluxer_core::client::typed_events::DispatchEvent;
use fluxer_core::client::{Client, ClientOptions};
use fluxer_rest::Rest;
use fluxer_types::gateway::{GatewayActivity, GatewayCustomStatus, GatewayPresenceUpdateSendData};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex}; // for boolean

struct CallManager {
    user_queue: VecDeque<String>,
    active_calls: Vec<(String, String)>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // stack to hold users
    let call_manager = Arc::new(Mutex::new(CallManager {
        user_queue: VecDeque::new(),
        active_calls: Vec::new(),
    }));

    tracing_subscriber::fmt().with_env_filter("info").init();

    let token = std::env::var("FLUXER_TOKEN").expect("set FLUXER_TOKEN env var");

    // let v:Vec<GatewayActivity>
    // 1. Create individual activities
    let activity1 = GatewayActivity {
        name: String::from("Playing Rust"),
        kind: 0,
        url: None,
    };

    let activity2 = GatewayActivity {
        name: String::from("Streaming"),
        kind: 1,
        url: Some(String::from("https://twitch.tv")),
    };

    // 2. Put them into a vector
    let my_activities: Vec<GatewayActivity> = vec![activity1, activity2];

    let options = ClientOptions {
        intents: 0,
        wait_for_guilds: true,
        presence: Some(GatewayPresenceUpdateSendData {
            since: None,
            activities: Some(my_activities),
            custom_status: Some(GatewayCustomStatus {
                text: Some(String::from("Hello everyone!")),
                emoji_name: Some(String::from("grin")),
                emoji_id: None,
            }),
            status: String::from("online"),
            afk: Some(true),
        }),
        ..Default::default()
    };

    let mut client = Client::new(options);
    let rest: Rest = client.rest.clone();
    let call_manager = Arc::clone(&call_manager);

    client.on_typed(move |event| {
        let rest = rest.clone();
        let call_manager = Arc::clone(&call_manager);

        Box::pin(async move {
            match event {
                DispatchEvent::Ready => {
                    tracing::info!("ALOHA is online");
                }

                DispatchEvent::MessageCreate { message, .. } => {
                    if message.content.trim() == "!ping" {
                        let embed = EmbedBuilder::new().title("Pong").color(0x5865F2).build();

                        let payload = MessagePayload::new()
                            .add_embed(embed)
                            .reply(
                                message.channel_id.clone(),
                                message.id.clone(),
                                message.guild_id.clone(),
                            )
                            .build();

                        if let Err(e) = message.send(&rest, &payload).await {
                            tracing::error!("send {e}");
                        }
                    }
                    
                    if message.content.trim() == "!userphone" {
    println!("user id was {}", message.author.id);

    let user_id = message.author.id.to_string();

    let (title, description, color) = {
        let mut manager = call_manager.lock().unwrap();

        if manager.user_queue.contains(&user_id) {
            println!("User is already in queue!");

            (
                "Already in Queue",
                format!(
                    "You're already in the queue, <@{}>!\nPlease wait for your turn.",
                    message.author.id
                ),
                0xF1C40F,
            )
        } else {
            manager.user_queue.push_back(user_id);

            println!("User added to queue!");

            (
                "Successfully Joined",
                format!(
                    "You have successfully joined the queue, <@{}>!\nPlease wait while we establish your connection.",
                    message.author.id
                ),
                0x2ECC71,
            )
        }
    };

    let embed = EmbedBuilder::new()
        .title(title)
        .description(description)
        .color(color)
        .build();

    let payload = MessagePayload::new()
        .add_embed(embed)
        .reply(
            message.channel_id.clone(),
            message.id.clone(),
            message.guild_id.clone(),
        )
        .build();

    if let Err(e) = message.send(&rest, &payload).await {
        tracing::error!("send {e}");
    }
}

                    if message.content.trim() == "!hangup" {
                        let user_id = message.author.id.to_string();
                        let (in_call, pair_index) = {
                            let manager = call_manager.lock().unwrap();
                            let mut found = false;
                            let mut idx = usize::MAX;
                            for (i, (a, b)) in manager.active_calls.iter().enumerate() {
                                if a == &user_id || b == &user_id {
                                    found = true;
                                    idx = i;
                                    break;
                                }
                            }
                            (found, idx)
                        };

                        let embed = if in_call {
                            println!("in call was true");
                            {
                                let mut manager = call_manager.lock().unwrap();
                                if pair_index != usize::MAX {
                                    manager.active_calls.remove(pair_index);
                                }
                                manager.user_queue.retain(|id| id != &user_id);
                            }
                            EmbedBuilder::new()
                                .title("Hanging up")
                                .color(0x27D3F5)
                                .build()
                        } else {
                            println!("in call wasn't true");
                            {
                                let mut manager = call_manager.lock().unwrap();
                                manager.user_queue.retain(|id| id != &user_id);
                            }
                            EmbedBuilder::new()
                                .title("you weren't in a call")
                                .color(0x27D3F5)
                                .build()
                        };

                        let payload = MessagePayload::new()
                            .add_embed(embed)
                            .reply(
                                message.channel_id.clone(),
                                message.id.clone(),
                                message.guild_id.clone(),
                            )
                            .build();

                        if let Err(e) = message.send(&rest, &payload).await {
                            tracing::error!("send {e}");
                        }
                    }
                }

                _ => {
                    tracing::info!("Received another event");
                }
            }
        })
    });

    if let Err(e) = client.login(&token).await {
        tracing::error!("login failed: {e:?}");
    }
}
