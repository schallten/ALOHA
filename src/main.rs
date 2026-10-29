// import the Fluxer client library
use fluxer_core::client::typed_events::DispatchEvent;
use fluxer_core::client::{Client, ClientOptions};

// tokio::main lets us use async/await in main
#[tokio::main]
async fn main() {
    // load variables from .env file
    dotenvy::dotenv().ok();

    // set up logging so we can see info/error messages
    tracing_subscriber::fmt().with_env_filter("info").init();

    // grab the bot token from the environment variable
    let token = std::env::var("FLUXER_TOKEN").expect("set FLUXER_TOKEN env var");

    // configure how the bot connects
    let options = ClientOptions {
        intents: 0,            // no special gateway intents needed yet
        wait_for_guilds: true, // wait until server data is loaded
        ..Default::default()   // use defaults for everything else
    };

    // create the bot client
    let mut client = Client::new(options);

    // register an event handler that runs on every gateway event
    client.on_typed(move |event| {
        Box::pin(async move {
            // when the bot is ready, log it
            if let DispatchEvent::Ready = event {
                tracing::info!("ALOHA is online");
            }
        })
    });

    // connect to Fluxer with the token (stays running / online)
    if let Err(e) = client.login(&token).await {
        tracing::error!("login failed: {e:?}");
    }
}
