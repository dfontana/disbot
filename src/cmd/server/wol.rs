use crate::config::ServerConfig;
use std::{
  iter,
  net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
  process::{Command, Stdio},
  str::FromStr,
  sync::{Arc, RwLock},
  time::{Duration, Instant},
};
use tracing::error;

const MAC_SIZE: usize = 6;
const MAC_PER_MAGIC: usize = 16;
const SEPERATOR: char = ':';
static HEADER: [u8; 6] = [0xFF; 6];

lazy_static! {
  static ref INSTANCE: RwLock<Option<Arc<Wol>>> = RwLock::new(None);
  static ref WAIT_SHUTDOWN: Duration = Duration::from_secs(600);
}

pub fn configure(cfg: &ServerConfig) -> Result<(), String> {
  let mut inst = INSTANCE
    .try_write()
    .map_err(|_| "Failed to get lock on wol instance")?;

  let hexed = cfg
    .mac
    .split(SEPERATOR)
    .flat_map(|x| hex::decode(x).expect("Invalid mac!"));
  let mut packet = Vec::with_capacity(HEADER.len() + MAC_SIZE * MAC_PER_MAGIC);
  packet.extend(HEADER.iter());
  packet.extend(iter::repeat(hexed).take(MAC_PER_MAGIC).flatten());
  let ip = Ipv4Addr::from_str(&cfg.ip).map_err(|e| format!("Invalid IP: {}", e))?;
  *inst = Some(Arc::new(Wol {
    packet,
    ip: IpAddr::V4(ip),
    user: cfg.user.to_owned(),
    last_shutdown: RwLock::new(None),
  }));

  Ok(())
}

#[derive(Debug)]
pub struct Wol {
  packet: Vec<u8>,
  ip: IpAddr,
  user: String,
  last_shutdown: RwLock<Option<Instant>>,
}

impl Wol {
  pub fn inst() -> Result<Arc<Self>, String> {
    match INSTANCE.try_read() {
      Err(_) => Err("Failed to get wol read lock".into()),
      Ok(lock) => match &*lock {
        Some(arc) => Ok(arc.clone()),
        None => Err("Wol was not configured".into()),
      },
    }
  }

  pub fn is_awake(&self) -> Result<bool, String> {
    let res = Command::new("ping")
      .arg(format!("{}", &self.ip))
      .args(&["-c", "1"])
      .args(&["-W", "1"])
      .stdout(Stdio::null())
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

  pub fn ensure_awake(&self) -> Result<bool, String> {
    self
      .is_awake()
      .map_err(|err| {
        error!("Failed to check Game Server is awake - {:?}", err);
        "Failed to determine if Game Server is up".into()
      })
      .and_then(|is_awake| {
        if !is_awake {
          return Err("Server is not awake, please start server".into());
        }
        Ok(is_awake)
      })
  }

  pub fn awake(&self) -> std::io::Result<()> {
    let src = SocketAddr::from(([0, 0, 0, 0], 0));
    let dst = SocketAddr::from(([255, 255, 255, 255], 9));

    let udp_sock = UdpSocket::bind(src)?;
    udp_sock.set_broadcast(true)?;
    udp_sock.send_to(&self.packet, dst)?;

    Ok(())
  }

  pub fn shutdown(&self) -> Result<u64, String> {
    let mut inst = self
      .last_shutdown
      .try_write()
      .map_err(|_| "Failed to get lock on shutdown timer")?;

    let elapsed = match *inst {
      Some(inst) => inst.elapsed(),
      None => {
        *inst = Some(Instant::now());
        Duration::new(0, 0)
      }
    };

    let diff = (WAIT_SHUTDOWN.as_secs() as i64 - elapsed.as_secs() as i64).max(0);
    if diff > 0 {
      return Ok(diff as u64);
    }

    // This is a bit dangerous piece of code here; given we're not host checking
    // and relying on running a sudo command on the remote host. Ideally we wouldn't
    // need to do this, but given there's no known alternate at the moment and
    // it's a closed system, it's 'good enough'
    let res = Command::new("ssh")
      .arg("-t")
      .arg(format!("{}@{}", &self.user, &self.ip))
      .args(&["-o", "PasswordAuthentication=no"])
      .args(&["-o", "StrictHostKeyChecking=no"])
      .args(&["-o", "ConnectTimeout=1"])
      .args(&["-o", "BatchMode=yes"])
      .arg("sudo shutdown now")
      .stdout(Stdio::null())
      .status()
      .map_err(|e| format!("Failed to run shutdown: {}", e))?
      .code();

    let ret = match res {
      Some(0) => Ok(0),
      Some(255) => Ok(0), // 255 means error but since the server closes this is fine.
      _ => Err("Failed to stop server".into()),
    };

    *inst = Some(Instant::now());

    ret
  }
}
