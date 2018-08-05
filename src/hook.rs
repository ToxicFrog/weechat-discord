use ffi::{self, *};
use std::ptr;
use std::thread;
use {buffers, discord, discord::DISCORD, plugin_print};

use serenity::model::id::ChannelId;

// *DO NOT* touch this outside of init/end
static mut MAIN_COMMAND_HOOK: *mut HookCommand = ptr::null_mut();
static mut BUFFER_SWITCH_CB: *mut SignalHook = ptr::null_mut();

pub fn init() -> Option<()> {
    let hook = ffi::hook_command(
        weechat_cmd::COMMAND,
        weechat_cmd::DESCRIPTION,
        weechat_cmd::ARGS,
        weechat_cmd::ARGDESC,
        weechat_cmd::COMPLETIONS,
        move |buffer, input| run_command(&buffer, input),
    )?;

    let buffer_switch_hook = ffi::hook_signal("buffer_switch", handle_buffer_switch)?;

    unsafe {
        MAIN_COMMAND_HOOK = Box::into_raw(Box::new(hook));
        BUFFER_SWITCH_CB = Box::into_raw(Box::new(buffer_switch_hook));
    };
    Some(())
}

pub fn destroy() {
    unsafe {
        let _ = Box::from_raw(MAIN_COMMAND_HOOK);
        MAIN_COMMAND_HOOK = ptr::null_mut();
        let _ = Box::from_raw(BUFFER_SWITCH_CB);
        BUFFER_SWITCH_CB = ptr::null_mut();
    };
}

fn handle_buffer_switch(data: SignalHookData) {
    match data {
        SignalHookData::Pointer(buffer) => {
            thread::spawn(move || {
                buffers::load_history(&buffer);
                buffers::load_nicks(&buffer);
            });
        }
        _ => {}
    }
}

// TODO: Transform irc/weechat style to discord style
pub fn buffer_input(buffer: Buffer, message: &str) {
    let channel = buffer
        .get("localvar_channelid")
        .and_then(|id| id.parse().ok())
        .map(|id| ChannelId(id));

    let message = ffi::remove_color(message);

    if let Some(channel) = channel {
        channel
            .say(message)
            .expect(&format!("Unable to send message to {}", channel.0));
    }
}

fn run_command(_buffer: &Buffer, command: &str) {
    // TODO: Add rename command
    match command {
        "" => plugin_print("see /help discord for more information"),
        "connect" => {
            match ffi::get_option("token") {
                Some(t) => {
                    if DISCORD.lock().is_none() {
                        discord::init(&t);
                    }
                }
                None => {
                    plugin_print("Error: plugins.var.weecord.token unset. Run:");
                    plugin_print("/discord token 123456789ABCDEF");
                    return;
                }
            };
        }
        "disconnect" => {
            let mut discord = DISCORD.lock();
            if discord.is_some() {
                if let Some(discord) = discord.take() {
                    discord.shutdown();
                };
            }
            plugin_print("Disconnected");
        }
        _ if command.starts_with("token ") => {
            let token = &command["token ".len()..];
            user_set_option("token", token.trim_matches('"'));
            plugin_print("Set Discord token");
        }
        "autostart" => {
            set_option("autostart", "true");
            plugin_print("Discord will now load on startup");
        }
        "noautostart" => {
            set_option("autostart", "false");
            plugin_print("Discord will not load on startup");
        }
        _ => {
            plugin_print("unknown command");
        }
    };
}

fn user_set_option(name: &str, value: &str) {
    plugin_print(&ffi::set_option(name, value));
}

mod weechat_cmd {
    pub const COMMAND: &'static str = "discord";
    pub const DESCRIPTION: &'static str = "\
Discord from the comfort of your favorite command-line IRC client!
Source code available at https://github.com/Noskcaj19/weechat-discord
Originally by https://github.com/khyperia/weechat-discord
Options used:
plugins.var.weecord.token = <discord_token>
plugins.var.weecord.rename.<id> = <string>
plugins.var.weecord.autostart = <bool>
";
    pub const ARGS: &'static str = "\
                     connect
                     disconnect
                     autostart
                     noautostart
                     token <token>
                     query <user>";
    pub const ARGDESC: &'static str = "\
connect: sign in to discord and open chat buffers
disconnect: sign out of Discord
autostart: automatically sign into discord on start
noautostart: disable autostart
token: set Discord login token
query: open PM buffer with user
Example:
  /discord token 123456789ABCDEF
  /discord connect
  /discord autostart
  /discord query khyperia
  /discord disconnect
";
    pub const COMPLETIONS: &'static str =
        "\
         connect || disconnect || token || autostart || noautostart || query";
}