use std::{
    convert::TryFrom,
    convert::TryInto,
    ffi::CString,
    fs::File,
    io::prelude::*,
    net::{SocketAddr, UdpSocket},
    path::Path,
    unreachable,
};

use log::info;
use logging_allocator::run_guarded;

const BUFFER_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy)]
pub enum ParsingError {
    NotEnoughData,
    InvalidOpcode,
    InvalidErrorMessage,
    SocketError,
    InvalidFilename,
    InvalidMode,
    FileReadError,
}

#[derive(Debug, Clone)]
pub enum PacketType {
    ReadRequest(ReadRequestPacket),
    WriteRequest(WriteRequestPacket),
    Data(DataPacket),
    Acknowledgment(AckPacket),
    TFTPError(ErrorPacket),
}

impl TryFrom<&[u8]> for PacketType {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let opcode = OpCode::try_from(&input[0..2])?;

        match opcode {
            OpCode::ReadRequest => {
                run_guarded(|| info!("Opcode : {:?}", opcode));
                return Ok(Self::ReadRequest(ReadRequestPacket::try_from(&input[2..])?));
            }
            OpCode::WriteRequest => {
                run_guarded(|| info!("Opcode : {:?}", opcode));
                return Ok(Self::WriteRequest(WriteRequestPacket::try_from(
                    &input[2..],
                )?));
            }
            OpCode::Data => {
                run_guarded(|| info!("Opcode : {:?}", opcode));
                return Ok(Self::Data(DataPacket::try_from(&input[2..])?));
            }
            OpCode::Acknowledgment => {
                run_guarded(|| info!("Opcode : {:?}", opcode));
                return Ok(Self::Acknowledgment(AckPacket::try_from(&input[2..])?));
            }
            OpCode::TFTPError => {
                run_guarded(|| info!("Opcode : {:?}", opcode));
                return Ok(Self::TFTPError(ErrorPacket::try_from(&input[2..])?));
            }
        }

        Err(ParsingError::NotEnoughData)
    }
}

#[derive(Debug, Clone)]
pub struct ReadRequestPacket {
    opcode: OpCode,
    filename: CString,
    mode: CString,
}

impl TryFrom<&[u8]> for ReadRequestPacket {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let opcode = OpCode::ReadRequest;

        let mut splitter = input.splitn(3, |x| *x == 0);

        let end_filename = splitter
            .next()
            .ok_or_else(|| ParsingError::NotEnoughData)?
            .len();
        let filename = CString::new(&input[..end_filename]).expect("Error creating CString");

        let end_mode = splitter
            .next()
            .ok_or_else(|| ParsingError::NotEnoughData)?
            .len();
        let mode = CString::new(&input[end_filename + 1..end_filename + end_mode])
            .expect("Error creating CString");

        Ok(Self {
            opcode,
            filename,
            mode,
        })
    }
}

impl ReadRequestPacket {
    fn serialize(&self) -> (usize, [u8; BUFFER_SIZE]) {
        let mut pkt = [0; BUFFER_SIZE];
        let mut length = 0;

        let opcode = (self.opcode as u16).to_be_bytes();
        pkt[0..2].copy_from_slice(&opcode);
        length += 2;

        let filename = self.filename.as_bytes_with_nul();
        pkt[2..2 + filename.len()].copy_from_slice(&filename);
        length += filename.len();

        let mode = self.mode.as_bytes_with_nul();
        pkt[2 + filename.len()..2 + filename.len() + mode.len()].copy_from_slice(mode);
        length += mode.len();

        (length, pkt)
    }
}

#[derive(Debug, Clone)]
pub struct WriteRequestPacket {
    opcode: OpCode,
    filename: CString,
    mode: CString,
}

impl TryFrom<&[u8]> for WriteRequestPacket {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let opcode = OpCode::WriteRequest;

        let mut splitter = input.splitn(3, |x| *x == 0);

        let end_filename = splitter
            .next()
            .ok_or_else(|| ParsingError::NotEnoughData)?
            .len();
        let filename = CString::new(&input[..end_filename]).expect("Error creating CString");

        let end_mode = splitter
            .next()
            .ok_or_else(|| ParsingError::NotEnoughData)?
            .len();
        let mode =
            CString::new(&input[end_filename + 1..end_mode]).expect("Error creating CString");

        Ok(Self {
            opcode,
            filename,
            mode,
        })
    }
}

impl WriteRequestPacket {
    fn serialize(&self) -> (usize, [u8; BUFFER_SIZE]) {
        let mut pkt = [0; BUFFER_SIZE];
        let mut length = 0;

        let opcode = (self.opcode as u16).to_be_bytes();
        pkt[0..2].copy_from_slice(&opcode);
        length += 2;

        let filename = self.filename.as_bytes_with_nul();
        pkt[2..2 + filename.len()].copy_from_slice(&filename);
        length += filename.len();

        let mode = self.mode.as_bytes_with_nul();
        pkt[2 + filename.len()..2 + filename.len() + mode.len()].copy_from_slice(mode);
        length += mode.len();

        (length, pkt)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DataPacket {
    opcode: OpCode,
    block_number: u16,
    data: [u8; 512],
    data_length: usize,
}

impl TryFrom<&[u8]> for DataPacket {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let opcode = OpCode::Data;

        let block_number =
            u16::from_be_bytes(input.try_into().map_err(|_| ParsingError::NotEnoughData)?);

        let data = input[2..]
            .try_into()
            .map_err(|_| ParsingError::NotEnoughData)?;

        let data_length = input[2..].len();

        Ok(Self {
            opcode,
            block_number,
            data,
            data_length,
        })
    }
}

impl DataPacket {
    fn serialize(&self) -> (usize, [u8; BUFFER_SIZE]) {
        let mut pkt = [0; BUFFER_SIZE];
        let mut length = 0;

        let opcode = (self.opcode as u16).to_be_bytes();
        pkt[0..2].copy_from_slice(&opcode);
        length += 2;

        let block_number = self.block_number.to_be_bytes();
        pkt[2..4].copy_from_slice(&block_number);
        length += 2;

        pkt[4..self.data_length + 4].copy_from_slice(&self.data[..self.data_length]);
        length += self.data_length;

        (length, pkt)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AckPacket {
    opcode: OpCode,
    block_number: u16,
}

impl TryFrom<&[u8]> for AckPacket {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let opcode = OpCode::Acknowledgment;

        let block_number = u16::from_be_bytes(
            input[..2]
                .try_into()
                .map_err(|_| ParsingError::NotEnoughData)?,
        );

        Ok(Self {
            opcode,
            block_number,
        })
    }
}

impl AckPacket {
    fn serialize(&self) -> (usize, [u8; BUFFER_SIZE]) {
        let mut pkt = [0; BUFFER_SIZE];
        let mut length = 0;

        let opcode = (self.opcode as u16).to_be_bytes();
        pkt[0..2].copy_from_slice(&opcode);
        length += 2;

        let block_number = self.block_number.to_be_bytes();
        pkt[2..4].copy_from_slice(&block_number);
        length += 2;

        (length, pkt)
    }
}

#[derive(Debug, Clone)]
pub struct ErrorPacket {
    opcode: OpCode,
    error_code: ErrorCode,
    error_msg: CString,
}

impl TryFrom<&[u8]> for ErrorPacket {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let opcode = OpCode::TFTPError;

        let error_code = ErrorCode::try_from(&input[0..2])?;

        let mut splitter = input[2..].splitn(2, |x| *x == 0);
        let end_error_msg = splitter
            .next()
            .ok_or_else(|| ParsingError::NotEnoughData)?
            .len();

        let error_msg = CString::new(&input[2..end_error_msg]).expect("Error creating CString");

        Ok(Self {
            opcode,
            error_code,
            error_msg,
        })
    }
}

impl ErrorPacket {
    fn serialize(&self) -> (usize, [u8; BUFFER_SIZE]) {
        let mut pkt = [0; BUFFER_SIZE];
        let mut length = 0;

        let opcode = (self.opcode as u16).to_be_bytes();
        pkt[0..2].copy_from_slice(&opcode);
        length += 2;

        let err_code = (self.error_code as u16).to_be_bytes();
        pkt[2..4].copy_from_slice(&err_code);
        length += 2;

        let cstr = self.error_msg.as_bytes_with_nul();
        pkt[4..cstr.len() + 4].copy_from_slice(cstr);
        length += cstr.len();

        (length, pkt)
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
enum OpCode {
    ReadRequest = 1,
    WriteRequest,
    Data,
    Acknowledgment,
    TFTPError,
}

impl TryFrom<&[u8]> for OpCode {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let op = u16::from_be_bytes(input.try_into().map_err(|_| ParsingError::NotEnoughData)?);
        match op {
            1 => Ok(Self::ReadRequest),
            2 => Ok(Self::WriteRequest),
            3 => Ok(Self::Data),
            4 => Ok(Self::Acknowledgment),
            5 => Ok(Self::TFTPError),
            _ => Err(ParsingError::InvalidOpcode),
        }
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
enum ErrorCode {
    NotDefined = 0,
    FileNotFound,
    AccessViolation,
    DiskFull,
    IllegalTFTPOperation,
    UnknownTransferID,
    FileAlreadyExists,
    NoSuchUser,
}

impl TryFrom<&[u8]> for ErrorCode {
    type Error = ParsingError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        let op = u16::from_be_bytes(input.try_into().map_err(|_| ParsingError::NotEnoughData)?);
        match op {
            0 => Ok(Self::NotDefined),
            1 => Ok(Self::FileNotFound),
            2 => Ok(Self::AccessViolation),
            3 => Ok(Self::DiskFull),
            4 => Ok(Self::IllegalTFTPOperation),
            5 => Ok(Self::UnknownTransferID),
            6 => Ok(Self::FileAlreadyExists),
            7 => Ok(Self::NoSuchUser),
            _ => Ok(Self::NotDefined),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TFTPServer {
    root_directory: String,
}

impl TFTPServer {
    pub fn new(path: String) -> Self {
        TFTPServer {
            root_directory: path,
        }
    }

    pub fn handle_read_request(
        &self,
        dst: SocketAddr,
        rrq: ReadRequestPacket,
    ) -> Result<(), ParsingError> {
        run_guarded(|| info!("Handling Read Request!"));
        let tmp_socket = UdpSocket::bind("localhost:0").map_err(|_| ParsingError::SocketError)?;

        info!(
            "Mode : {:?}",
            rrq.mode.to_str().map_err(|_| ParsingError::InvalidMode)
        );

        match rrq.mode.to_str().map_err(|_| ParsingError::InvalidMode)? {
            "binary" | "octet" | "octe" => {}
            _ => self.send_error(&tmp_socket, &dst, ErrorCode::NotDefined)?,
        }

        let path = Path::new(&self.root_directory).join(
            rrq.filename
                .to_str()
                .map_err(|_| ParsingError::InvalidFilename)?,
        );

        info!("Attempting to read file @ {:?}", path.as_os_str());

        let mut f = File::open(path).map_err(|_| {
            self.send_error(&tmp_socket, &dst, ErrorCode::FileNotFound);
            ParsingError::InvalidFilename
        })?;

        info!("File opened");

        let mut file_buffer = [0; 512];
        let mut ack_buffer = [0; 512];
        let mut packet_counter = 1;
        loop {
            let data_read = f
                .read(&mut file_buffer)
                .map_err(|_| ParsingError::FileReadError)?;

            info!("Reading 512 bytes into file buffer");
            let data_packet = DataPacket {
                opcode: OpCode::Data,
                block_number: packet_counter,
                data: file_buffer,
                data_length: data_read,
            };

            let (size, buf) = data_packet.serialize();

            tmp_socket
                .send_to(&buf[0..size], dst)
                .map_err(|_| ParsingError::SocketError)?;

            info!("Sending Data");

            tmp_socket
                .recv(&mut ack_buffer)
                .map_err(|_| ParsingError::SocketError)?;

            match PacketType::try_from(&ack_buffer[..]) {
                Ok(val) => match val {
                    PacketType::Acknowledgment(ackp) => {
                        if ackp.block_number != packet_counter {
                            info!("Ack packet block number does not match packet counter");
                            tmp_socket
                                .send_to(&buf[0..size], dst)
                                .map_err(|_| ParsingError::SocketError)?;
                        } else {
                            info!("Ack packet block number matches packet counter");
                        }
                    }
                    _ => {
                        info!("Ack packet not seen!");
                        self.send_error(&tmp_socket, &dst, ErrorCode::IllegalTFTPOperation)?
                    }
                },
                Err(_) => {
                    info!("Error while interpreting packet from client");
                    self.send_error(&tmp_socket, &dst, ErrorCode::IllegalTFTPOperation)?;
                    break;
                }
            }

            packet_counter += 1;
            if data_read != 512 {
                info!("Data read not 512 bytes, that is the end of the file");
                break;
            }
        }

        Ok(())
    }

    pub fn handle_write_request(
        &self,
        dst: SocketAddr,
        wrq: WriteRequestPacket,
    ) -> Result<(), ParsingError> {
        run_guarded(|| info!("Handling Write Request!"));
        let tmp_socket = UdpSocket::bind("localhost:0").map_err(|_| ParsingError::SocketError)?;

        // tmp_socket.send_to(&buf[0..length], dst).map_err(|_| ParsingError::SocketError)?;

        Ok(())
    }

    fn send_error(
        &self,
        s: &UdpSocket,
        dst: &SocketAddr,
        error_code: ErrorCode,
    ) -> Result<(), ParsingError> {
        let error_pkt = ErrorPacket {
            opcode: OpCode::TFTPError,
            error_code,
            error_msg: CString::new("").map_err(|_| ParsingError::InvalidErrorMessage)?,
        };

        let (size, buf) = error_pkt.serialize();

        s.send_to(&buf[0..size], dst);

        Ok(())
    }
}

pub fn send_error(dst: SocketAddr, error_str: &str) -> Result<(), ParsingError> {
    let tmp_socket = UdpSocket::bind("localhost:0").map_err(|_| ParsingError::SocketError)?;

    let pkt = ErrorPacket {
        opcode: OpCode::TFTPError,
        error_code: ErrorCode::NotDefined,
        error_msg: CString::new(error_str).map_err(|_| ParsingError::InvalidErrorMessage)?,
    };

    let (length, buf) = pkt.serialize();

    tmp_socket
        .send_to(&buf[..length], dst)
        .map_err(|_| ParsingError::SocketError)?;

    Ok(())
}
