extern crate vmci;

use vmci::{VSocketStream, VSocketListener};

use std::env;
use std::io::prelude::*;
use std::process::Command;

const PORT: u32 = 15000;

fn client() {
    let mut sock = VSocketStream::connect(PORT).unwrap();

    sock.write(b"ping").unwrap();

    let mut buf = [0; 5];

    sock.read(&mut buf).unwrap();
    println!("recv={}", String::from_utf8_lossy(&buf))
}

fn vm_info(id: i32) {
    let output = Command::new("/sbin/vsish")
                     .arg("-e")
                     .arg("get")
                     .arg(format!("/userworld/cartel/{}/vmmLeader", id))
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e))
                     .stdout;

    let leader = String::from_utf8_lossy(&output);

    println!("leader={}", leader.trim());

    let info = Command::new("/sbin/vsish")
                   .arg("-e")
                   .arg("get")
                   .arg(format!("/vm/{}/vmmGroupInfo", leader.trim()))
                   .output()
                   .unwrap_or_else(|e| panic!("failed to execute process: {}", e))
                   .stdout;

    println!("{}", String::from_utf8_lossy(&info))
}

fn server() {
    let listener = VSocketListener::bind(PORT).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut buf = [0; 5];
                let mut stream = stream;
                let laddr = stream.local_addr().unwrap();
                let raddr = stream.peer_addr().unwrap();
                let vmid = stream.peer_host_vm_id().unwrap();

                stream.read(&mut buf).unwrap();
                println!("recv={}", String::from_utf8_lossy(&buf));
                stream.write(b"pong").unwrap();

                println!("laddr={:?}, raddr={:?}, vmid={}", laddr, raddr, vmid);
                vm_info(vmid);
            }
            Err(e) => println!("accept: {}", e),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 1 {
        client();
    } else {
        server();
    }
}
