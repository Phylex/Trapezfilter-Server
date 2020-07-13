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
use std::time::Duration;
use std::ptr;
use std::mem;

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    // set up the channel for the data_read thread to talk to the network threads
    let (data_thread_tx, data_network_rx) = channel::unbounded();
    let (cmd_tx, cmd_rx) = channel::unbounded();

    let data_file = File::open(Path::new(&config.data_file_path)).unwrap();

    // the peak data now has to be decoded and turned into the "peak" data structure
    let data_thread_handle = thread::spawn(move || {
        read_data(cmd_rx, data_thread_tx, data_file)
    });

    // open the network socket and listen for connections
    let listener = TcpListener::bind(config.socket).unwrap();
    println!("Server listening on port {}", config.socket.port());

    for stream in listener.incoming() {
        let stream_data_rx = data_network_rx.clone();
        let stream_cmd_tx = cmd_tx.clone();
        let stream = stream.unwrap();
        println!("New connection: {}", stream.peer_addr().unwrap());
        thread::spawn(move || {
            handle_client(stream, stream_cmd_tx, stream_data_rx);
        });
    }
    let _ = data_thread_handle.join().unwrap();
}

fn shift_message_out_of_buffer<'a>(mut buffer: &'a mut Box<[u8]>, mut tmp_buffer: &'a mut Box<[u8]>, msg_size: usize, read_size: usize) {
    unsafe {
        let src_ptr = buffer.as_ptr().offset(msg_size as isize);
        let dst_ptr = tmp_buffer.as_mut_ptr();
        ptr::copy_nonoverlapping(src_ptr, dst_ptr, read_size-msg_size);
    }
    mem::swap(&mut buffer, &mut tmp_buffer);
}

fn read_from_client(stream: &mut TcpStream) -> std::io::Result<Message> {
    let mut buffer: Box<[u8]> = Box::new([0 as u8; 1048576]);
    let mut tmp_buffer: Box<[u8]> = Box::new([0 as u8; 1048576]);
    let mut new_read_idx = 0;
    loop {
        let nbytes = stream.read(&mut buffer[new_read_idx..])?;
        let msg_candidate = &buffer[..nbytes];
        match Message::deserialize(&msg_candidate) {
            Ok((desermsg, size)) => {
                if size < nbytes {
                    shift_message_out_of_buffer(&mut buffer, &mut tmp_buffer, size, nbytes) 
                }
                return Ok(desermsg);
            },
            Err(e) => {
                stream.read_exact(&mut buffer[nbytes..nbytes+bytes_needed]);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, cmd_tx: Sender<Status>, data_stream: Receiver<MeasuredPeak>) {
    let mut data = [0 as u8; 50];
    stream.set_nonblocking(true).expect("setting stream to nonblocking failed");
    loop {
        match stream.read(&mut data) {
            Ok(size) => {
                let (deser_msg, msg_size) = Message::deserialize(&data).expect("help");
                if size != msg_size { panic!("wrong sizes"); }
                match deser_msg {
                    Message::Status(stat) => {
                        match stat {
                            Status::Start => cmd_tx.send(Status::Start).unwrap(),
                            Status::Stop => cmd_tx.send(Status::Stop).unwrap(),
                        }
                    },
                    Message::Config(_) => panic!("stillnotimplemented"),
                    Message::Data(_) => panic!("thisMakesNoSense"),
                }
                stream.write(&data[0..size]).unwrap();
                stream.flush().unwrap();
                stream.shutdown(Shutdown::Both).unwrap();
                stream.write(&data[0..size]).unwrap();
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                match data_stream.recv() {
                    Ok(data) => {}
                }


            },
            Err(_) => {
                println!("an error occurred, terminating connection with {}", stream.peer_addr().unwrap());
                stream.shutdown(Shutdown::Both).unwrap();
            }
        }
    }
}

fn read_data(cmd_rx: Receiver<Status>, data_tx: Sender<MeasuredPeak>, mut data_file: File) -> Result<(), std::io::Error> {
    let mut raw_data = [0 as u8; 12];
    let metadata = data_file.metadata()?;
    if metadata.len() % 12 == 0 {
        let peak_cnt = metadata.len()/12;
        for _ in 0..peak_cnt {
            match cmd_rx.recv() {
                
            }
            data_file.read_exact(&mut raw_data[..])?;
            let peak = MeasuredPeak::new(&raw_data);
            data_tx.send(peak).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}
