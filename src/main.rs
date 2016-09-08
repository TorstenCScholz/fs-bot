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

		let voice_handle = connection.voice(*server_id);
		voice_handle.play(source);
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
				// TODO: Why do I have to clone here? (otherwise: cannot move out of borrowed content [E0507])
				for voice_state in server.clone().voice_states {
					if voice_state.channel_id.unwrap() == voice_channel_id {
						let member = discord.get_member(server_id, voice_state.user_id).unwrap();
						println!("User is in voice channel: {}", member.user.name);

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

	let my_id = UserId(u64::from_str(&env::var("FSB_MY_ID").expect("Cannot find bot id")).expect("Id is not a number"));

	{
		let voice_handle = connection.voice(server_id);
		voice_handle.connect(voice_channel_id);
	}

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
				continue
			},
		};
		state.update(&event);

		sync_voice_user_state(&mut has_synced, &mut voice_users, &discord, &state, server_id, voice_channel_id);

		match event {
			Event::MessageCreate(message) => {
				println!("{} says: {}", message.author.name, message.content);
				if message.content == "!test" {
					let _ = discord.send_message(&message.channel_id, "This is a reply to the test.", "", false);
				} else if message.content == "!code" {
					let _ = discord.send_message(&message.channel_id, "You can find my internals at https://github.com/TorstenCScholz/fs-bot", "", false);
				} else if message.content == "!quit" {
					println!("Quitting.");
					let text = "Bye ".to_string() + &message.author.name + ".";
					let _ = discord.send_message(&message.channel_id, &text, "", false);
					break;
				} else if message.content.starts_with("!") {
					let command_name: &str = &message.content[1..];
					play_sound(command_name, &mut connection, &server_id);
				}
			}
			Event::VoiceStateUpdate(server_id, voice_state) => {
				println!("[Voice update] {:?}", voice_state);

				let user_id = voice_state.user_id;

				if my_id == user_id {
					continue;
				}

				if let Some(channel_id) = voice_state.channel_id {
					if channel_id == voice_channel_id {
						if !voice_users.contains(&user_id) {
							// User joined
							voice_users.insert(user_id);

							say_hello(&discord, &user_id, &status_channel_id, &mut connection, &server_id);
						}
					} else {
						if voice_users.contains(&user_id) {
							// User in observed voice channel left
							voice_users.remove(&user_id);

							say_goodbye(&discord, &user_id, &status_channel_id, &mut connection, &server_id);
						}
					}
				} else {
					// Only say goodbye if the user was prev. known to us (that is he/she was in our observed voice channel)
					if voice_users.contains(&user_id) {
						voice_users.remove(&user_id);

						say_goodbye(&discord, &user_id, &status_channel_id, &mut connection, &server_id);
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
