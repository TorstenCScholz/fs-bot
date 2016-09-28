extern crate discord;
extern crate hound;
extern crate chrono;
extern crate rand;

use discord::{Discord, Connection, State};
use discord::model::{Event, ChannelId, ServerId, UserId};
use discord::voice::{AudioSource};

use chrono::{Local};

use rand::{Rng};

use std::collections::{HashSet};
use std::{thread, time, env};
use std::str::{FromStr};

mod command;

use command::Command;

fn send_to_channel(discord: &Discord, server_id: &ServerId, channel_id: &ChannelId, user_id: &UserId, message_postfix: &str) {
	let now = Local::now();

	let member = discord.get_member(*server_id, *user_id).unwrap();
	let message = format!("[{}] **{}**{}", now.format("%d.%m.%Y %H:%M:%S").to_string(), member.user.name, message_postfix);
	let _ = discord.send_message(channel_id, &message, "", false);
}

fn say_hello(discord: &Discord, user_id: &UserId, status_channel_id: &ChannelId, connection: &mut Connection, server_id: &ServerId) {
	let delay_before_hello = time::Duration::from_millis(500);

	thread::sleep(delay_before_hello);

	send_to_channel(discord, server_id, status_channel_id, user_id, " joined.");

	let mut rng = rand::thread_rng();
	// TODO: find number of "helloX.wav" files dynamically
	let index = rng.gen_range(0, 7).to_string(); // 7 = Number of hello files

	play_sound(&format!("{}{}", "hello", index), connection, server_id);
}

fn say_goodbye(discord: &Discord, user_id: &UserId, status_channel_id: &ChannelId, connection: &mut Connection, server_id: &ServerId) {
	send_to_channel(discord, server_id, status_channel_id, user_id, " left.");

	play_sound("goodbye", connection, server_id);
}

fn play_sound(command: &str, connection: &mut Connection, server_id: &ServerId) {
	if let Ok(mut reader) = hound::WavReader::open(command.to_string() + ".wav") {
		println!("Playing file {}.", command.to_string() + ".wav");

		let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
		let source = create_pcm_source(true, samples);

		let voice_handle = connection.voice(Some(*server_id));
		voice_handle.play(source);
	} else {
		println!("Trying to play invalid sound: {}", command);
	}
}

pub fn create_pcm_source (stereo: bool, read: Vec<i16>) -> Box<AudioSource> {
	Box::new(PcmSource(stereo, read, 0))
}

struct PcmSource(bool, Vec<i16>, usize);

impl AudioSource for PcmSource {
	fn is_stereo(&mut self) -> bool { self.0 }
	fn read_frame(&mut self, buffer: &mut [i16]) -> Option<usize> {
		for (_, val) in buffer.iter_mut().enumerate() {
			if self.2 >= self.1.len() {
				return None;
			}

			*val = self.1[self.2];
			self.2 = self.2 + 1;
		}
		Some(buffer.len())
	}
}

fn sync_voice_user_state(has_synced: &mut bool, voice_users: &mut HashSet<UserId>, discord: &Discord, state: &State, server_id: ServerId, voice_channel_id: ChannelId) {
	if !*has_synced {
		for server in state.servers() {
			if server.id == server_id {
				for voice_state in &server.voice_states {
					if voice_state.channel_id.unwrap() == voice_channel_id {
						//let member = discord.get_member(server_id, voice_state.user_id).unwrap();
						//println!("User is in voice channel: {}", member.user.name);
						voice_users.insert(voice_state.user_id);
					}
				}

				*has_synced = true;
			}
		}
	}
}

fn main() {
	let mut voice_users: HashSet<UserId> = HashSet::new();

	// Log in to Discord using a bot token from the environment
	let discord = Discord::from_bot_token(&env::var("FSB_DISCORD_TOKEN").expect("Cannot find bot token.")).expect("login failed");

	// Establish and use a websocket connection
	let (mut connection, ready) = discord.connect().expect("connect failed");
	let mut state = State::new(ready);
	println!("Ready.");

	let server_id = ServerId(u64::from_str(&env::var("FSB_SERVER_ID").expect("Cannot find server id")).expect("Id is not a number"));
	let voice_channel_id = ChannelId(u64::from_str(&env::var("FSB_VOICE_CHANNEL_ID").expect("Cannot find voice channel id")).expect("Id is not a number"));
	let status_channel_id = ChannelId(u64::from_str(&env::var("FSB_STATUS_CHANNEL_ID").expect("Cannot find status channel id")).expect("Id is not a number"));
	let master_permission_id = UserId(u64::from_str(&env::var("FSB_MASTER_PERMISSION_ID").expect("Cannot find master permission id")).expect("Id is not a number"));

	let my_id = UserId(u64::from_str(&env::var("FSB_MY_ID").expect("Cannot find bot id")).expect("Id is not a number"));

	{
		let voice_handle = connection.voice(Some(server_id));
		voice_handle.connect(voice_channel_id);
	}

	let play_callback = |mut connection: &mut Connection, server_id: &ServerId, args: &[&str]| {
		if args.len() < 1 {
			return;
		}

		play_sound(args[0], &mut connection, &server_id);
	};

	let voice_join_callback = |mut connection: &mut Connection, server_id: &ServerId, args: &[&str]| {
		if args.len() < 1 {
			return;
		}

		// TODO: Inject voice_channel_id via Context or something like that
		let voice_channel_id = ChannelId(u64::from_str(&env::var("FSB_VOICE_CHANNEL_ID").expect("Cannot find voice channel id")).expect("Id is not a number"));
		let voice_handle = connection.voice(Some(*server_id));

		if args[0] == "join" {
			voice_handle.connect(voice_channel_id);
		} else if args[0] == "leave" {
			voice_handle.disconnect();
		}
	};

	let mut commands: HashSet<Command> = HashSet::new();
	commands.insert(Command::new_default("play", Box::new(play_callback)));
	commands.insert(Command::new_default("voice", Box::new(voice_join_callback)));

	let mut has_synced = false;

	loop {
		let event = match connection.recv_event() {
			Ok(event) => event,
			Err(err) => {
				println!("[Warning] Receive error: {:?}", err);
				if let discord::Error::WebSocket(..) = err {
					// Handle the websocket connection being dropped
					let (new_connection, ready) = discord.connect().expect("connect failed");
					connection = new_connection;
					state = State::new(ready);
					println!("[Ready] Reconnected successfully.");
				}
				if let discord::Error::Closed(..) = err {
					break
				}

				// TODO: If we left the voice channel, simply rejoin it
				let voice_handle = connection.voice(Some(server_id));
				voice_handle.disconnect();
				voice_handle.connect(voice_channel_id);

				continue
			},
		};
		state.update(&event);

		sync_voice_user_state(&mut has_synced, &mut voice_users, &discord, &state, server_id, voice_channel_id);

		match event {
			Event::MessageCreate(message) => {
				println!("{} says: {}", message.author.name, message.content);
				let user_id = message.author.id;

				if message.content.starts_with("!") && message.content.len() > 1 {
					let content_sans_action = &message.content[1..];
					let split_contents: Vec<&str> = content_sans_action.split(" ").collect();
					let command_name = &split_contents[0];

					let mut parameters: &[&str] = &[];

					if split_contents.len() > 1 {
						parameters = &split_contents[1..];
					}

					let mut has_invoked_cmd = false;
					for command in &commands {
						if command.matches(command_name) {
							command.invoke(&mut connection, &server_id, &user_id, parameters);
							has_invoked_cmd = true;
						}
					}

					if !has_invoked_cmd {
						println!("[Info] Unknown command: {}", command_name);
					}
				}

				if message.content == "!code" {
					let _ = discord.send_message(&message.channel_id, "You can find my internals at https://github.com/TorstenCScholz/fs-bot", "", false);
				} else if message.content == "!quit" {
					if master_permission_id == user_id {
						println!("Quitting.");
						let text = "Bye ".to_string() + &message.author.name + ".";
						let _ = discord.send_message(&message.channel_id, &text, "", false);
						break;
					}
				}
			}
			Event::VoiceStateUpdate(server_id, voice_state) => {
				println!("[Voice update] {:?}", voice_state);

				let user_id = voice_state.user_id;

				if let Some(channel_id) = voice_state.channel_id {
					if channel_id == voice_channel_id {
						if !voice_users.contains(&user_id) {
							// User joined
							voice_users.insert(user_id);

							say_hello(&discord, &user_id, &status_channel_id, &mut connection, &server_id.unwrap());
						}
					} else {
						if voice_users.contains(&user_id) {
							// User in observed voice channel switched voice channel
							if user_id == my_id { // If it was us (maybe we got moved) just rejoin
								let voice_handle = connection.voice(server_id);
								voice_handle.connect(voice_channel_id);
							} else {
								voice_users.remove(&user_id);

								say_goodbye(&discord, &user_id, &status_channel_id, &mut connection, &server_id.unwrap());
							}
						}
					}
				} else {
					// Only say goodbye if the user was prev. known to us (that is he/she was in our observed voice channel)
					if voice_users.contains(&user_id) {
						if user_id != my_id {
							voice_users.remove(&user_id);

							say_goodbye(&discord, &user_id, &status_channel_id, &mut connection, &server_id.unwrap());
						}
					}
				}

				println!("[Users after voice update] {:?}", voice_users);
			}
			_ => {}
		}
	}

	// Log out from the API
	connection.shutdown().expect("closing websocket failed");
}
