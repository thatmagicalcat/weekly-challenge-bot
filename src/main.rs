use std::env;

use discord::model::ChannelId;
use discord::model::RoleId;
use discord::model::ServerId;
use discord::model::Event;
use discord::Discord;
use dotenv::dotenv;

mod message_handler;
mod tester;

#[derive(Debug, Clone, Copy)]
pub struct EnvInfo {
    result_channel: ChannelId,
    botcmd_channel: ChannelId,
    hidden_sol_channel: ChannelId,
    submit_channel: ChannelId,
    submitted_role_id: RoleId,
    winner_role_id: RoleId,
    server_id: ServerId,
}

fn main() {
    dotenv().expect("Failed to load env file");

    let token = env::var("DISCORD_TOKEN").expect("token not found");
    let env_info = EnvInfo {
        result_channel: ChannelId(
            env::var("RESULT_CHANNEL")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
        server_id: ServerId(
            env::var("SERVER_ID")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
        submitted_role_id: RoleId(
            env::var("SUBMITTED_ROLE_ID")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
        winner_role_id: RoleId(
            env::var("WINNER_ROLE_ID")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
        submit_channel: ChannelId(
            env::var("SUBMIT_CHANNEL")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
        botcmd_channel: ChannelId(
            env::var("BOTCMD_CHANNEL")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
        hidden_sol_channel: ChannelId(
            env::var("HIDDEN_SOLUTION_CHANNEL")
                .expect("submit channel id not found")
                .parse()
                .unwrap(),
        ),
    };

    let discord = Discord::from_bot_token(&token).expect("Failed to login");
    let (mut connection, ready) = discord.connect().expect("failed to connect");

    println!(
        "Logged in as: {}#{}",
        ready.user.username, ready.user.discriminator
    );

    loop {
        use Event::*;
        match connection.recv_event() {
            Ok(MessageCreate(message)) => {
                message_handler::handle_message(
                    &discord,
                    message,
                    env_info
                );
            }

            Ok(_) => {}

            Err(discord::Error::Closed(code, body)) => {
                println!("Gateway closed on us with code {:?}: {}", code, body);
                break;
            }

            Err(err) => println!("Receive error: {:?}", err),
        }
    }
}
