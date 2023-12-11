use std::{
    net::UdpSocket,
    thread::sleep,
    time::Duration,
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    println!("starting 1");
    let mut buf = [0u8; 4096];
    loop {
        match UdpSocket::bind("0.0.0.0:53") {
            Ok(socket) => {
                let mut error_number = 0;
                loop {
                    match socket.recv_from(&mut buf) {
                        Ok((n, addr)) => {
                            let msg = &buf[0 .. n];
                            println!("Got message {}", String::from_utf8_lossy(msg));
                            match socket.send_to(msg, addr) {
                                Ok(_) => { },
                                Err(e) => println!("Send to {} error: {:?}", addr.to_string(), e),
                            }
                            error_number = 0;
                        },
                        Err(e) => {
                            error_number += 1;
                            println!("Receive error #{}: {:?}", error_number, e);
                            if error_number >= 10 {
                                break;
                            }

                            // 2 ^ 9 = 512 ms
                            sleep(Duration::from_millis(2u64.pow(error_number as u32)));
                        },
                    }
                }
            },
            Err(e) => println!("UDP bind error: {:?}", e),
        }
        sleep(Duration::from_secs(1));
    }
}
