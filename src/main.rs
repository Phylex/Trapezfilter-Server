extern crate moessbauer_data;
use moessbauer_data::*;
use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use moessbauer_server::*;
use std::env;
use std::process;
use std::str;

fn handle_client(mut stream: TcpStream) {
    let mut data = [0 as u8; 50];
    while match stream.read(&mut data) {
        Ok(size) => {
            if str::from_utf8(&data[..size]).unwrap().ends_with("end") {
                stream.write(&data[0..size]).unwrap();
                stream.flush().unwrap();
                stream.shutdown(Shutdown::Both).unwrap();
                false
            } else {
                stream.write(&data[0..size]).unwrap();
                true
            }
        },
        Err(_) => {
            println!("An error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    let test_data = read_test_data(&config.test_data_path[..]);
    let peak_cnt = (test_data.len()/12) as usize;
    println!("Read in {} test peaks", peak_cnt);
    
    // the peak data now has to be decoded and turned into the "peak" data structure
    let mut test_peaks: Vec<MeasuredPeak> = Vec::new();
    for i in 0..peak_cnt {
        test_peaks.push(MeasuredPeak::new(&test_data[i*12..(i+1)*12]))
    }


    let listener = TcpListener::bind(config.socket).unwrap();
    println!("Server listening on port {}", config.socket.port());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move || {
                    handle_client(stream)
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
