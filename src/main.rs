mod utility;
use dotenvy::dotenv;
use regex::Regex;
use serenity::{
    all::{CreateAllowedMentions, CreateMessage, GetMessages},
    http::Http,
};
use std::env;
use utility::constants::{
    AFFFILIATES_CHANNEL_ID, DISCORD_EMOJI, FRIENDS_CHAT_EMOJI, RAW_AFFILIATES_CHANNEL_ID,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenv().ok();
    let discord_token = env::var("DISCORD_TOKEN").expect("Error retrieving DISCORD_TOKEN.");
    let client = Http::new(&discord_token);

    // Fetch the messages in the channel.
    let messages = RAW_AFFILIATES_CHANNEL_ID
        .messages(&client, GetMessages::new().limit(100))
        .await;

    match messages {
        Ok(mut messages) => {
            messages.reverse();

            for message in messages {
                // Everything is within an embed. If there is no embed, continue.
                let embed = match message.embeds.first() {
                    Some(embed) => embed,
                    None => continue,
                };

                // We require a title and a description.
                let (title, description) = match (&embed.title, &embed.description) {
                    (Some(title), Some(description)) => (title, description),
                    _ => continue,
                };

                // Begin building information!
                let heading = format!("## {}", title);
                let body = description;
                let mut custom_fields: Vec<String> = vec![];

                let friends_chat = embed.fields.iter().find(|field| {
                    field.name == "__Friends Chat__"
                    // Deep Sea Fishing has this error.
                        || field.name == "__Friend's Chat__"
                });

                let invite_url = &embed.url;

                let contact = embed
                    .fields
                    .iter()
                    .find(|field| field.name == "__Contact__");

                let additional_information =
                    embed.footer.as_ref().map(|footer| footer.text.clone());

                let mut content = format!("{}\n{}", heading, body);

                if let Some(friends_chat) = friends_chat {
                    custom_fields.push(format!(
                        "{FRIENDS_CHAT_EMOJI} **Friends Chat**: `{}`",
                        friends_chat.value
                    ));
                }

                if let Some(invite_url) = invite_url {
                    custom_fields.push(format!("{DISCORD_EMOJI} **Discord**: {invite_url}"));
                }

                if !custom_fields.is_empty() {
                    content = format!("{content}\n\n{}", custom_fields.join("\n"));
                }

                if let Some(contact) = contact {
                    let user_ids: Vec<&str> = Regex::new(r"<@!?(\d{17,19})>")
                        .unwrap()
                        .captures_iter(&contact.value)
                        .map(|capture| capture.get(1).unwrap().as_str())
                        .collect();

                    let mut user_mentions = Vec::new();

                    for user_id in user_ids {
                        user_mentions.push(format!("- <@{user_id}>"));
                    }

                    if !user_mentions.is_empty() {
                        content = format!("{content}\n### Contacts\n{}", user_mentions.join("\n"));
                    }
                }

                if let Some(additional_information) = additional_information {
                    content = format!("{content}\n\n-# {additional_information}");
                }

                // Send!
                let message = AFFFILIATES_CHANNEL_ID
                    .send_message(
                        &client,
                        CreateMessage::new()
                            .allowed_mentions(CreateAllowedMentions::new())
                            .content(content),
                    )
                    .await;

                if let Err(error) = message {
                    eprintln!("Error sending message: {:#?}", error);
                }
            }
        }
        Err(error) => {
            eprintln!("Error fetching messages: {:#?}", error);
        }
    }
}
