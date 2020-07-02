use std::env::Args;
use std::net::SocketAddr;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn read_test_data(fpath: &str) -> Vec<u8> {
    let path = Path::new(fpath);
    let display = path.display();

    //Open the path in read-only mode returns `io::Result<File>`
    let mut file = match File::open(&path) {
        Err(why) => panic!("Could not read {}: {}", display, why),
        Ok(file) => file,
    };
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    data
}


pub struct Config {
    pub socket: SocketAddr,
    pub test_data_path: String,
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

        let test_data_path = match args.next() {
            Some(arg) => arg,
            None => return Err("No path to test data given"),
        };
        Ok(Config { socket, test_data_path })
    }
}
