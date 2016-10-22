# fs-bot
A Discord bot for the Frostshock community on our server.

As I am still learning Rust, the code might (hint: it actually is) be very ugly in some parts (or everywhere). Please don't judge me :(

# Installation
In order to install the bot on an Ubuntu system (i.e. 16.4), you have to
* Install Rust (at least version 1.9) and Cargo (at least version 0.10.0)
* Install libsodium (https://download.libsodium.org/doc/) if necessary
* Install openssl (via `sudo apt-get install openssl openssl-dev`) if necessary
* Install opus if necessary
* Build the bot via `cargo build`
* Set environment variables containing the discord bot token and the ids of the server, voice channel, status channel and bot user itself the bot should run on

## Environment variables
The environment variables names are
* FSB_DISCORD_TOKEN
* FSB_SERVER_ID
* FSB_VOICE_CHANNEL_ID
* FSB_STATUS_CHANNEL_ID
* FSB_MY_ID
* FSB_MASTER_PERMISSION_ID

You can find the ids in Discord when you enable Developer mode and then right clicking everything of interest.
