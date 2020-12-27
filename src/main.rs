use std::net::UdpSocket;
use std::convert::TryFrom;
use logging_allocator::{LoggingAllocator, run_guarded};
use log::info;
use simple_logger::SimpleLogger;
use tftp_rs;

#[global_allocator]
static ALLOC: LoggingAllocator = LoggingAllocator::new();

fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();
    ALLOC.enable_logging();
    run_guarded(|| info!("TFTP-RS Started..."));

    let socket = UdpSocket::bind("localhost:69")?;
    loop {
        let mut buf = [0; 4096];
        let (_, src) = socket.recv_from(&mut buf)?;


        match tftp_rs::PacketType::try_from(&buf[..]) {
            Ok(val) => {
                run_guarded(|| info!("Packet : {:?}", val));
                match val {
                    tftp_rs::PacketType::ReadRequest(rrq) => {
                        std::thread::spawn(move ||{
                            tftp_rs::handle_read_request(src.to_owned(), rrq);
                        });
                    },
                    tftp_rs::PacketType::WriteRequest(wrq) => {
                        // tftp_rs::handle_write_request(src.to_owned(), wrq);
                    },
                    _ => {
                        tftp_rs::send_error(src, "Don't wanna parse");
                    },
                }
            },
            Err(e) => {
                run_guarded(|| info!("{:?}", e));
                tftp_rs::send_error( src, "Invalid Initial Request");
            }
        }
    }
    
    Ok(())
}
