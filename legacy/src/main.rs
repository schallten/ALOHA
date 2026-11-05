// We keep plain nested `if`s on purpose (easy to read for newcomers), so don't
// let clippy push us into `let`-chains here.
#![allow(clippy::collapsible_if)]

use fluxer_builders::{EmbedBuilder, MessagePayload};
use fluxer_core::client::typed_events::DispatchEvent;
use fluxer_core::client::{Client, ClientOptions};
use fluxer_rest::Rest;
use fluxer_types::gateway::{GatewayActivity, GatewayCustomStatus, GatewayPresenceUpdateSendData};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex}; // for boolean
use std::time::{Duration, Instant};

// Hard caps so the bot can't melt under load.
const MAX_ACTIVE_PAIRS: usize = 100;
const MAX_QUEUE_SIZE: usize = 200;

// Minimum gap between any two messages the bot sends, to stay well inside
// the platform's per-route rate limits.
const MIN_SEND_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
struct Call {
    user1: String,
    channel1: String,
    guild1: String,
    user2: String,
    channel2: String,
    guild2: String,
}

impl Call {
    // Given one user's ID, return the channel (and guild) of the other user
    fn partner_destination(&self, user_id: &str) -> Option<(String, String)> {
        if self.user1 == user_id {
            Some((self.channel2.clone(), self.guild2.clone()))
        } else if self.user2 == user_id {
            Some((self.channel1.clone(), self.guild1.clone()))
        } else {
            None
        }
    }

    fn contains(&self, user_id: &str) -> bool {
        self.user1 == user_id || self.user2 == user_id
    }
}

struct CallManager {
    user_queue: VecDeque<(String, String, String)>,
    active_calls: Vec<Call>,
}

// Pops queued users two at a time and returns the pairs that were just formed,
// so the caller can notify them after dropping the lock. Stops at MAX_ACTIVE_PAIRS.
fn pair_waiting_users(manager: &mut CallManager) -> Vec<Call> {
    let mut new_pairs = Vec::new();

    while manager.user_queue.len() >= 2 && manager.active_calls.len() < MAX_ACTIVE_PAIRS {
        let (user1, channel1, guild1) = manager.user_queue.pop_front().unwrap();
        let (user2, channel2, guild2) = manager.user_queue.pop_front().unwrap();

        println!("Pair created: {} <-> {}", user1, user2);
        let call = Call {
            user1,
            channel1,
            guild1,
            user2,
            channel2,
            guild2,
        };
        manager.active_calls.push(call.clone());
        new_pairs.push(call);
    }

    new_pairs
}

// Keeps a small gap between outgoing messages, per channel.
// Each channel remembers when we last sent to it, so one busy call
// doesn't slow down every other call.
struct Pacer {
    last_sent: tokio::sync::Mutex<HashMap<String, Instant>>,
}

impl Pacer {
    fn new() -> Self {
        Self {
            last_sent: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    // Waits until it is okay to send to this channel, then records the time.
    async fn wait(&self, channel_id: &str) {
        loop {
            // The block ends (and the lock is released) before we sleep,
            // so other channels can send while this one waits.
            let wait_time = {
                let mut last_sent = self.last_sent.lock().await;

                match last_sent.get(channel_id) {
                    Some(previous) => {
                        let elapsed = previous.elapsed();

                        if elapsed < MIN_SEND_INTERVAL {
                            // Too soon: remember how long is left.
                            Some(MIN_SEND_INTERVAL - elapsed)
                        } else {
                            last_sent.insert(channel_id.to_string(), Instant::now());
                            None
                        }
                    }
                    None => {
                        last_sent.insert(channel_id.to_string(), Instant::now());
                        None
                    }
                }
            };

            match wait_time {
                Some(time) => tokio::time::sleep(time).await,
                None => return, // it's our turn
            }
        }
    }
}

const COLOR_CONNECTED: u32 = 0x2ECC71;
const COLOR_DISCONNECTED: u32 = 0xE74C3C;
const COLOR_INFO: u32 = 0x27D3F5;

// The platform rejects anything longer than this, so we cut messages to fit.
const RELAY_CONTENT_LIMIT: usize = 2000;

// An invisible character (zero-width space). We put it inside a mention so the
// platform no longer sees a mention, while the text still looks the same.
const INVISIBLE: char = '\u{200b}';

/// Messages are relayed into someone else's server, so they must not be able to
/// ping anyone there
///   "<@123>"          becomes "<" + invisible + "@123>"
///   "@everyone"       becomes "@" + invisible + "everyone"
///   "@here"           becomes "@" + invisible + "here"
fn neutralize_mentions(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        out.push(c);

        // "<@..." is how a user or role mention starts, so split it up.
        if c == '<' && chars.peek() == Some(&'@') {
            out.push(INVISIBLE);
        }

        // "@everyone" and "@here" ping whole servers. Check the next few
        // characters, ignoring upper/lower case.
        if c == '@' {
            let next: String = chars.clone().take(8).collect();
            let next = next.to_lowercase();
            let is_everyone = next.starts_with("everyone");
            let is_here = next.starts_with("here");

            if is_everyone || is_here {
                out.push(INVISIBLE);
            }
        }
    }

    out
}

/// Builds the line we actually send: "**name:** message", with mentions removed
/// and the length cut down if needed. We count whole characters so we never
/// slice a multi-byte character in half (that would panic).
fn relay_text(username: &str, content: &str) -> String {
    // Clean the whole line, not just the message: a username could be
    // "@everyone" too, and the platform would honor that just the same.
    let full = neutralize_mentions(&format!("**{username}:** {}", sanitize_content(content)));

    if full.len() <= RELAY_CONTENT_LIMIT {
        return full;
    }

    // Too long: keep as many characters as fit, plus "...".
    let keep_under = RELAY_CONTENT_LIMIT - 3;
    let mut out = String::new();
    let mut used = 0;

    for c in full.chars() {
        let size = c.len_utf8();

        if used + size > keep_under {
            break;
        }

        out.push(c);
        used += size;
    }

    out.push_str("...");
    out
}

/// Things that would make the recipient's client go fetch something, or turn
/// into a clickable link someone could be tricked into opening.
const LINK_MARKERS: [&str; 4] = ["http://", "https://", "www.", "data:"];

/// Removes anything link-like. The recipient is a stranger, so their client
/// must not be made to load an image, a tracking pixel, or a phishing page.
fn remove_links(text: &str) -> String {
    let mut out = String::new();

    // `split_inclusive` keeps the space or newline that ended each word.
    for word in text.split_inclusive(char::is_whitespace) {
        let lower = word.to_lowercase();
        let mut has_link = false;

        for marker in LINK_MARKERS {
            if lower.contains(marker) {
                has_link = true;
            }
        }

        if has_link {
            out.push_str("[link removed]");

            // Add back the whitespace this word ended with.
            let trimmed_len = word.trim_end().len();
            out.push_str(&word[trimmed_len..]);
        } else {
            out.push_str(word);
        }
    }

    out
}

/// "![alt](url)" is how an image gets embedded, so turn it into a plain-text
/// link first. "](url)" is turned into "] (url)" for the same reason.
fn remove_markdown_links(text: &str) -> String {
    let text = text.replace("![", "[");
    text.replace("](", "] (")
}

/// Adds an invisible character after every formatting symbol, so nothing can
/// render as bold, italic, a code block, a spoiler or a strikethrough.
fn defuse_markdown(text: &str) -> String {
    let mut out = String::new();

    for c in text.chars() {
        out.push(c);

        if c == '*' || c == '_' || c == '`' || c == '~' || c == '|' {
            out.push(INVISIBLE);
        }
    }

    out
}

/// Everything we do to a message before it crosses into someone else's server.
fn sanitize_content(text: &str) -> String {
    let text = remove_links(text);
    let text = remove_markdown_links(&text);
    defuse_markdown(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mentions_are_broken() {
        let out = neutralize_mentions("<@123> @everyone @HERE hello @bob");

        assert!(!out.contains("<@"));
        assert!(!out.contains("@everyone"));
        assert!(!out.contains("@HERE"));
        // a normal "@name" is not a mention for the platform, so it is fine
        assert!(out.contains("@bob"));
    }

    #[test]
    fn long_messages_are_cut() {
        let long = "❤".repeat(2000);
        let line = relay_text("someone", &long);

        assert!(line.len() <= RELAY_CONTENT_LIMIT);
        assert!(line.ends_with("..."));
    }

    #[test]
    fn short_messages_pass_through() {
        assert_eq!(relay_text("bob", "hello"), "**bob:** hello");
    }

    #[test]
    fn username_cannot_ping() {
        let line = relay_text("@everyone", "hi");

        assert!(!line.contains("@everyone"));
        assert!(line.contains("everyone"));
    }

    #[test]
    fn links_are_removed() {
        let line = relay_text("bob", "look https://klipy.com/a.gif and www.evil.com");

        assert!(!line.contains("https://"));
        assert!(!line.contains("www."));
        assert!(line.contains("[link removed]"));
    }

    #[test]
    fn gif_images_cannot_embed() {
        let line = relay_text("bob", "![cat](https://evil.com/a.png)");

        assert!(!line.contains("https://"));
        assert!(!line.contains("!["));
        assert!(!line.contains("]("));
    }

    #[test]
    fn formatting_is_defused() {
        let body = sanitize_content("**fake name:** ||secret|| ~~old~~ ``code`` snake_case");

        // none of these can render as formatting any more
        assert!(!body.contains("**"));
        assert!(!body.contains("||"));
        assert!(!body.contains("~~"));
        assert!(!body.contains("``"));

        // the words are still readable once the invisible characters are ignored
        let visible: String = body.chars().filter(|c| *c != INVISIBLE).collect();
        assert!(visible.contains("fake name"));
        assert!(visible.contains("snake_case"));
    }

    #[test]
    fn bot_prefix_stays_bold() {
        let line = relay_text("bob", "hi");

        assert!(line.starts_with("**bob:**"));
    }
}

async fn send_embed(
    rest: &Rest,
    pacer: &Pacer,
    channel_id: &str,
    embed: fluxer_types::embed::ApiEmbed,
) {
    pacer.wait(channel_id).await;

    let payload = MessagePayload::new().add_embed(embed).build();

    if let Err(e) = rest
        .post::<serde_json::Value>(
            &fluxer_types::Routes::channel_messages(channel_id),
            Some(&payload),
        )
        .await
    {
        tracing::error!("failed to post to channel {channel_id}: {e}");
    }
}

// Tells both users that they have been paired up. `skip_channel` is the channel
// that already got the "Connection Found" reply from the command itself.
async fn announce_connection(rest: &Rest, pacer: &Pacer, call: &Call, skip_channel: Option<&str>) {
    if Some(call.channel1.as_str()) != skip_channel {
        send_connected_embed(rest, pacer, &call.channel1, &call.user2).await;
    }

    if Some(call.channel2.as_str()) != skip_channel {
        send_connected_embed(rest, pacer, &call.channel2, &call.user1).await;
    }
}

async fn send_connected_embed(rest: &Rest, pacer: &Pacer, channel_id: &str, partner_id: &str) {
    let embed = EmbedBuilder::new()
        .title("Connection Found ☎️")
        .description(format!(
            "You've been connected with <@{partner_id}>!\n\n\
             Say hi, be kind, and please keep it friendly. 💚\n\
             When you're done, type `!hangup`."
        ))
        .color(COLOR_CONNECTED)
        .build();

    send_embed(rest, pacer, channel_id, embed).await;
}

// Tells the person left behind that their partner hung up.
async fn announce_hangup(
    rest: &Rest,
    pacer: &Pacer,
    partner_id: &str,
    partner_channel: &str,
    hanger_id: &str,
) {
    let embed = EmbedBuilder::new()
        .title("Call Ended")
        .description(format!(
            "<@{hanger_id}> hung up the call.\n\n\
             Hope it was a good chat! If you'd like to talk to someone new, \
             type `!userphone` to join the queue again."
        ))
        .color(COLOR_DISCONNECTED)
        .build();

    tracing::info!("notifying {partner_id} that {hanger_id} hung up");
    send_embed(rest, pacer, partner_channel, embed).await;
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // stack to hold users
    let call_manager = Arc::new(Mutex::new(CallManager {
        user_queue: VecDeque::new(),
        active_calls: Vec::new(),
    }));

    let log_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| String::from("info"));
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

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
    let _worker_test = rest.clone();
    let call_manager = Arc::clone(&call_manager);
    let pacer = Arc::new(Pacer::new());

    // everyone inside owns what it uses hence the move
    client.on_typed(move |event| {
        let rest = rest.clone();
        let call_manager = Arc::clone(&call_manager);
        let pacer = Arc::clone(&pacer);

        Box::pin(async move {
            match event {
                DispatchEvent::Ready => {
                    tracing::info!("ALOHA is online");
                }



                DispatchEvent::MessageCreate { message, .. } => {

                    let content = message.content.trim().to_string();

                    if !content.starts_with('!') {
                        let user_id = message.author.id.to_string();

                        let partner_destination = {
                            let manager = call_manager.lock().unwrap();
                            manager
                                .active_calls
                                .iter()
                                .find_map(|call| call.partner_destination(&user_id))
                        };

                        if let Some((partner_channel, partner_guild)) = partner_destination {
                            let relay_text = relay_text(&message.author.username, &content);

                            let payload = MessagePayload::new().content(relay_text).build();

                            // Don't let a busy pair hammer the API.
                            pacer.wait(&partner_channel).await;

                            // `message.send()` posts back into *this* channel. We need the
                            // partner's channel, which may live in another guild.
                            if let Err(e) = message.send_to(&rest, &partner_channel, &payload).await {
                                tracing::error!(
                                    "relay {user_id} -> channel {partner_channel} (guild {partner_guild}) failed: {e}"
                                );
                            }
                        }
                    }

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
                        let mut new_pairs = Vec::new();

                        let (title, description, color, image_url) = {
                            let mut manager = call_manager.lock().unwrap();

                            let already_active = manager.active_calls.iter().any(|c| c.contains(&user_id));

                            if manager.user_queue.iter().any(|(id, _, _)| id == &user_id) || already_active {
                                println!("User is already in queue!");

                                (
                                    "Already in Queue",
                                    format!(
                                        "You're already in the queue, <@{}>!\nPlease wait for your turn.",
                                        message.author.id
                                    ),
                                    0xF1C40F,
                                    Some("https://media1.tenor.com/m/v52lOlHNtjAAAAAd/ronaldo-cheeky-boy.gif".to_string()),
                                )
                            } else if manager.active_calls.len() >= MAX_ACTIVE_PAIRS {
                                println!(
                                    "pair cap reached ({} active)",
                                    manager.active_calls.len()
                                );

                                (
                                    "All Lines Busy 📵",
                                    format!(
                                        "All **{MAX_ACTIVE_PAIRS}** call slots are in use right now, <@{}>.\n\
                                         Someone will hang up soon — please try again in a little while. 🙏",
                                        message.author.id
                                    ),
                                    COLOR_DISCONNECTED,
                                    Some("https://media1.tenor.com/m/v52lOlHNtjAAAAAd/ronaldo-cheeky-boy.gif".to_string()),
                                )
                            } else if manager.user_queue.len() >= MAX_QUEUE_SIZE {
                                println!("queue cap reached ({} waiting)", manager.user_queue.len());

                                (
                                    "Queue Full 🚦",
                                    format!(
                                        "The waiting queue is full (**{MAX_QUEUE_SIZE}** people), <@{}>.\n\
                                         Please try again in a little while. 🙏",
                                        message.author.id
                                    ),
                                    COLOR_DISCONNECTED,
                                    Some("https://media1.tenor.com/m/v52lOlHNtjAAAAAd/ronaldo-cheeky-boy.gif".to_string()),
                                )
                            } else {
                                manager.user_queue.push_back((
                                    user_id.clone(),
                                    message.channel_id.to_string(),
                                    message.guild_id.clone().unwrap_or_default().to_string(),
                                ));
                                println!("User added to queue!");

                                // Try to create pairs
                                new_pairs = pair_waiting_users(&mut manager);

                                let (title, description, color) = if let Some(call) =
                                    new_pairs.iter().find(|c| c.contains(&user_id))
                                {
                                    let partner = if call.user1 == user_id {
                                        &call.user2
                                    } else {
                                        &call.user1
                                    };
                                    (
                                        "Connection Found ☎️",
                                        format!(
                                            "You've been connected with <@{partner}>!\n\n\
                                             Say hi, be kind, and please keep it friendly. 💚\n\
                                             When you're done, type `!hangup`."
                                        ),
                                        COLOR_CONNECTED,
                                    )
                                } else {
                                    (
                                        "Successfully Joined",
                                        format!(
                                            "You have successfully joined the queue, <@{}>!\nPlease wait while we establish your connection.",
                                            message.author.id
                                        ),
                                        COLOR_INFO,
                                    )
                                };

                                (
                                    title,
                                    description,
                                    color,
                                    Some("https://media1.tenor.com/m/47EjvEixiNUAAAAd/cop-detective.gif".to_string()),
                                )
                            }
                        };

                        let mut embed_builder = EmbedBuilder::new()
                            .title(title)
                            .description(description)
                            .color(color);
                        if let Some(url) = image_url {
                            embed_builder = embed_builder.image(url);
                        }
                        let embed = embed_builder.build();

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

                        // Notify both sides of every pair formed by this command.
                        // The channel that ran the command already got the reply above.
                        let own_channel = message.channel_id.to_string();
                        for call in &new_pairs {
                            announce_connection(&rest, &pacer, call, Some(&own_channel)).await;
                        }
                    }

                    if message.content.trim() == "!hangup" {
                        let user_id = message.author.id.to_string();

                        let (ended_call, new_pairs) = {
                            let mut manager = call_manager.lock().unwrap();
                            let position =
                                manager.active_calls.iter().position(|c| c.contains(&user_id));

                            manager.user_queue.retain(|(id, _, _)| id != &user_id);

                            let ended_call = position.map(|index| manager.active_calls.remove(index));

                            // Freeing a slot may let waiting users pair up now.
                            let new_pairs = if ended_call.is_some() {
                                pair_waiting_users(&mut manager)
                            } else {
                                Vec::new()
                            };

                            (ended_call, new_pairs)
                        };

                        let embed = if ended_call.is_some() {
                            println!("in call was true");
                            EmbedBuilder::new()
                                .title("Hanging up")
                                .description("You've left the call. Thanks for being kind! 💚")
                                .color(COLOR_INFO)
                                .image("https://media1.tenor.com/m/rYE88J8ArOUAAAAd/phone-anime.gif")
                                .build()
                        } else {
                            println!("in call wasn't true");
                            EmbedBuilder::new()
                                .title("you weren't in a call")
                                .color(COLOR_INFO)
                                .image("https://media1.tenor.com/m/rYE88J8ArOUAAAAd/phone-anime.gif")
                                .build()
                        };

                        // Let the other side know they've been left alone.
                        if let Some(call) = ended_call {
                            if let Some((partner_channel, _)) = call.partner_destination(&user_id)
                            {
                                let partner_id = if call.user1 == user_id {
                                    call.user2.clone()
                                } else {
                                    call.user1.clone()
                                };

                                announce_hangup(
                                    &rest,
                                    &pacer,
                                    &partner_id,
                                    &partner_channel,
                                    &user_id,
                                )
                                .await;
                            }
                        }

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

                        // Tell any pair that just formed from the waiting queue.
                        for call in &new_pairs {
                            announce_connection(&rest, &pacer, call, None).await;
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
