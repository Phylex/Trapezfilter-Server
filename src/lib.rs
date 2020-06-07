use std::env::Args;
use std::net::SocketAddr;

pub struct Config {
    pub socket: SocketAddr,
}

impl Config {
    pub fn new(mut args: Args) -> Result<Config, &'static str> {
        args.next();

        let socket = match args.next() {
            Some(arg) => {
                match arg.parse() {
                    Ok(socket) => socket,
                    Err(_) => return Err("The Socket Address could not be parsed")
                }},
            None => return Err("No Socket Address was given"),
        };
        Ok(Config { socket })
    }
}
