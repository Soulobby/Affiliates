mod structures;
mod utility;
use dotenvy::dotenv;
use futures::future;
use regex::Regex;
use serenity::{
    all::{CreateAllowedMentions, CreateMessage, GetMessages, Mentionable, UserId},
    http::Http,
};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashSet, env};
use structures::affiliate::Affiliate;
use utility::constants::{
    AFFFILIATES_CHANNEL_ID, AFFILIATE_ROLE_ID, DISCORD_EMOJI, FRIENDS_CHAT_EMOJI, GUILD_ID,
    RAW_AFFILIATES_CHANNEL_ID,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenv().ok();
    let discord_token = env::var("DISCORD_TOKEN").expect("Error retrieving DISCORD_TOKEN.");
    let database_url = env::var("DATABASE_URL").expect("Error retrieving DATABASE_URL.");
    let client = Http::new(&discord_token);

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Error connecting to database.");

    // Fetch the messages in the channel.
    let messages = RAW_AFFILIATES_CHANNEL_ID
        .messages(&client, GetMessages::new().limit(100))
        .await;

    match messages {
        Ok(mut messages) => {
            messages.reverse();

            // Truncate the table, returning the user ids.
            let existing_affiliates: Vec<Affiliate> =
                sqlx::query_as("delete from affiliates returning *;")
                    .fetch_all(&pool)
                    .await
                    .expect("Error truncating table.");

            let existing_affiliate_user_ids = existing_affiliates
                .iter()
                .map(|affiliate| affiliate.user_id as u64)
                .collect::<HashSet<_>>();

            let mut affiliate_user_ids = HashSet::new();

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
                        affiliate_user_ids.insert(user_id.parse::<u64>().unwrap());
                        user_mentions.push(format!("- <@{user_id}>"));
                    }

                    if !user_mentions.is_empty() {
                        content = format!(
                            "{content}\n\n__{}__\n{}",
                            AFFILIATE_ROLE_ID.mention(),
                            user_mentions.join("\n")
                        );
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

            // Update the table.
            sqlx::query("insert into affiliates (user_id) SELECT unnest($1)")
                .bind(
                    affiliate_user_ids
                        .iter()
                        .map(|id| *id as i64)
                        .collect::<Vec<_>>(),
                )
                .execute(&pool)
                .await
                .expect("Error updating table.");

            // Remove the role from existing affiliates that are no longer affiliates.
            let members = future::join_all(
                existing_affiliate_user_ids
                    .difference(&affiliate_user_ids)
                    .map(|affiliate| async {
                        client
                            .remove_member_role(
                                GUILD_ID,
                                UserId::from(*affiliate),
                                AFFILIATE_ROLE_ID,
                                None,
                            )
                            .await
                    }),
            )
            .await;

            for member in members {
                if let Err(error) = member {
                    tracing::error!("Error removing role. {error:#?}");
                }
            }

            // Add the role to affiliates that were not already affiliates.
            let members = future::join_all(
                affiliate_user_ids
                    .difference(&existing_affiliate_user_ids)
                    .map(|affiliate| async {
                        client
                            .add_member_role(
                                GUILD_ID,
                                UserId::from(*affiliate),
                                AFFILIATE_ROLE_ID,
                                None,
                            )
                            .await
                    }),
            )
            .await;

            for member in members {
                if let Err(error) = member {
                    tracing::error!("Error adding role. {error:#?}");
                }
            }
        }
        Err(error) => {
            eprintln!("Error fetching messages: {:#?}", error);
        }
    }
}
