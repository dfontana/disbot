use bincode::{impl_borrow_decode, Decode, Encode};
use chrono::{NaiveTime, Timelike};
use derive_more::{Deref, Display};
use serenity::{
  all::{EmojiId, GuildId, RoleId},
  model::prelude::ChannelId,
};
use uuid::Uuid;

macro_rules! impl_encode {
  ($name:ident, |$this:ident| $inner:expr) => {
    impl Encode for $name {
      fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
      ) -> Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(&(|$this: &$name| $inner)(self), encoder)
      }
    }
  };
}

macro_rules! impl_decode {
  ($name:ident, |$this:ident| $inner:expr) => {
    impl<Context> Decode<Context> for $name {
      fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
      ) -> Result<Self, bincode::error::DecodeError> {
        Ok($name((|$this| $inner)(bincode::Decode::decode(decoder)?)))
      }
    }
  };
}

// Formats: https://discord.com/developers/docs/reference#message-formatting

#[derive(Clone, Deref, Display)]
#[display("<#{_0}>")]
pub struct Chan(pub ChannelId);
impl_encode!(Chan, |s| s.0.get());
impl_decode!(Chan, |d| ChannelId::new(d));
impl_borrow_decode!(Chan);

#[derive(Clone, Deref)]
pub struct Guil(pub GuildId);
impl_encode!(Guil, |s| s.0.get());
impl_decode!(Guil, |d| GuildId::new(d));
impl_borrow_decode!(Guil);

#[derive(Clone, Deref, Display)]
#[display("<&@{_0}>")]
pub struct Rol(pub RoleId);
impl_encode!(Rol, |s| s.0.get());
impl_decode!(Rol, |d| RoleId::new(d));
impl_borrow_decode!(Rol);

#[derive(Clone, Deref)]
pub struct Emoj(pub EmojiId);
impl_encode!(Emoj, |s| s.0.get());
impl_decode!(Emoj, |d| EmojiId::new(d));
impl_borrow_decode!(Emoj);

#[derive(Clone, Deref)]
pub struct NaiveT(pub NaiveTime);

#[derive(Clone, Debug, Deref, PartialEq, Eq)]
pub struct Pid(pub Uuid);
impl_encode!(Pid, |s| s.0.into_bytes());
impl_decode!(Pid, |d| Uuid::from_bytes(d));
impl_borrow_decode!(Pid);

impl Encode for NaiveT {
  fn encode<E: bincode::enc::Encoder>(
    &self,
    encoder: &mut E,
  ) -> Result<(), bincode::error::EncodeError> {
    bincode::Encode::encode(&self.0.num_seconds_from_midnight(), encoder)?;
    bincode::Encode::encode(&self.0.nanosecond(), encoder)?;
    Ok(())
  }
}
impl<Context> Decode<Context> for NaiveT {
  fn decode<D: bincode::de::Decoder<Context = Context>>(
    decoder: &mut D,
  ) -> Result<Self, bincode::error::DecodeError> {
    let secs: u32 = bincode::Decode::decode(decoder)?;
    let nanos: u32 = bincode::Decode::decode(decoder)?;
    Ok(NaiveT(
      NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos).ok_or(
        bincode::error::DecodeError::InvalidDuration {
          secs: secs.into(),
          nanos,
        },
      )?,
    ))
  }
}
impl_borrow_decode!(NaiveT);
