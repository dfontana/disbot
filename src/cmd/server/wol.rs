use crate::config::ServerConfig;
use std::{
  iter,
  net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
  process::{Command, Stdio},
  str::FromStr,
};

const MAC_SIZE: usize = 6;
const MAC_PER_MAGIC: usize = 16;
const SEPERATOR: char = ':';
static HEADER: [u8; 6] = [0xFF; 6];

#[derive(Debug, Clone)]
pub struct Wol {
  packet: Vec<u8>,
  ip: IpAddr,
  user: String,
}

impl Wol {
  pub fn new(cfg: &ServerConfig) -> Result<Self, String> {
    let hexed = cfg
      .mac
      .split(SEPERATOR)
      .flat_map(|x| hex::decode(x).expect("Invalid mac!"));
    let mut packet = Vec::with_capacity(HEADER.len() + MAC_SIZE * MAC_PER_MAGIC);
    packet.extend(HEADER.iter());
    packet.extend(iter::repeat(hexed).take(MAC_PER_MAGIC).flatten());
    let ip = Ipv4Addr::from_str(&cfg.ip).map_err(|e| format!("Invalid IP: {}", e))?;
    Ok(Wol {
      packet,
      ip: IpAddr::V4(ip),
      user: cfg.user.to_owned(),
    })
  }

  pub fn is_awake(&self) -> Result<bool, String> {
    let res = Command::new("ping")
      .stdout(Stdio::null())
      .arg(format!("{}", &self.ip))
      .args(&["-c", "1"])
      .args(&["-W", "1"])
      .status()
      .map_err(|e| format!("Failed to run Ping: {}", e))?
      .code();
    match res {
      Some(0) => Ok(true),
      Some(1) => Ok(false),
      Some(2) => Ok(false),
      _ => Err("Unknown error running ping".into()),
    }
  }

  pub fn awake(&self) -> std::io::Result<()> {
    let src = SocketAddr::from(([0, 0, 0, 0], 0));
    let dst = SocketAddr::from(([255, 255, 255, 255], 9));

    let udp_sock = UdpSocket::bind(src)?;
    udp_sock.set_broadcast(true)?;
    udp_sock.send_to(&self.packet, dst)?;

    Ok(())
  }

  pub fn shutdown(&self) -> Result<(), String> {
    let res = Command::new("ssh")
      .arg("-t")
      .arg(format!("{}@{}", &self.user, &self.ip))
      .args(&["-o", "PasswordAuthentication=no"])
      .arg("sudo shutdown now")
      .stdout(Stdio::null())
      .status()
      .map_err(|e| format!("Failed to run shutdown: {}", e))?
      .code();

    match res {
      Some(0) => Ok(()),
      Some(255) => Ok(()), // 255 means error but since the server closes this is fine.
      _ => Err("Failed to stop server".into()),
    }
  }
}
