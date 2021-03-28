use std::iter;
use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;

const MAC_SIZE: usize = 6;
const MAC_PER_MAGIC: usize = 16;
const SEPERATOR: char = ':';
static HEADER: [u8; 6] = [0xFF; 6];

#[derive(Debug, Clone)]
pub struct Wol {
  packet: Vec<u8>,
}

impl FromStr for Wol {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let hexed = s
      .split(SEPERATOR)
      .flat_map(|x| hex::decode(x).expect("Invalid mac!"));
    let mut packet = Vec::with_capacity(HEADER.len() + MAC_SIZE * MAC_PER_MAGIC);
    packet.extend(HEADER.iter());
    packet.extend(iter::repeat(hexed).take(MAC_PER_MAGIC).flatten());
    Ok(Wol { packet })
  }
}

impl Wol {
  pub fn is_awake(&self) -> Result<bool, String> {
    // TODO, first see it works
    Ok(false)
  }

  pub fn awake(&self) -> std::io::Result<()> {
    let src = SocketAddr::from(([0, 0, 0, 0], 0));
    let dst = SocketAddr::from(([255, 255, 255, 255], 9));

    let udp_sock = UdpSocket::bind(src)?;
    udp_sock.set_broadcast(true)?;
    udp_sock.send_to(&self.packet, dst)?;

    Ok(())
  }
}
