mod utility;
use dotenvy::dotenv;
use regex::Regex;
use serenity::{
    all::{ChannelId, CreateAllowedMentions, CreateMessage},
    http::Http,
};
use std::env;
use utility::constants::{
    AFFFILIATES_CHANNEL_ID, AFFILIATE_ROLE_ID, DISCORD_EMOJI, FRIENDS_CHAT_EMOJI,
    RAW_AFFILIATES_CHANNEL_ID,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    dotenv().ok();
    let discord_token = env::var("DISCORD_TOKEN").expect("Error retrieving DISCORD_TOKEN");
    let client = Http::new(&discord_token);

    // Fetch the messages in the channel.
    let messages = client
        .get_messages(ChannelId::new(RAW_AFFILIATES_CHANNEL_ID), None, Some(100))
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

                let additional_information = match &embed.footer {
                    Some(footer) => Some(footer.text.clone()),
                    None => None,
                };

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

                if custom_fields.len() > 0 {
                    content = format!("{content}\n\n{}", custom_fields.join("\n"));
                }

                if let Some(contact) = contact {
                    let users_regular_expression = Regex::new(r"(<@!?\d+>)");

                    match users_regular_expression {
                        Ok(users_regular_expression) => {
                            let users: Vec<String> = users_regular_expression
                                .captures_iter(&contact.value)
                                .map(|capture| format!("- {}", capture.get(0).unwrap().as_str()))
                                .collect();

                            if users.len() > 0 {
                                content = format!(
                                    "{content}\n\n__<@&{AFFILIATE_ROLE_ID}>__\n{}",
                                    users.join("\n")
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Error performing the regular expression for affiliates: {:#?}",
                                error
                            );
                        }
                    }
                }

                if let Some(additional_information) = additional_information {
                    content = format!("{content}\n\n-# {additional_information}");
                }

                // Send!
                let message = client
                    .send_message(
                        ChannelId::new(AFFFILIATES_CHANNEL_ID),
                        vec![],
                        &CreateMessage::new()
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
