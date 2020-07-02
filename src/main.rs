extern crate moessbauer_data;
extern crate crossbeam;
use crossbeam::{channel, Sender, Receiver};
use moessbauer_data::*;
use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write, ErrorKind};
use moessbauer_server::*;
use std::fs::File;
use std::path::Path;
use std::env;
use std::process;
use std::str;
use std::time::Duration;

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    // set up the channel for the data_read thread to talk to the network threads
    let (data_thread_tx, data_network_rx) = channel::unbounded();

    let data_file = File::open(Path::new(&config.data_file_path)).unwrap();

    // the peak data now has to be decoded and turned into the "peak" data structure
    let data_thread_handle = thread::spawn(move || {
        read_data(data_thread_tx, data_file)
    });

    // open the network socket and listen for connections
    let listener = TcpListener::bind(config.socket).unwrap();
    println!("Server listening on port {}", config.socket.port());

    for stream in listener.incoming() {
        let stream_data_rx = data_network_rx.clone();
        let stream = stream.unwrap();
        println!("New connection: {}", stream.peer_addr().unwrap());
        thread::spawn(move || {
            handle_client(stream, stream_data_rx);
        });
    }
    let _ = data_thread_handle.join().unwrap();
}

fn handle_client(mut stream: TcpStream, data_stream: Receiver<MeasuredPeak>) {
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
            println!("an error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}

fn read_data(data_tx: Sender<MeasuredPeak>, mut data_file: File) -> Result<(), std::io::Error> {
    let mut raw_data = [0 as u8; 12];
    let metadata = data_file.metadata()?;
    if metadata.len() % 12 == 0 {
        let peak_cnt = metadata.len()/12;
        for _ in 0..peak_cnt {
            data_file.read_exact(&mut raw_data[..])?;
            let peak = MeasuredPeak::new(&raw_data);
            data_tx.send(peak).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}
