use fluxer_builders::{EmbedBuilder, MessagePayload};
use fluxer_core::client::typed_events::DispatchEvent;
use fluxer_core::client::{Client, ClientOptions};
use fluxer_rest::Rest;
use fluxer_types::gateway::{GatewayActivity, GatewayCustomStatus, GatewayPresenceUpdateSendData};
use std::sync::{Arc, Mutex}; // for boolean

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt().with_env_filter("info").init();

    let in_call = Arc::new(Mutex::new(false));

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
    let in_call = Arc::clone(&in_call);

    client.on_typed(move |event| {
        let rest = rest.clone();
        let in_call = Arc::clone(&in_call);

        Box::pin(async move {
            match event {
                DispatchEvent::Ready => {
                    tracing::info!("ALOHA is online");
                }

                DispatchEvent::MessageCreate { message, .. } => {
                    if message.content.trim() == "!ping" {
                        let embed = EmbedBuilder::new().title("Pong").color(0x5865F2).build();

                        let payload = MessagePayload::new().add_embed(embed).build();

                        if let Err(e) = message.send(&rest, &payload).await {
                            tracing::error!("send {e}");
                        }
                    }
                    if message.content.trim() == "!userphone" {
                        println!("user id was {}", message.author.id);
                        let embed = EmbedBuilder::new()
                            .title("Establishing connection")
                            .description(format!(
                                "Remember to be a nice person <@{}>",
                                message.author.id
                            ))
                            .color(0xF54927)
                            .build();

                        let payload = MessagePayload::new().add_embed(embed).build();

                        if let Err(e) = message.send(&rest, &payload).await {
                            tracing::error!("send {e}");
                        }
                        println!("on call setting true");
                        // in_call = true;
                        println!("on call should be true now");
                        *in_call.lock().unwrap() = true;

                        if *in_call.lock().unwrap() {
                            println!("Hello world!");
                        }
                    }
                    if message.content.trim() == "!hangup" {
                        let embed;
                        if *in_call.lock().unwrap() {
                            println!("in call was true");

                            *in_call.lock().unwrap() = false;

                            embed = EmbedBuilder::new()
                                .title("Hanging up")
                                .color(0x27D3F5)
                                .build();
                        } else {
                            println!("in call wasn't true");

                            embed = EmbedBuilder::new()
                                .title("you weren't in a call")
                                .color(0x27D3F5)
                                .build();
                        }

                        let payload = MessagePayload::new().add_embed(embed).build();
                        if *in_call.lock().unwrap() {
                            *in_call.lock().unwrap() = false;

                            println!("in call should be false now");

                            if *in_call.lock().unwrap() {
                                println!("didn't become false");
                            }
                        }

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
