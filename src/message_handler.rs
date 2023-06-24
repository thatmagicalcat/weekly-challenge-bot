use std::fs;
use std::io::Write;
use std::thread;
use std::time::Duration;

use discord::model::{ChannelId, Message, UserId};
use discord::Discord;

use super::tester;
use super::EnvInfo;

const FILE_EXTENSIONS: [&str; 11] = [
    "rs", "cpp", "cc", "c++", "c", "cxx", "lua", "py", "js", "ts", "nim",
];

extern "C" {
    fn system(command: *mut u8) -> i32;
}

pub fn handle_message(discord: &Discord, message: Message, env_info: EnvInfo) {
    let EnvInfo {
        result_channel,
        botcmd_channel,
        hidden_sol_channel,
        submit_channel,
        submitted_role_id,
        winner_role_id,
        server_id,
    } = env_info;

    println!("message received: {}", message.content);

    static mut SUBMISSIONS_OPEN: bool = false;

    let Message {
        channel_id,
        content,
        author,
        ref attachments,
        ..
    } = message;

    if author.bot {
        return;
    }

    if channel_id == submit_channel {
        discord.delete_message(channel_id, message.id).unwrap();
        if unsafe { SUBMISSIONS_OPEN } {
            if attachments.is_empty() {
                let message = discord
                    .send_message(
                        channel_id,
                        &format!(
                            "<@{}> Please attach the solution file with the file extension.",
                            author.id
                        ),
                        "",
                        false,
                    )
                    .expect("Failed to send message");
                thread::sleep(Duration::from_secs(5));
                discord
                    .delete_message(channel_id, message.id)
                    .expect("Failed to delete message");
            } else {
                // reading the directory is faster than looking the role
                if fs::read_dir("./sub")
                    .expect("failed to read directory")
                    .any(|file| {
                        file.as_ref()
                            .expect("Failed to read directory entry")
                            .file_name()
                            .to_str()
                            .unwrap()
                            .split_once('.')
                            .unwrap()
                            .0
                            == author.id.to_string()
                    })
                {
                    let msg = discord
                        .send_message(
                            channel_id,
                            &format!("<@{}> You have already submitted!", author.id),
                            "",
                            false,
                        )
                        .unwrap();

                    thread::sleep(Duration::from_secs(5));
                    discord.delete_message(channel_id, msg.id).unwrap();
                    return;
                }

                let file = message.attachments.first().unwrap();
                let file_extension = dbg!(&file.filename).split('.').last().unwrap().trim();
                let file_url = dbg!(&file.url);

                if !FILE_EXTENSIONS.iter().any(|i| *i == file_extension) {
                    discord.send_message(
                        channel_id,
                        &format!("Invalid file extension: {file_extension}"),
                        "",
                        false,
                    );
                    return;
                }

                let file_contents = reqwest::blocking::get(file_url)
                    .expect("Failed to fetch file contents")
                    .text()
                    .unwrap();

                let solution_length = file_contents.len();

                let mut file_object =
                    fs::File::create(format!("./sub/{}.{file_extension}", author.id))
                        .expect("failed to create file");

                file_object
                    .write_all(file_contents.as_bytes())
                    .expect("failed to write to file");

                discord
                    .send_message_ex(hidden_sol_channel, |m| {
                        m.embed(|embed| {
                            embed
                                .author(|a| {
                                    a.name(&author.name).icon_url(&author.avatar_url().unwrap())
                                })
                                .description(&format!("```{file_extension}\n{file_contents}\n```"))
                                .footer(|footer| {
                                    footer.text(&format!(
                                        "Lang: {file_extension}, Length: {solution_length} chars"
                                    ))
                                })
                        })
                    })
                    .expect("Failed to send message");

                discord
                    .add_member_role(server_id, author.id, submitted_role_id)
                    .expect("failed to assign role");
            }
        } else {
            let msg = discord
                .send_message(
                    channel_id,
                    &format!("<@{}> submittion is not open yet!", author.id),
                    "",
                    false,
                )
                .unwrap();
            thread::sleep(Duration::from_secs(3));
            discord.delete_message(channel_id, msg.id).unwrap();
        }
    } else if channel_id == botcmd_channel && author.id.0 == 873410122616037456 {
        let mut split = content.trim().split(' ');
        match split.next().unwrap() {
            "reset" => {
                for member in discord
                    .get_server_members(server_id)
                    .expect("Failed to fetch server members")
                {
                    if !member.roles.iter().any(|i| i != &winner_role_id) {
                        discord
                            .remove_member_role(server_id, member.user.id, submitted_role_id)
                            .unwrap();
                    }

                    if !member.roles.into_iter().any(|i| i != submitted_role_id) {
                        discord
                            .remove_member_role(server_id, member.user.id, submitted_role_id)
                            .unwrap();
                    }
                }

                for i in fs::read_dir("./sub").unwrap() {
                    fs::remove_file(i.unwrap().path()).expect("failed to delete solutions");
                }

                discord.send_message(channel_id, "ok", "", false).unwrap();
            }

            "close" => {
                unsafe {
                    SUBMISSIONS_OPEN = false;
                }
                discord.send_message(channel_id, "ok", "", false).unwrap();
            }

            "open" => {
                unsafe {
                    SUBMISSIONS_OPEN = true;
                }
                discord.send_message(channel_id, "ok", "", false).unwrap();
            }

            "list" => {
                let mut msg = String::new();
                for i in fs::read_dir("./sub").unwrap() {
                    if i.as_ref().unwrap().file_type().unwrap().is_file() {
                        msg += &format!("{}\n", i.unwrap().file_name().to_str().unwrap());
                    }
                }

                discord
                    .send_message(
                        channel_id,
                        if msg.is_empty() {
                            "No solutions yet!"
                        } else {
                            &msg
                        },
                        "",
                        false,
                    )
                    .unwrap();
            }

            "clear" => {
                for i in fs::read_dir("./sub").unwrap() {
                    fs::remove_file(i.unwrap().path()).expect("failed to delete solutions");
                }

                discord
                    .send_message(channel_id, "deleted all solutions", "", false)
                    .unwrap();
            }

            "remove" => {
                if let Some(id) = split.next() {
                    if let Ok(id) = id.parse::<u64>() {
                        if let Some(file) = fs::read_dir("./sub")
                            .expect("failed to read directory")
                            .find(|file| {
                                file.as_ref()
                                    .expect("Failed to read directory entry")
                                    .file_name()
                                    .to_str()
                                    .unwrap()
                                    .split_once('.')
                                    .unwrap()
                                    .0
                                    == id.to_string()
                            })
                        {
                            fs::remove_file(file.expect("Failed to read file").path())
                                .expect("failed to remove file");
                            discord
                                .send_message(channel_id, "Solution removed!", "", false)
                                .unwrap();
                        } else {
                            discord
                                .send_message(channel_id, "User has not submitted yet!", "", false)
                                .unwrap();
                        }
                    } else {
                        discord
                            .send_message(channel_id, "Invalid id", "", false)
                            .unwrap();
                    }
                } else {
                    discord
                        .send_message(channel_id, "Please input id", "", false)
                        .unwrap();
                }
            }

            "test" => {
                if attachments.is_empty() {
                    discord
                        .send_message(channel_id, "Please give the tester file", "", false)
                        .unwrap();
                } else {
                    let file = attachments.first().unwrap();
                    let file_url = dbg!(&file.url);

                    let tester_file = reqwest::blocking::get(file_url)
                        .expect("Failed to fetch tester file contents")
                        .text()
                        .unwrap();

                    let mut result: Vec<(&str, u64, usize, Option<f64>)> = Vec::new();

                    fs::read_dir("./sub")
                        .unwrap()
                        .filter(|i| i.as_ref().unwrap().metadata().unwrap().is_file())
                        .for_each(|i| {
                            let file_name = i.as_ref().unwrap().file_name();
                            let file_name = file_name.to_str().unwrap();
                            let solution_length =
                                fs::read_to_string(i.unwrap().path()).unwrap().len();

                            let (user_id, file_extension) = file_name.split_once('.').unwrap();

                            result.push((
                                get_lang_emoji_name(file_extension),
                                user_id.parse().unwrap(),
                                solution_length,
                                tester::test(
                                    &match file_extension {
                                        "rs" => {
                                            exec(&format!("rustc ./sub/{file_name} -O -o main"));
                                            "./main".to_string()
                                        }

                                        "cpp" | "cc" | "cxx" | "c++" => {
                                            exec(&format!(
                                                "g++ -std=c++20 ./sub/{file_name} -O2 -o main"
                                            ));
                                            "./main".to_string()
                                        }

                                        "c" => {
                                            exec(&format!(
                                                "gcc -std=c17 ./sub/{file_name} -O2 -o main"
                                            ));
                                            "./main".to_string()
                                        }

                                        "js" | "ts" => format!("node ./sub/{file_name}"),
                                        "py" => format!("python3 ./sub/{file_name}"),
                                        "lua" => format!("lua ./sub/{file_name}"),
                                        "nim" => format!("nim c -r ./sub/{file_name}"),

                                        _ => unreachable!("what file extension is this???"),
                                    },
                                    &tester_file,
                                ),
                            ));
                        });

                    let passed = result.iter().filter(|i| i.3.is_some()).collect::<Vec<_>>();

                    for (_, id, _, _) in passed.iter() {
                        discord
                            .add_member_role(server_id, UserId(*id), winner_role_id)
                            .unwrap();
                    }

                    let mut fastest_solutions = passed
                        .iter()
                        .map(|i| (i.0, i.1, i.3.unwrap()))
                        .collect::<Vec<_>>();
                    fastest_solutions.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());

                    let mut shortest_solutions =
                        passed.iter().map(|i| (i.0, i.1, i.2)).collect::<Vec<_>>();
                    shortest_solutions.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());

                    let failed_solutions = result
                        .into_iter()
                        .filter(|i| i.3.is_none())
                        .map(|i| i.1)
                        .collect::<Vec<_>>();

                    if !shortest_solutions.is_empty() {
                        let mut message = String::from("# Challenge winners - Code length\n");

                        for (i, (emoji, user_id, length)) in
                            shortest_solutions.into_iter().enumerate()
                        {
                            message +=
                                &format!("{emoji} {}. {length} chars | <@{}>\n", i + 1, user_id);
                        }

                        discord
                            .send_message(result_channel, &message, "", false)
                            .unwrap();
                    }

                    if !fastest_solutions.is_empty() {
                        let mut message = String::from("# Challenge winners - Fastest\n");

                        for (i, (emoji, user_id, time)) in fastest_solutions.into_iter().enumerate()
                        {
                            message += &format!("{emoji} {}. {time}s | <@{}>\n", i + 1, user_id);
                        }

                        discord
                            .send_message(result_channel, &message, "", false)
                            .unwrap();
                    }

                    if !failed_solutions.is_empty() {
                        let mut message = String::from("# Failed solutions\n");

                        for (i, user_id) in failed_solutions.into_iter().enumerate() {
                            message += &format!("{}. <@{}>\n", i + 1, user_id);
                        }

                        discord
                            .send_message(result_channel, &message, "", false)
                            .unwrap();
                    }
                }
            }

            _ => {}
        }
    }

    if content.starts_with("!ping") {
        println!("sending");
        discord
            .send_message(channel_id, "pong!", "", false)
            .unwrap();
    }
}

fn exec(cmd: &str) {
    let mut cmd = String::from(cmd);
    unsafe { system(cmd.as_mut_ptr()) };
}

fn get_lang_emoji_name(extension: &str) -> &'static str {
    match extension {
        "rs" => "<:rust:1121679837657038928>",
        "cpp" | "cc" | "cxx" | "c++" => "<:cpp:1121679809005748305>",
        "c" => "<:clang:1121679805960683530>",
        "js" | "ts" => "<:js:1121679814278004776>",
        "py" => "<:python:1121679833181732985>",
        "lua" => "<:lua:1121679825694892162>",
        "nim" => "<:nim:1121679830371536968>",
        _ => unreachable!("what file extension is this???"),
    }
}
