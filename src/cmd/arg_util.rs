use std::collections::HashMap;

use anyhow::anyhow;
use serenity::all::{ResolvedOption, ResolvedValue, Role};

#[derive(Debug)]
pub struct Args<'a>(HashMap<&'a str, &'a ResolvedValue<'a>>);

impl<'a> Args<'a> {
  pub fn from(args: &'a [ResolvedOption]) -> Args<'a> {
    Self(args.iter().map(|v| (v.name, &v.value)).collect())
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }

  pub fn i64(&self, key: &str) -> Result<&i64, anyhow::Error> {
    self
      .0
      .get(key)
      .ok_or_else(|| anyhow!("Could not get arg: {}", key))
      .and_then(|d| match d {
        ResolvedValue::Integer(v) => Ok(v),
        _ => Err(anyhow!("{} is not an Integer", key)),
      })
  }

  pub fn str(&self, key: &str) -> Result<&str, anyhow::Error> {
    self
      .0
      .get(key)
      .ok_or_else(|| anyhow!("Could not get arg: {}", key))
      .and_then(|d| match d {
        ResolvedValue::String(v) => Ok(*v),
        _ => Err(anyhow!("{} is not an String", key)),
      })
  }

  pub fn opt_role(&self, key: &str) -> Result<Option<&Role>, anyhow::Error> {
    if let Some(d) = self.0.get(key) {
      return match d {
        ResolvedValue::Role(v) => Ok(Some(*v)),
        _ => Err(anyhow!("{} is not an Role", key)),
      };
    }
    Ok(None)
  }
}
